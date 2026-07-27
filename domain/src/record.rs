//! Flash record format: the pure pack/unpack of the persisted 40-byte record.
//!
//! The hardware I/O (page erase, memory-mapped read) lives in the firmware's
//! `flash` module; this module only owns the byte layout so it can be tested
//! on the host.

use crate::types::{LorawanKeys, ValveState};

/// Record magic. `FCV1` = Flow Controller v1. Bumped from the legacy keys-only
/// `LORA` record; old records fail this check and read as "no record".
pub const MAGIC: [u8; 4] = *b"FCV1";
/// MAGIC(4) + DEVEUI(8) + APPEUI(8) + APPKEY(16) + VALVE_STATE(1) + RESERVED(3) = 40.
pub const RECORD_LEN: usize = 40;
/// Byte offset of the persisted [`ValveState`] within the record.
const VALVE_STATE_OFFSET: usize = 36;

/// Serialize keys + valve state into the canonical 40-byte record.
///
/// Because flash erase is page-level, keys and valve state always share one
/// physical record; callers must supply both halves so a rewrite can never
/// silently drop the other.
pub fn pack_record(keys: &LorawanKeys, state: ValveState) -> [u8; RECORD_LEN] {
    let mut buf = [0u8; RECORD_LEN];
    buf[0..4].copy_from_slice(&MAGIC);
    buf[4..12].copy_from_slice(&keys.deveui);
    buf[12..20].copy_from_slice(&keys.appeui);
    buf[20..36].copy_from_slice(&keys.appkey);
    buf[VALVE_STATE_OFFSET] = state.to_byte();
    // [37..40] reserved, left zero for a future schema_version or small fields.
    buf
}

/// Parse a raw record, validating the magic. Returns `None` when no valid
/// record is present (wrong magic / erased page).
pub fn unpack_record(buf: &[u8; RECORD_LEN]) -> Option<(LorawanKeys, ValveState)> {
    if buf[0..4] != MAGIC {
        return None;
    }

    let mut deveui = [0u8; 8];
    let mut appeui = [0u8; 8];
    let mut appkey = [0u8; 16];
    deveui.copy_from_slice(&buf[4..12]);
    appeui.copy_from_slice(&buf[12..20]);
    appkey.copy_from_slice(&buf[20..36]);

    let keys = LorawanKeys {
        deveui,
        appeui,
        appkey,
    };
    let state = ValveState::from_byte(buf[VALVE_STATE_OFFSET]);
    Some((keys, state))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_keys() -> LorawanKeys {
        LorawanKeys {
            deveui: [1, 2, 3, 4, 5, 6, 7, 8],
            appeui: [9, 10, 11, 12, 13, 14, 15, 16],
            appkey: [0xAA; 16],
        }
    }

    #[test]
    fn pack_unpack_round_trip() {
        let keys = sample_keys();
        for state in [ValveState::Open, ValveState::Closed, ValveState::Unknown] {
            let buf = pack_record(&keys, state);
            let (k, s) = unpack_record(&buf).expect("valid record");
            assert_eq!(k, keys);
            assert_eq!(s, state);
        }
    }

    #[test]
    fn bad_magic_reads_as_none() {
        let mut buf = pack_record(&sample_keys(), ValveState::Open);
        buf[0] = b'X';
        assert!(unpack_record(&buf).is_none());
        // Erased flash (all 0xFF) is also "no record".
        assert!(unpack_record(&[0xFF; RECORD_LEN]).is_none());
    }
}
