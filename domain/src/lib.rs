//! Portable client-side business logic for the flow controller.
//!
//! This crate is `no_std` and depends only on `micropb` — no embassy, no nrf,
//! no radio. That keeps it host-compilable so the whole command path (wire
//! codec, flash record format, valve state machine, and the uplink/downlink
//! loop body) can be unit-tested with `cargo test` on a laptop, no device.
//!
//! The `firmware` crate provides the board/wire-specific setup: it implements
//! the [`Network`], [`Valve`], and [`Store`] traits with real hardware and
//! drives [`run_iteration`] from its embassy task.
#![no_std]

// Host tests use std collections (and the `vec!` macro) for the in-memory fakes.
#[cfg(test)]
#[macro_use]
extern crate std;

pub mod codec;
pub mod proto;
pub mod record;
pub mod runtime;
pub mod state;
mod types;

pub use codec::{decode_downlink, encode_uplink, DecodeError};
pub use record::{pack_record, unpack_record};
pub use runtime::{init_state, run_iteration, NetError, Network, Store, Valve};
pub use state::{on_actuated, on_downlink, ClientState, DownlinkOutcome};
pub use types::{LorawanKeys, ValveState};
