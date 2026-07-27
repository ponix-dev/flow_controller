use domain::LorawanKeys;
use embassy_nrf::nvmc::Nvmc;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;

// ── Shared state ────────────────────────────────────────────────────────────

/// Signal used by BLE (or boot flash read) to deliver keys to the LoRaWAN task.
pub(crate) static LORAWAN_KEYS: Signal<CriticalSectionRawMutex, LorawanKeys> = Signal::new();

/// Shared flash controller. Both the BLE task (key writes) and the LoRaWAN task
/// (valve-state writes) need to erase/write the storage page, so `Nvmc` lives
/// behind an async mutex handed to both. Writes are rare and page erase is
/// ~85 ms; if that latency ever disrupts LoRaWAN RX timing, move all writes to
/// a dedicated flash-writer task fed by a channel (plan option B).
pub(crate) type SharedNvmc = Mutex<CriticalSectionRawMutex, Nvmc<'static>>;

// ── Constants ───────────────────────────────────────────────────────────────

pub(crate) const MAX_TX_POWER: u8 = 14;
/// Class-A poll cadence. 300 s (5 min) per the Phase 3 roadmap.
pub(crate) const UPLINK_INTERVAL_SECS: u64 = 300;

/// L2CAP buffers per link
pub(crate) const L2CAP_TXQ: u8 = 3;
pub(crate) const L2CAP_RXQ: u8 = 3;
pub(crate) const CONNECTIONS_MAX: usize = 1;
pub(crate) const L2CAP_CHANNELS_MAX: usize = 2; // Signal + ATT
