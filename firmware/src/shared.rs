use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

// ── Shared state ────────────────────────────────────────────────────────────

/// LoRaWAN OTAA credentials received via BLE provisioning.
pub(crate) struct LorawanKeys {
    pub deveui: [u8; 8],
    pub appeui: [u8; 8],
    pub appkey: [u8; 16],
}

/// Signal used by BLE (or boot flash read) to deliver keys to the LoRaWAN task.
pub(crate) static LORAWAN_KEYS: Signal<CriticalSectionRawMutex, LorawanKeys> = Signal::new();

// ── Constants ───────────────────────────────────────────────────────────────

pub(crate) const MAX_TX_POWER: u8 = 14;
pub(crate) const UPLINK_INTERVAL_SECS: u64 = 5;

/// L2CAP buffers per link
pub(crate) const L2CAP_TXQ: u8 = 3;
pub(crate) const L2CAP_RXQ: u8 = 3;
pub(crate) const CONNECTIONS_MAX: usize = 1;
pub(crate) const L2CAP_CHANNELS_MAX: usize = 2; // Signal + ATT
