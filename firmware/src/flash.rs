use defmt::info;
use domain::record::{pack_record, unpack_record, RECORD_LEN};
use domain::{LorawanKeys, Store, ValveState};
use embassy_nrf::nvmc::Nvmc;
use embedded_storage::nor_flash::NorFlash;

use crate::shared::SharedNvmc;

/// Last 4 KB page of nRF52840 flash (1 MB total).
const STORAGE_ADDR: u32 = 0x000F_F000;

/// Read the raw record via memory-mapped access. The byte layout is validated
/// and interpreted by `domain::record`.
fn read_record() -> [u8; RECORD_LEN] {
    let mut buf = [0u8; RECORD_LEN];
    // Safety: reading from a known-good flash address via memory-mapped access.
    let flash_ptr = STORAGE_ADDR as *const u8;
    unsafe {
        core::ptr::copy_nonoverlapping(flash_ptr, buf.as_mut_ptr(), RECORD_LEN);
    }
    buf
}

/// Read LoRaWAN keys from flash. Returns `None` if no valid record exists.
pub(crate) fn read_keys() -> Option<LorawanKeys> {
    unpack_record(&read_record()).map(|(keys, _)| keys)
}

/// Read the persisted valve state. Returns `None` if no valid record exists.
pub(crate) fn read_valve_state() -> Option<ValveState> {
    unpack_record(&read_record()).map(|(_, state)| state)
}

/// Erase the storage page and write the full record (both halves always).
pub(crate) fn write_record(nvmc: &mut Nvmc<'_>, keys: &LorawanKeys, state: ValveState) {
    let buf = pack_record(keys, state);
    nvmc.erase(STORAGE_ADDR, STORAGE_ADDR + 4096).unwrap();
    nvmc.write(STORAGE_ADDR, &buf).unwrap();
}

/// Write keys, **preserving** the currently-persisted valve state (the BLE
/// provisioning path has no notion of valve state).
pub(crate) fn write_keys(nvmc: &mut Nvmc<'_>, keys: &LorawanKeys) {
    let state = read_valve_state().unwrap_or(ValveState::Unknown);
    write_record(nvmc, keys, state);
    info!("[flash] keys written");
}

/// Flash-backed [`domain::Store`] handed to the LoRaWAN loop.
pub(crate) struct FlashStore {
    nvmc: &'static SharedNvmc,
}

impl FlashStore {
    pub(crate) fn new(nvmc: &'static SharedNvmc) -> Self {
        Self { nvmc }
    }
}

impl Store for FlashStore {
    fn load_valve_state(&self) -> Option<ValveState> {
        read_valve_state()
    }

    async fn persist(&mut self, keys: &LorawanKeys, state: ValveState) {
        let mut guard = self.nvmc.lock().await;
        write_record(&mut guard, keys, state);
        info!("[flash] valve state written: {}", state.to_byte());
    }
}
