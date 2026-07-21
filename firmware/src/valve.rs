//! Valve actuator — the firmware-side implementation of [`domain::Valve`].
//!
//! Phase 3 is a **stub**: `open`/`close` only log via defmt and never touch a
//! GPIO. Phase 4 replaces the bodies with STSPIN250 H-bridge pulses; the
//! `domain::Valve` boundary above is unchanged.

use defmt::info;
use domain::Valve;

pub(crate) struct StubValve;

impl Valve for StubValve {
    async fn open(&mut self) {
        info!("[valve] STUB open");
    }

    async fn close(&mut self) {
        info!("[valve] STUB close");
    }
}
