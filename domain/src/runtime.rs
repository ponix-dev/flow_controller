//! The client-side loop body, generic over the three hardware-facing seams:
//! [`Network`] (radio), [`Valve`] (actuator), and [`Store`] (flash). The
//! firmware implements these against real peripherals; tests implement them
//! with in-memory fakes and drive [`run_iteration`] with no device.

use crate::codec::encode_uplink;
use crate::state::{on_actuated, on_downlink, ClientState, DownlinkOutcome};
use crate::types::{LorawanKeys, ValveState};

/// Uplink fport. Downlinks piggyback in the RX windows this uplink opens.
pub const FPORT: u8 = 1;

/// A network operation (join/send) failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetError;

/// The LoRaWAN transport seam for one Class-A cycle. (Joining + retry/backoff
/// stays in the firmware shell, which owns the radio's RNG for jitter.)
#[allow(async_fn_in_trait)]
pub trait Network {
    /// Send an uplink on `fport`.
    async fn send_uplink(&mut self, bytes: &[u8], fport: u8) -> Result<(), NetError>;
    /// Copy the next buffered downlink payload into `buf`, returning its length.
    /// `None` when no more downlinks are queued.
    fn next_downlink(&mut self, buf: &mut [u8]) -> Option<usize>;
}

/// The valve actuator seam.
#[allow(async_fn_in_trait)]
pub trait Valve {
    async fn open(&mut self);
    async fn close(&mut self);
}

/// The persistence seam.
#[allow(async_fn_in_trait)]
pub trait Store {
    /// Read the persisted valve state, or `None` if no record exists.
    fn load_valve_state(&self) -> Option<ValveState>;
    /// Persist keys + valve state as one record.
    async fn persist(&mut self, keys: &LorawanKeys, state: ValveState);
}

/// One Class-A cycle: send the current state, then drain and apply every
/// downlink received in the RX window. Returns the send result so the caller
/// can log or retry.
pub async fn run_iteration<N, V, S>(
    state: &mut ClientState,
    keys: &LorawanKeys,
    net: &mut N,
    valve: &mut V,
    store: &mut S,
) -> Result<(), NetError>
where
    N: Network,
    V: Valve,
    S: Store,
{
    let (buf, len) = encode_uplink(state.current, state.last_commanded);
    net.send_uplink(&buf[..len], FPORT).await?;

    let mut dl = [0u8; 32];
    while let Some(n) = net.next_downlink(&mut dl) {
        match on_downlink(&dl[..n], state) {
            DownlinkOutcome::Transition(target) => {
                match target {
                    ValveState::Open => valve.open().await,
                    ValveState::Closed => valve.close().await,
                    ValveState::Unknown => continue, // never produced by on_downlink
                }
                on_actuated(state, target);
                store.persist(keys, state.current).await;
            }
            DownlinkOutcome::Unchanged
            | DownlinkOutcome::Ignored
            | DownlinkOutcome::Undecodable => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{pack_record, unpack_record};
    use std::collections::VecDeque;
    use std::vec::Vec;

    #[derive(Default)]
    struct FakeNetwork {
        sent: Vec<Vec<u8>>,
        downlinks: VecDeque<Vec<u8>>,
    }
    impl Network for FakeNetwork {
        async fn send_uplink(&mut self, bytes: &[u8], _fport: u8) -> Result<(), NetError> {
            self.sent.push(bytes.to_vec());
            Ok(())
        }
        fn next_downlink(&mut self, buf: &mut [u8]) -> Option<usize> {
            let dl = self.downlinks.pop_front()?;
            buf[..dl.len()].copy_from_slice(&dl);
            Some(dl.len())
        }
    }

    #[derive(Default)]
    struct FakeValve {
        opens: u32,
        closes: u32,
    }
    impl Valve for FakeValve {
        async fn open(&mut self) {
            self.opens += 1;
        }
        async fn close(&mut self) {
            self.closes += 1;
        }
    }

    #[derive(Default)]
    struct FakeStore {
        record: Option<[u8; crate::record::RECORD_LEN]>,
    }
    impl Store for FakeStore {
        fn load_valve_state(&self) -> Option<ValveState> {
            self.record.and_then(|r| unpack_record(&r)).map(|(_, s)| s)
        }
        async fn persist(&mut self, keys: &LorawanKeys, state: ValveState) {
            self.record = Some(pack_record(keys, state));
        }
    }

    fn keys() -> LorawanKeys {
        LorawanKeys {
            deveui: [1; 8],
            appeui: [2; 8],
            appkey: [3; 16],
        }
    }

    /// The full client-side loop, end to end, with no device: a downlink flips
    /// the reported state, it's idempotent, and it survives a "reboot".
    #[test]
    fn downlink_flips_state_and_persists_across_reboot() {
        let k = keys();
        let mut net = FakeNetwork::default();
        let mut valve = FakeValve::default();
        let mut store = FakeStore::default();
        let mut state = ClientState::new(ValveState::Unknown);

        // Iter 1: nothing queued -> uplink reports Unknown/Unknown.
        pollster::block_on(run_iteration(&mut state, &k, &mut net, &mut valve, &mut store)).unwrap();
        assert_eq!(net.sent.last().unwrap().as_slice(), &[0x08, 0x03, 0x10, 0x03]);

        // Backend queues OPEN. Iter 2: actuate once, persist.
        net.downlinks.push_back(vec![0x08, 0x01]);
        pollster::block_on(run_iteration(&mut state, &k, &mut net, &mut valve, &mut store)).unwrap();
        assert_eq!(valve.opens, 1);
        assert_eq!(store.load_valve_state(), Some(ValveState::Open));

        // Iter 3: next uplink echoes OPEN.
        pollster::block_on(run_iteration(&mut state, &k, &mut net, &mut valve, &mut store)).unwrap();
        assert_eq!(net.sent.last().unwrap().as_slice(), &[0x08, 0x01, 0x10, 0x01]);

        // Backend re-sends OPEN while already open -> no new actuation.
        net.downlinks.push_back(vec![0x08, 0x01]);
        pollster::block_on(run_iteration(&mut state, &k, &mut net, &mut valve, &mut store)).unwrap();
        assert_eq!(valve.opens, 1); // unchanged — idempotent

        // Reboot: rebuild state from the persisted record; uplink still OPEN.
        let rebooted = ClientState::new(store.load_valve_state().unwrap());
        let (buf, n) = encode_uplink(rebooted.current, rebooted.last_commanded);
        assert_eq!(&buf[..n], &[0x08, 0x01, 0x10, 0x01]);
    }
}
