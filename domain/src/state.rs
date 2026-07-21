//! Pure valve state machine: how a received downlink maps to an action, and
//! how the two tracked states evolve. No I/O.

use crate::codec::decode_downlink;
use crate::types::ValveState;

/// The two states the device tracks.
///
/// `current` is what we believe the valve physically is; `last_commanded` is
/// the most recent backend command. They match under the stub actuator; a real
/// driver (Phase 4) can make them diverge on a failed actuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientState {
    pub current: ValveState,
    pub last_commanded: ValveState,
}

impl ClientState {
    /// Hydrate from a single known state (e.g. the value read from flash on
    /// boot); `last_commanded` starts equal to it.
    pub fn new(current: ValveState) -> Self {
        Self {
            current,
            last_commanded: current,
        }
    }
}

/// What the caller should do about a received downlink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownlinkOutcome {
    /// Payload did not parse as a `Downlink`.
    Undecodable,
    /// Desired state already matches `current` — nothing to do (idempotent).
    Unchanged,
    /// Command was `Unknown`, which the backend must never send — ignored.
    Ignored,
    /// Caller must actuate to this state, then call [`on_actuated`] + persist.
    Transition(ValveState),
}

/// Decode a downlink and decide what to do. Updates `last_commanded` on any
/// decodable command (before actuation); never touches `current`.
pub fn on_downlink(payload: &[u8], state: &mut ClientState) -> DownlinkOutcome {
    let desired = match decode_downlink(payload) {
        Ok(v) => v,
        Err(_) => return DownlinkOutcome::Undecodable,
    };
    // Record the command on receipt, before actuation.
    state.last_commanded = desired;
    match desired {
        ValveState::Unknown => DownlinkOutcome::Ignored,
        _ if desired == state.current => DownlinkOutcome::Unchanged,
        target => DownlinkOutcome::Transition(target),
    }
}

/// Confirm a successful actuation: `current` now matches the commanded state.
pub fn on_actuated(state: &mut ClientState, target: ValveState) {
    state.current = target;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_from_unknown_transitions_and_records_command() {
        let mut s = ClientState::new(ValveState::Unknown);
        assert_eq!(on_downlink(&[0x08, 0x01], &mut s), DownlinkOutcome::Transition(ValveState::Open));
        assert_eq!(s.last_commanded, ValveState::Open); // recorded on receipt
        assert_eq!(s.current, ValveState::Unknown); // not until on_actuated
        on_actuated(&mut s, ValveState::Open);
        assert_eq!(s.current, ValveState::Open);
    }

    #[test]
    fn repeat_command_is_idempotent() {
        let mut s = ClientState::new(ValveState::Open);
        assert_eq!(on_downlink(&[0x08, 0x01], &mut s), DownlinkOutcome::Unchanged);
    }

    #[test]
    fn unknown_command_is_ignored() {
        let mut s = ClientState::new(ValveState::Open);
        assert_eq!(on_downlink(&[0x08, 0x03], &mut s), DownlinkOutcome::Ignored);
    }

    #[test]
    fn garbage_is_undecodable_and_leaves_command_untouched() {
        let mut s = ClientState::new(ValveState::Open);
        s.last_commanded = ValveState::Open;
        assert_eq!(on_downlink(&[0x08], &mut s), DownlinkOutcome::Undecodable);
        assert_eq!(s.last_commanded, ValveState::Open); // unchanged
    }
}
