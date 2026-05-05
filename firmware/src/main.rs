//! Flow controller for RAK4631 WisBlock (nRF52840 + SX1262).
//! BLE peripheral accepts a flow-rate value (1-10) from a central.
//! Sends that value as a LoRaWAN uplink every 5 seconds.
#![no_std]
#![no_main]

mod ble;
mod board;
mod flash;
mod lorawan;
mod proto;
mod rng;
mod shared;

use defmt::{info, unwrap};
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Input, Level, Output, OutputDrive, Pull};
use embassy_nrf::nvmc::Nvmc;
use embassy_nrf::{rng as nrf_rng, spim};
use nrf_sdc::{self as sdc, mpsl};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use crate::shared::LORAWAN_KEYS;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    info!("Flow controller with BLE starting");

    // ── MPSL init ───────────────────────────────────────────────────────
    let mpsl_p = mpsl::Peripherals::new(
        p.RTC0, p.TIMER0, p.TEMP, p.PPI_CH19, p.PPI_CH30, p.PPI_CH31,
    );
    let lfclk_cfg = mpsl::raw::mpsl_clock_lfclk_cfg_t {
        source: mpsl::raw::MPSL_CLOCK_LF_SRC_RC as u8,
        rc_ctiv: mpsl::raw::MPSL_RECOMMENDED_RC_CTIV as u8,
        rc_temp_ctiv: mpsl::raw::MPSL_RECOMMENDED_RC_TEMP_CTIV as u8,
        accuracy_ppm: mpsl::raw::MPSL_DEFAULT_CLOCK_ACCURACY_PPM as u16,
        skip_wait_lfclk_started: mpsl::raw::MPSL_DEFAULT_SKIP_WAIT_LFCLK_STARTED != 0,
    };
    static MPSL: StaticCell<mpsl::MultiprotocolServiceLayer> = StaticCell::new();
    let mpsl = MPSL.init(unwrap!(mpsl::MultiprotocolServiceLayer::new(
        mpsl_p,
        board::Irqs,
        lfclk_cfg
    )));
    spawner.spawn(unwrap!(board::mpsl_task(&*mpsl)));

    // ── SDC init ────────────────────────────────────────────────────────
    let sdc_p = sdc::Peripherals::new(
        p.PPI_CH17, p.PPI_CH18, p.PPI_CH20, p.PPI_CH21, p.PPI_CH22, p.PPI_CH23, p.PPI_CH24,
        p.PPI_CH25, p.PPI_CH26, p.PPI_CH27, p.PPI_CH28, p.PPI_CH29,
    );

    static RNG: StaticCell<nrf_rng::Rng<'static, embassy_nrf::mode::Async>> = StaticCell::new();
    let rng = RNG.init(nrf_rng::Rng::new(p.RNG, board::Irqs));

    // Grab a hardware-random seed before SDC takes ownership of the RNG
    let mut seed_bytes = [0u8; 8];
    rng.blocking_fill_bytes(&mut seed_bytes);
    let lorawan_seed = u64::from_le_bytes(seed_bytes);

    static SDC_MEM: StaticCell<sdc::Mem<4720>> = StaticCell::new();
    let sdc_mem = SDC_MEM.init(sdc::Mem::new());
    let sdc = unwrap!(board::build_sdc(sdc_p, rng, mpsl, sdc_mem));

    // ── SX1262 SPI setup ────────────────────────────────────────────────
    let nss = Output::new(p.P1_10, Level::High, OutputDrive::Standard);
    let reset = Output::new(p.P1_06, Level::High, OutputDrive::Standard);
    let dio1 = Input::new(p.P1_15, Pull::Down);
    let busy = Input::new(p.P1_14, Pull::None);
    let rf_switch_rx = Output::new(p.P1_05, Level::Low, OutputDrive::Standard);
    let rf_switch_tx = Output::new(p.P1_07, Level::Low, OutputDrive::Standard);

    let mut spi_config = spim::Config::default();
    spi_config.frequency = spim::Frequency::M16;
    let spim = spim::Spim::new(p.TWISPI1, board::Irqs, p.P1_11, p.P1_13, p.P1_12, spi_config);

    let lorawan_rng = rng::SoftwareRng(lorawan_seed);

    // ── Flash: load stored keys ────────────────────────────────────────
    let nvmc = Nvmc::new(p.NVMC);
    if let Some(keys) = flash::read_keys() {
        info!("Found LoRaWAN keys in flash, DevEUI: {:02x}", keys.deveui);
        LORAWAN_KEYS.signal(keys);
    } else {
        info!("No LoRaWAN keys in flash, waiting for BLE provisioning");
    }

    // ── Spawn tasks ─────────────────────────────────────────────────────
    spawner.spawn(unwrap!(lorawan::lorawan_task(
        spim,
        nss,
        reset,
        dio1,
        busy,
        rf_switch_rx,
        rf_switch_tx,
        lorawan_rng,
    )));

    // BLE runs in main context (not spawned — owns non-'static sdc)
    ble::run_ble(sdc, nvmc).await;
}
