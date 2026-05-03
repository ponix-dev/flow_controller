// GPIO pin assignments are documented in `hardware/PINOUT.md` — that file is
// the single source of truth shared by this crate and the schematic.

use embassy_nrf::mode::Async;
use embassy_nrf::{bind_interrupts, peripherals, rng, spim};
use nrf_sdc::mpsl::MultiprotocolServiceLayer;
use nrf_sdc::{self as sdc};
use trouble_host::prelude::DefaultPacketPool;
use trouble_host::PacketPool;

use crate::shared::{L2CAP_RXQ, L2CAP_TXQ};

// ── Interrupt bindings ──────────────────────────────────────────────────────

bind_interrupts!(pub struct Irqs {
    // SPI for SX1262
    TWISPI1 => spim::InterruptHandler<peripherals::TWISPI1>;
    // RNG
    RNG => rng::InterruptHandler<peripherals::RNG>;
    // MPSL
    EGU0_SWI0 => nrf_sdc::mpsl::LowPrioInterruptHandler;
    CLOCK_POWER => nrf_sdc::mpsl::ClockInterruptHandler;
    RADIO => nrf_sdc::mpsl::HighPrioInterruptHandler;
    TIMER0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
    RTC0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
});

// ── SDC builder ─────────────────────────────────────────────────────────────

pub(crate) fn build_sdc<'d, const N: usize>(
    p: nrf_sdc::Peripherals<'d>,
    rng: &'d mut rng::Rng<Async>,
    mpsl: &'d MultiprotocolServiceLayer,
    mem: &'d mut sdc::Mem<N>,
) -> Result<nrf_sdc::SoftdeviceController<'d>, nrf_sdc::Error> {
    sdc::Builder::new()?
        .support_adv()
        .support_peripheral()
        .peripheral_count(1)?
        .buffer_cfg(
            DefaultPacketPool::MTU as u16,
            DefaultPacketPool::MTU as u16,
            L2CAP_TXQ,
            L2CAP_RXQ,
        )?
        .build(p, rng, mpsl, mem)
}

// ── Tasks ───────────────────────────────────────────────────────────────────

#[embassy_executor::task]
pub(crate) async fn mpsl_task(mpsl: &'static MultiprotocolServiceLayer<'static>) -> ! {
    mpsl.run().await
}
