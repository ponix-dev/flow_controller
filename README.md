# flow_controller

LoRaWAN-controlled drip-irrigation valve. The end state is a Class-A device that
polls every 5 minutes for downlink commands and drives a Rain Bird latching
solenoid via a RAK17001 H-bridge module on a RAK4631 WisBlock.

OTAA keys are provisioned over BLE by a host-side CLI; keys persist in flash.

## Status

Roughly halfway: the full command path works in software, but the actuator is
still a **stub** — no GPIO moves the valve yet.

**Done**
- Joins LoRaWAN US915 (OTAA) after BLE key provisioning; keys persist in flash.
- Each poll, sends a serialized `Uplink` (current + last-commanded valve state).
- Decodes `Downlink` commands, routes them through the valve state machine,
  persists the new state to flash, and echoes it on the next uplink.

**Next**
- Replace the stub with a real STSPIN250 H-bridge driver that pulses the
  latching solenoid, and reconcile the schematic.
- Validate the downlink RX path and physical actuation on hardware.

## Workspace layout

```
domain/          # portable, host-testable business logic (no_std; codec, record, state machine, loop)
firmware/        # embedded firmware (no_std, thumbv7em-none-eabi, Embassy); board/wire setup + trait impls
lorawan_flash/   # host-side BLE provisioning CLI (lf binary)
proto/           # wire-format definitions (.proto, buf.yaml)
proto_gen/       # host crate; runs micropb-gen to regenerate domain/src/proto/
```

## Where to start

- **Adding a feature or fixing firmware** — see `firmware/` and the overview in
  [`CLAUDE.md`](CLAUDE.md). KiCad schematic and pin assignments live in the sibling
  `flow_controller_hardware/` repo.
- **Provisioning OTAA keys onto a device** — see [`lorawan_flash/README.md`](lorawan_flash/README.md).

## Build / flash

```bash
mise run build              # build firmware
mise run flash              # flash via probe-rs
mise run lf:scan            # scan for BLE devices
mise run lf:provision       # provision LoRaWAN keys (reads .env)
```

See [`CLAUDE.md`](CLAUDE.md) for the full command reference and the explicit
cargo invocations.
