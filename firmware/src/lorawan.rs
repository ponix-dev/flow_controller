use defmt::{error, info};
use domain::{init_state, run_iteration, NetError, Network};
use embassy_futures::select::{select, Either};
use embassy_nrf::gpio::{Input, Output};
use embassy_nrf::spim;
use embassy_time::{Delay, Duration, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use lora_phy::iv::GenericSx126xInterfaceVariant;
use lora_phy::lorawan_radio::LorawanRadio;
use lora_phy::sx126x::{self, Sx1262, Sx126x, TcxoCtrlVoltage};
use lora_phy::LoRa;
use lorawan_device::async_device::{
    radio, region, Device, EmbassyTimer, JoinMode, JoinResponse, Timings,
};
use lorawan_device::region::Subband;
use lorawan_device::{AppEui, AppKey, DevEui};
use rand_core::RngCore;

use crate::flash::FlashStore;
use crate::rng::SoftwareRng;
use crate::shared::{SharedNvmc, LORAWAN_KEYS, MAX_TX_POWER, UPLINK_INTERVAL_SECS};
use crate::valve::StubValve;

// ── Helpers ─────────────────────────────────────────────────────────────────

fn generate_delay(rng: &mut impl RngCore, retries: u16) -> u16 {
    let base = core::cmp::min(10 + (10 * retries), 3600);
    let jitter = base / 5;
    let range = 2 * jitter - jitter + 1; // jitter..=2*jitter
    let random_jitter = (rng.next_u32() % range as u32) as u16 + jitter;
    (base - jitter).saturating_add(random_jitter)
}

// ── Network seam ──────────────────────────────────────────────────────────────
// The orphan rule forbids implementing `domain::Network` (foreign trait) for the
// foreign `Device`, so wrap it in a local newtype. Joining stays outside this
// impl (see the task below) because the retry/backoff needs the radio's RNG.

struct RadioNetwork<R, T, G, const N: usize, const D: usize>
where
    R: radio::PhyRxTx + Timings,
    T: radio::Timer,
    G: RngCore,
{
    device: Device<R, T, G, N, D>,
}

impl<R, T, G, const N: usize, const D: usize> Network for RadioNetwork<R, T, G, N, D>
where
    R: radio::PhyRxTx + Timings,
    T: radio::Timer,
    G: RngCore,
{
    async fn send_uplink(&mut self, bytes: &[u8], fport: u8) -> Result<(), NetError> {
        self.device
            .send(bytes, fport, false)
            .await
            .map(|_| ())
            .map_err(|_| NetError)
    }

    fn next_downlink(&mut self, buf: &mut [u8]) -> Option<usize> {
        let dl = self.device.take_downlink()?;
        let data = dl.data.as_slice();
        let n = core::cmp::min(data.len(), buf.len());
        buf[..n].copy_from_slice(&data[..n]);
        Some(n)
    }
}

// ── Task ────────────────────────────────────────────────────────────────────

/// SPI bus + SX1262 control pins for the LoRaWAN radio, bundled into one value
/// so the task takes a handful of arguments instead of a long peripheral list.
/// Built as a struct literal in `main.rs`.
pub(crate) struct RadioResources {
    pub spim: spim::Spim<'static>,
    pub nss: Output<'static>,
    pub reset: Output<'static>,
    pub dio1: Input<'static>,
    pub busy: Input<'static>,
    pub rf_switch_rx: Output<'static>,
    pub rf_switch_tx: Output<'static>,
}

pub(crate) async fn run(
    radio: RadioResources,
    lorawan_rng: SoftwareRng,
    nvmc: &'static SharedNvmc,
) {
    let RadioResources {
        spim,
        nss,
        reset,
        dio1,
        busy,
        rf_switch_rx,
        rf_switch_tx,
    } = radio;

    let spi = ExclusiveDevice::new(spim, nss, Delay);

    let config = sx126x::Config {
        chip: Sx1262,
        tcxo_ctrl: Some(TcxoCtrlVoltage::Ctrl1V7),
        use_dcdc: true,
        rx_boost: false,
    };
    let iv = GenericSx126xInterfaceVariant::new(
        reset,
        dio1,
        busy,
        Some(rf_switch_rx),
        Some(rf_switch_tx),
    )
    .unwrap();
    let lora = LoRa::new(Sx126x::new(spi, iv, config), true, Delay)
        .await
        .unwrap();

    let mut radio: LorawanRadio<_, _, MAX_TX_POWER> = lora.into();
    radio.set_rx_window_lead_time(100);
    radio.set_rx_window_buffer(200);

    let mut us915 = lorawan_device::region::US915::new();
    us915.set_join_bias(Subband::_2);
    let region = region::Configuration::from(us915);
    let mut device: Device<_, _, _, 256, 1> =
        Device::new(region, radio, EmbassyTimer::new(), lorawan_rng);

    info!("[lorawan] waiting for keys...");
    let mut keys = LORAWAN_KEYS.wait().await;

    // Actuator + persistence seams are stable across rejoins.
    let mut valve = StubValve;
    let mut store = FlashStore::new(nvmc);

    // Restore the persisted valve position, or drive closed on a fresh device.
    let mut client = init_state(&keys, &mut valve, &mut store).await;
    info!("[lorawan] boot valve state: {}", client.current.to_byte());

    loop {
        info!("[lorawan] keys received, DevEUI: {:02x}", keys.deveui);
        info!("[lorawan] uplink interval: {} seconds", UPLINK_INTERVAL_SECS);

        // OTAA join with jittered backoff — on the raw device (needs its RNG).
        let join_mode = JoinMode::OTAA {
            deveui: DevEui::from(keys.deveui),
            appeui: AppEui::from(keys.appeui),
            appkey: AppKey::from(keys.appkey),
        };
        let mut retries = 0;
        loop {
            match device.join(&join_mode).await {
                Ok(JoinResponse::JoinSuccess) => {
                    info!("[lorawan] network joined");
                    break;
                }
                Ok(other) => error!("Join failed: {:?}", defmt::Debug2Format(&other)),
                Err(e) => error!("Join error: {:?}", defmt::Debug2Format(&e)),
            }
            let delay = generate_delay(&mut device.rng, retries);
            info!("[lorawan] retrying join in {} seconds", delay);
            Timer::after(Duration::from_secs(delay.into())).await;
            retries += 1;
        }

        // Uplink/downlink loop via the domain seam; wrap the joined device.
        let mut net = RadioNetwork { device };
        let mut uplink_count: u32 = 0;
        let new_keys = loop {
            info!("[lorawan] sending uplink #{}", uplink_count);
            match run_iteration(&mut client, &keys, &mut net, &mut valve, &mut store).await {
                Ok(()) => info!(
                    "[lorawan] uplink #{} done (current={}, last={})",
                    uplink_count,
                    client.current.to_byte(),
                    client.last_commanded.to_byte()
                ),
                Err(_) => error!("[lorawan] uplink #{} failed", uplink_count),
            }
            uplink_count += 1;

            match select(
                Timer::after(Duration::from_secs(UPLINK_INTERVAL_SECS)),
                LORAWAN_KEYS.wait(),
            )
            .await
            {
                Either::First(()) => {}
                Either::Second(nk) => break nk,
            }
        };

        // Reclaim the device from the wrapper to re-join with the new keys.
        device = net.device;
        keys = new_keys;
        info!("[lorawan] new keys received, re-joining. DevEUI: {:02x}", keys.deveui);
    }
}
