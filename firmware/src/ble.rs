use defmt::{error, info, warn};
use embassy_nrf::nvmc::Nvmc;
use embassy_time::{Duration, Timer};
use trouble_host::prelude::*;

use crate::flash;
use crate::shared::{CONNECTIONS_MAX, L2CAP_CHANNELS_MAX, LORAWAN_KEYS, LorawanKeys};

// ── GATT Service ────────────────────────────────────────────────────────────
// UUIDs must match cli/src/main.rs:
//   Service:        12345678-1234-5678-1234-56789abcdef0
//   Provision char: 12345678-1234-5678-1234-56789abcdef2

#[gatt_server]
struct ProvisionServer {
    provision_service: ProvisionService,
}

#[gatt_service(uuid = "12345678-1234-5678-1234-56789abcdef0")]
struct ProvisionService {
    #[characteristic(uuid = "12345678-1234-5678-1234-56789abcdef2", read, write)]
    lorawan_keys: [u8; 32],
}

// ── BLE helpers ─────────────────────────────────────────────────────────────

async fn ble_task<C: Controller>(mut runner: Runner<'_, C, DefaultPacketPool>) {
    loop {
        if let Err(e) = runner.run().await {
            let e = defmt::Debug2Format(&e);
            defmt::panic!("[ble] runner error: {:?}", e);
        }
    }
}

async fn advertise<'values, 'server, C: Controller>(
    peripheral: &mut Peripheral<'values, C, DefaultPacketPool>,
    server: &'server ProvisionServer<'values>,
) -> Result<GattConnection<'values, 'server, DefaultPacketPool>, BleHostError<C::Error>> {
    let mut adv_data = [0; 31];
    let len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::CompleteLocalName(b"flow_ctrl"),
        ],
        &mut adv_data[..],
    )?;
    let advertiser = peripheral
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &adv_data[..len],
                scan_data: &[],
            },
        )
        .await?;
    info!("[ble] advertising as 'flow_ctrl'");
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    info!("[ble] connection established");
    Ok(conn)
}

async fn gatt_events_task(
    server: &ProvisionServer<'_>,
    conn: &GattConnection<'_, '_, DefaultPacketPool>,
    nvmc: &mut Nvmc<'_>,
) -> Result<(), Error> {
    let keys_handle = server.provision_service.lorawan_keys;
    let reason = loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => break reason,
            GattConnectionEvent::Gatt { event } => {
                let mut pending_keys = None;
                if let GattEvent::Write(event) = &event {
                    if event.handle() == keys_handle.handle {
                        let data = event.data();
                        if data.len() == 32 {
                            let mut deveui = [0u8; 8];
                            let mut appeui = [0u8; 8];
                            let mut appkey = [0u8; 16];
                            deveui.copy_from_slice(&data[0..8]);
                            appeui.copy_from_slice(&data[8..16]);
                            appkey.copy_from_slice(&data[16..32]);
                            pending_keys = Some(LorawanKeys {
                                deveui,
                                appeui,
                                appkey,
                            });
                        } else {
                            warn!(
                                "[ble] invalid key payload length: {}, expected 32",
                                data.len()
                            );
                        }
                    }
                }
                // Send the GATT reply before flash write (flash erase halts the CPU)
                match event.accept() {
                    Ok(reply) => reply.send().await,
                    Err(e) => warn!("[ble] error sending reply: {:?}", e),
                }
                if let Some(keys) = pending_keys {
                    let existing = flash::read_keys();
                    if existing.as_ref() == Some(&keys) {
                        info!("[ble] keys unchanged, skipping update");
                    } else {
                        info!("[ble] LoRaWAN keys provisioned, DevEUI: {:02x}", keys.deveui);
                        flash::write_keys(nvmc, &keys);
                        LORAWAN_KEYS.signal(keys);
                    }
                }
            }
            _ => {}
        }
    };
    info!("[ble] disconnected: {:?}", reason);
    Ok(())
}

// ── Public entry point ──────────────────────────────────────────────────────

pub(crate) async fn run_ble(
    controller: nrf_sdc::SoftdeviceController<'static>,
    mut nvmc: Nvmc<'static>,
) {
    let address = Address::random([0xf0, 0x10, 0x42, 0xd0, 0xcb, 0xee]);
    info!("[ble] address = {:?}", address);

    let mut resources: HostResources<_, DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
        HostResources::new();
    let stack = trouble_host::new(controller, &mut resources)
        .set_random_address(address)
        .build();
    let runner = stack.runner();
    let mut peripheral = stack.peripheral();

    let server = ProvisionServer::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: "flow_ctrl",
        appearance: &appearance::sensor::GENERIC_SENSOR,
    }))
    .unwrap();

    info!("[ble] starting advertising and GATT service");

    let _ = embassy_futures::join::join(ble_task(runner), async {
        loop {
            match advertise(&mut peripheral, &server).await {
                Ok(conn) => {
                    let _ = gatt_events_task(&server, &conn, &mut nvmc).await;
                }
                Err(e) => {
                    let e = defmt::Debug2Format(&e);
                    error!("[ble] advertise error: {:?}", e);
                    Timer::after(Duration::from_secs(1)).await;
                }
            }
        }
    })
    .await;
}
