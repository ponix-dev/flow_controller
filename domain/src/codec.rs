//! Pure micropb codec for the `Uplink` (encode) and `Downlink` (decode) wire
//! messages. No radio — just bytes in, bytes out.

use micropb::{MessageDecode, MessageEncode, PbDecoder, PbEncoder};

use crate::proto;
use crate::types::ValveState;

/// A downlink payload could not be parsed as a `Downlink` message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeError;

/// Encode an `Uplink` into a stack buffer, returning the buffer and the number
/// of bytes written. `Uplink::MAX_SIZE` is ~22 bytes (two int32-encoded enums +
/// tags), so 32 is ample.
pub fn encode_uplink(current: ValveState, last_commanded: ValveState) -> ([u8; 32], usize) {
    let uplink = proto::Uplink {
        current_state: current.to_proto(),
        last_commanded_state: last_commanded.to_proto(),
    };
    let mut buf = [0u8; 32];
    let mut encoder = PbEncoder::new(&mut buf[..]);
    uplink
        .encode(&mut encoder)
        .expect("Uplink fits in 32-byte buffer");
    // `PbWrite for &mut [u8]` advances the slice as it writes, so the remaining
    // tail's length tells us how many bytes we produced.
    let remaining = encoder.into_writer().len();
    let written = buf.len() - remaining;
    (buf, written)
}

/// Decode a `Downlink` payload into the desired valve state.
pub fn decode_downlink(payload: &[u8]) -> Result<ValveState, DecodeError> {
    let mut decoder = PbDecoder::new(payload);
    let mut msg = proto::Downlink::default();
    msg.decode(&mut decoder, payload.len())
        .map_err(|_| DecodeError)?;
    Ok(ValveState::from_proto(msg.desired_state))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uplink_wire_format() {
        let (buf, n) = encode_uplink(ValveState::Open, ValveState::Open);
        assert_eq!(&buf[..n], &[0x08, 0x01, 0x10, 0x01]);

        let (buf, n) = encode_uplink(ValveState::Unknown, ValveState::Unknown);
        assert_eq!(&buf[..n], &[0x08, 0x03, 0x10, 0x03]);

        let (buf, n) = encode_uplink(ValveState::Closed, ValveState::Open);
        assert_eq!(&buf[..n], &[0x08, 0x02, 0x10, 0x01]);
    }

    #[test]
    fn downlink_decodes() {
        assert_eq!(decode_downlink(&[0x08, 0x01]), Ok(ValveState::Open));
        assert_eq!(decode_downlink(&[0x08, 0x02]), Ok(ValveState::Closed));
        // Empty payload = default Downlink = Unspecified -> Unknown.
        assert_eq!(decode_downlink(&[]), Ok(ValveState::Unknown));
    }

    #[test]
    fn truncated_payload_is_error() {
        // Tag says field 1 is a varint, but the value byte is missing.
        assert_eq!(decode_downlink(&[0x08]), Err(DecodeError));
    }

    #[test]
    fn encode_decode_round_trip() {
        // A Downlink{OPEN} and the Uplink's current_state field share tag 0x08.
        let (buf, n) = encode_uplink(ValveState::Closed, ValveState::Unknown);
        // First field of the uplink is current_state; decode it as a Downlink.
        assert_eq!(decode_downlink(&buf[..n.min(2)]), Ok(ValveState::Closed));
    }
}
