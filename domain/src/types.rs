//! Portable domain types shared across the codec, record format, and state
//! machine.

use crate::proto;

/// Domain valve state.
///
/// Deliberately separate from the generated `proto::ValveState` newtype
/// (`struct ValveState(pub i32)`) so control logic can `match` ergonomically;
/// conversions live at the protocol/persistence boundary below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValveState {
    Open,
    Closed,
    Unknown,
}

impl ValveState {
    /// Map to the wire enum. `Unknown` has no dedicated wire value — it maps to
    /// `Unspecified` (0), the proto default for "no known state".
    pub fn to_proto(self) -> proto::ValveState {
        match self {
            ValveState::Open => proto::ValveState::Open,
            ValveState::Closed => proto::ValveState::Closed,
            ValveState::Unknown => proto::ValveState::Unspecified,
        }
    }

    /// Map from the wire enum. `Unspecified` (the proto default, `0`) and any
    /// unrecognized value collapse to `Unknown`.
    pub fn from_proto(p: proto::ValveState) -> Self {
        match p.0 {
            1 => ValveState::Open,
            2 => ValveState::Closed,
            _ => ValveState::Unknown,
        }
    }

    /// Persisted single-byte representation. Shares integer values with the
    /// wire enum for consistency, but is an independent on-flash contract.
    pub fn to_byte(self) -> u8 {
        match self {
            ValveState::Open => 1,
            ValveState::Closed => 2,
            ValveState::Unknown => 3,
        }
    }

    /// Inverse of [`to_byte`](Self::to_byte). Unrecognized bytes — including `0`
    /// and the `0xFF` of erased flash — read as `Unknown`.
    pub fn from_byte(b: u8) -> Self {
        match b {
            1 => ValveState::Open,
            2 => ValveState::Closed,
            _ => ValveState::Unknown,
        }
    }
}

/// LoRaWAN OTAA credentials received via BLE provisioning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LorawanKeys {
    pub deveui: [u8; 8],
    pub appeui: [u8; 8],
    pub appkey: [u8; 16],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proto_round_trip() {
        for s in [ValveState::Open, ValveState::Closed, ValveState::Unknown] {
            assert_eq!(ValveState::from_proto(s.to_proto()), s);
        }
    }

    #[test]
    fn byte_round_trip() {
        for s in [ValveState::Open, ValveState::Closed, ValveState::Unknown] {
            assert_eq!(ValveState::from_byte(s.to_byte()), s);
        }
    }

    #[test]
    fn unspecified_and_erased_read_as_unknown() {
        // proto default (0) and erased flash (0xFF) both collapse to Unknown.
        assert_eq!(ValveState::from_proto(proto::ValveState::Unspecified), ValveState::Unknown);
        assert_eq!(ValveState::from_byte(0), ValveState::Unknown);
        assert_eq!(ValveState::from_byte(0xFF), ValveState::Unknown);
    }
}
