# flow_controller

LoRaWAN-controlled drip-irrigation valve. RAK4631 firmware that joins LoRaWAN
US915 OTAA, polls every 5 minutes for downlink commands, and drives a Rain Bird
latching solenoid via a RAK17001 H-bridge module.

OTAA keys are provisioned over BLE by a host-side CLI; keys persist in flash.

## Workspace layout

```
firmware/        # embedded firmware (no_std, thumbv7em-none-eabi, Embassy)
lorawan_flash/   # host-side BLE provisioning CLI (lf binary)
hardware/        # KiCad schematic + pinout source-of-truth (PINOUT.md)
proto/           # micropb message definitions (added in Phase 2)
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
