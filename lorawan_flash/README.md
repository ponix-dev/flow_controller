# lorawan_flash

Host-side BLE central CLI for provisioning LoRaWAN OTAA keys to a `flow_ctrl` firmware device over BLE.

Installed as the `lf` binary (via `mise run lf:install` or `cargo install --path lorawan_flash`).

## Subcommands

### `lf scan [--duration <secs>]`
Scans for nearby BLE peripherals and prints `name (id)` for each. Useful for confirming the firmware is advertising as `flow_ctrl` before provisioning. Defaults to a 5-second scan.

### `lf provision --deveui <hex> --appeui <hex> --appkey <hex> --name <ble_name>`
Connects to the named peripheral, verifies it exposes the provisioning service, and writes a 32-byte payload (`DevEUI[8] || AppEUI[8] || AppKey[16]`) to the LoRaWAN Keys characteristic using `WriteType::WithResponse` (so the call only returns after the firmware acknowledges the write). Then disconnects.

All four flags can be supplied via env vars instead — `DEVEUI`, `APPEUI`, `APPKEY`, `END_DEVICE` — which is how `mise run lf:provision` works (it loads `.env` automatically via `dotenvy`).

## BLE contract
Must match the firmware:
- Service UUID: `12345678-1234-5678-1234-56789abcdef0`
- LoRaWAN Keys characteristic UUID: `12345678-1234-5678-1234-56789abcdef2`
- Payload: exactly 32 bytes, `DevEUI[8] || AppEUI[8] || AppKey[16]` (big-endian hex as written on the network server)

## Behavior notes
- Re-provisioning a running device is safe: the firmware compares the new payload against what's already in flash and skips the write if unchanged; otherwise it persists the new keys and triggers a clean LoRaWAN rejoin.
- Hex inputs are validated for length (8/8/16 bytes) before any BLE traffic.
- Uses `btleplug` for cross-platform BLE (works on macOS and Linux).
