use defmt::info;
use embassy_nrf::nvmc::Nvmc;
use embedded_storage::nor_flash::NorFlash;

use crate::shared::LorawanKeys;

/// Last 4 KB page of nRF52840 flash (1 MB total).
const STORAGE_ADDR: u32 = 0x000F_F000;
const MAGIC: [u8; 4] = *b"LORA";
/// MAGIC(4) + DEVEUI(8) + APPEUI(8) + APPKEY(16) = 36 bytes.
const RECORD_LEN: usize = 36;

/// Read LoRaWAN keys from flash. Returns `None` if no valid record exists.
pub(crate) fn read_keys() -> Option<LorawanKeys> {
    let mut buf = [0u8; RECORD_LEN];
    // Safety: reading from a known-good flash address via memory-mapped access.
    let flash_ptr = STORAGE_ADDR as *const u8;
    unsafe {
        core::ptr::copy_nonoverlapping(flash_ptr, buf.as_mut_ptr(), RECORD_LEN);
    }

    if buf[0..4] != MAGIC {
        return None;
    }

    let mut deveui = [0u8; 8];
    let mut appeui = [0u8; 8];
    let mut appkey = [0u8; 16];
    deveui.copy_from_slice(&buf[4..12]);
    appeui.copy_from_slice(&buf[12..20]);
    appkey.copy_from_slice(&buf[20..36]);

    Some(LorawanKeys {
        deveui,
        appeui,
        appkey,
    })
}

/// Erase the storage page and write keys to flash.
pub(crate) fn write_keys(nvmc: &mut Nvmc<'_>, keys: &LorawanKeys) {
    let mut buf = [0u8; RECORD_LEN];
    buf[0..4].copy_from_slice(&MAGIC);
    buf[4..12].copy_from_slice(&keys.deveui);
    buf[12..20].copy_from_slice(&keys.appeui);
    buf[20..36].copy_from_slice(&keys.appkey);

    // Erase the 4 KB page, then write the record.
    nvmc.erase(STORAGE_ADDR, STORAGE_ADDR + 4096).unwrap();
    nvmc.write(STORAGE_ADDR, &buf).unwrap();
    info!("[flash] keys written");
}
