# flow_controller

LoRaWAN-controlled drip-irrigation valve. The end state is a Class-A device that
polls every 5 minutes for downlink commands and drives a Rain Bird latching
solenoid via a RAK17001 H-bridge module on a RAK4631 WisBlock.

OTAA keys are provisioned over BLE by a host-side CLI; keys persist in flash.

## Status

- **Today**: Class-A polling with a **stub** actuator — joins LoRaWAN US915
  OTAA, sends a serialized `Uplink` (current + last-commanded valve state)
  every 5 min, decodes `Downlink` commands and routes them into a stub valve
  module that persists state to flash but drives no GPIO yet.
- **Phase 1** (workspace foundation, hardware in-tree, pinout doc): **complete**.
- **Phase 2** (`proto/` + `buf` lint + micropb codegen pipeline): **complete**.
- **Phase 3** (end-to-end command path, stub actuator, `FCV1` flash record):
  **complete** — the full encode/decode/persist/echo path is exercised with no
  hardware risk.
- **Up next**: Phase 4 — swap the stub for a real STSPIN250 H-bridge driver
  that pulses the latching solenoid, and reconcile the schematic. See
  [`ROADMAP.md`](ROADMAP.md).

## Workspace layout

```
domain/          # portable, host-testable business logic (no_std; codec, record, state machine, loop)
firmware/        # embedded firmware (no_std, thumbv7em-none-eabi, Embassy); board/wire setup + trait impls
lorawan_flash/   # host-side BLE provisioning CLI (lf binary)
proto/           # wire-format definitions (.proto, buf.yaml)
proto_gen/       # host crate; runs micropb-gen to regenerate domain/src/proto/
hardware/        # KiCad schematic + pinout source-of-truth (PINOUT.md)
ROADMAP.md       # phased plan, see for current state and direction
```

## Where to start

- **Adding a feature or fixing firmware** — see `firmware/` and the overview in
  [`CLAUDE.md`](CLAUDE.md). For pin assignments, [`hardware/PINOUT.md`](hardware/PINOUT.md)
  is authoritative.
- **Provisioning OTAA keys onto a device** — see [`lorawan_flash/README.md`](lorawan_flash/README.md).
- **Project direction and what to work on next** — see [`ROADMAP.md`](ROADMAP.md).

## Build / flash

```bash
mise run build              # build firmware
mise run flash              # flash via probe-rs
mise run lf:scan            # scan for BLE devices
mise run lf:provision       # provision LoRaWAN keys (reads .env)
```

See [`CLAUDE.md`](CLAUDE.md) for the full command reference and the explicit
cargo invocations.
