# flow_controller

LoRaWAN-controlled drip-irrigation valve. The end state is a Class-A device that
polls every 5 minutes for downlink commands and drives a Rain Bird latching
solenoid via a RAK17001 H-bridge module on a RAK4631 WisBlock.

OTAA keys are provisioned over BLE by a host-side CLI; keys persist in flash.

## Status

- **Today**: BLE-provisioned beacon — joins LoRaWAN US915 OTAA, sends a
  placeholder uplink every 5 s, ignores downlinks. No actuator wired up yet.
- **Phase 1** (workspace foundation, hardware in-tree, pinout doc): **complete**.
- **Phase 2** (`proto/` + `buf` lint + micropb codegen pipeline): **complete** —
  generated types compile into the firmware but aren't yet wired into the
  uplink/downlink path.
- **Up next**: Phase 3 — replace the placeholder `b"ponix"` payload with a
  serialized `Uplink`, decode `Downlink` and route to a stub valve. See
  [`ROADMAP.md`](ROADMAP.md).

## Workspace layout

```
firmware/        # embedded firmware (no_std, thumbv7em-none-eabi, Embassy)
lorawan_flash/   # host-side BLE provisioning CLI (lf binary)
proto/           # wire-format definitions (.proto, buf.yaml)
proto_gen/       # host crate; runs micropb-gen to regenerate firmware/src/proto/
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
