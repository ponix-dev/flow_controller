use defmt::{error, info};
use embassy_futures::select::{select, Either};
use embassy_nrf::gpio::{Input, Output};
use embassy_nrf::spim;
use embassy_time::{Delay, Duration, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use lora_phy::iv::GenericSx126xInterfaceVariant;
use lora_phy::lorawan_radio::LorawanRadio;
use lora_phy::sx126x::{self, Sx1262, Sx126x, TcxoCtrlVoltage};
use lora_phy::LoRa;
use lorawan_device::async_device::{region, Device, EmbassyTimer, JoinMode, JoinResponse};
use lorawan_device::region::Subband;
use lorawan_device::{AppEui, AppKey, DevEui};
use rand_core::RngCore;

use crate::rng::SoftwareRng;
use crate::shared::{LORAWAN_KEYS, MAX_TX_POWER, UPLINK_INTERVAL_SECS};

// ── Helpers ─────────────────────────────────────────────────────────────────

fn generate_delay(rng: &mut impl RngCore, retries: u16) -> u16 {
    let base = core::cmp::min(10 + (10 * retries), 3600);
    let jitter = base / 5;
    let range = 2 * jitter - jitter + 1; // jitter..=2*jitter
    let random_jitter = (rng.next_u32() % range as u32) as u16 + jitter;
    (base - jitter).saturating_add(random_jitter)
}

// ── Task ────────────────────────────────────────────────────────────────────

#[embassy_executor::task]
pub(crate) async fn lorawan_task(
    spim: spim::Spim<'static>,
    nss: Output<'static>,
    reset: Output<'static>,
    dio1: Input<'static>,
    busy: Input<'static>,
    rf_switch_rx: Output<'static>,
    rf_switch_tx: Output<'static>,
    lorawan_rng: SoftwareRng,
) {
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

    loop {
        info!("[lorawan] keys received, DevEUI: {:02x}", keys.deveui);
        info!("[lorawan] starting join and uplink loop");
        info!("[lorawan] uplink interval: {} seconds", UPLINK_INTERVAL_SECS);

        // OTAA join
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
                Ok(other) => {
                    error!("Join failed: {:?}", defmt::Debug2Format(&other));
                }
                Err(e) => {
                    error!("Join error: {:?}", defmt::Debug2Format(&e));
                }
            }
            let delay = generate_delay(&mut device.rng, retries);
            info!("[lorawan] retrying join in {} seconds", delay);
            Timer::after(Duration::from_secs(delay.into())).await;
            retries += 1;
        }

        // Main uplink loop — select waits for either the timer or new keys
        let mut uplink_count: u32 = 0;
        let message = b"ponix";
        loop {
            info!("[lorawan] sending uplink #{}", uplink_count);

            match device.send(message, 1, false).await {
                Ok(response) => {
                    info!(
                        "[lorawan] uplink sent, response: {:?}",
                        defmt::Debug2Format(&response)
                    );
                    while let Some(downlink) = device.take_downlink() {
                        info!(
                            "[lorawan] downlink received on port {}: {:02x}",
                            downlink.fport,
                            downlink.data.as_slice()
                        );
                    }
                }
                Err(e) => {
                    error!("[lorawan] uplink failed: {:?}", defmt::Debug2Format(&e));
                }
            }

            uplink_count += 1;

            match select(
                Timer::after(Duration::from_secs(UPLINK_INTERVAL_SECS)),
                LORAWAN_KEYS.wait(),
            )
            .await
            {
                Either::First(()) => {}
                Either::Second(new_keys) => {
                    info!("[lorawan] new keys received, exiting current uplink loop");
                    info!("[lorawan] new DevEUI: {:02x}", new_keys.deveui);
                    keys = new_keys;
                    break;
                }
            }
        }
    }
}
