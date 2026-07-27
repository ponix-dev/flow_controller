# CLAUDE.md

## Overview
LoRaWAN flow controller for RAK4631 WisBlock (nRF52840 + SX1262) with BLE provisioning. The firmware advertises as a BLE peripheral; a host-side CLI connects as a BLE central to provision LoRaWAN OTAA keys (DevEUI, AppEUI, AppKey). Keys are persisted to flash and survive reboots.

Once joined, the device runs a Class-A poll loop (Phase 3): every 5 minutes it sends a serialized `proto::Uplink` carrying `current_state` + `last_commanded_state`, then decodes any `proto::Downlink` in the RX window and routes `desired_state` into `firmware/src/valve.rs`. The valve module is a **stub** — `open()`/`close()` log via defmt and persist state but drive no GPIO yet (the real STSPIN250 H-bridge driver is Phase 4). Valve state is persisted alongside the keys in a single 40-byte flash record with magic `b"FCV1"`.

## Project Structure
Cargo workspace with four crates plus a `proto/` directory:
- **`domain/`** — portable, host-testable business logic (`#![no_std]`, only depends on `micropb`). Holds the generated wire types, `ValveState`/`LorawanKeys`, the `Uplink`/`Downlink` codec, the 40-byte flash record format, the valve state machine, and the trait-generic loop body `run_iteration`. Runs under `cargo test -p domain` on the host — **no device, no radio, no flash**. The `firmware` crate implements its `Network`/`Valve`/`Store` traits with real hardware.
- **`firmware/`** — embedded firmware (`#![no_std]`, `thumbv7em-none-eabi`); board/wire-specific setup + real trait impls, depends on `domain`.
- **`lorawan_flash/`** — host-side BLE CLI (`lf` binary). See [`lorawan_flash/README.md`](lorawan_flash/README.md) for subcommand details and env-var contract.
- **`proto_gen/`** — host-only crate that runs `micropb-gen` to regenerate `domain/src/proto/`. Triggered by `mise run proto:gen`.
- **`proto/`** — wire-format definitions. `flow_controller/v1/valve.proto` is the schema; [`proto/README.md`](proto/README.md) explains the regen workflow and why there's no `buf.gen.yaml`.

KiCad schematic and PCB sources, along with the canonical GPIO pin assignments, live in the sibling `flow_controller_hardware/` repo.

## Commands

```bash
# Firmware
mise run build              # Build firmware
mise run flash              # Build and flash via probe-rs
mise run check              # Check firmware
mise run clippy             # Clippy on firmware

# CLI
mise run lf:scan            # Scan for BLE devices
mise run lf:provision       # Provision LoRaWAN keys (reads .env)

# Proto / micropb codegen
mise run proto:lint         # buf lint
mise run proto:format       # buf format -w (in place)
mise run proto:gen          # regenerate domain/src/proto/valve.rs
mise run proto:check        # CI helper: regen + verify clean git diff

# Domain (host-testable business logic)
cargo test -p domain        # unit + full-loop tests, no device required

# Explicit cargo commands
cargo build --release -p flow_controller --target thumbv7em-none-eabi
cargo build --release -p lorawan_flash
cargo run -p lorawan_flash -- scan
cargo run -p lorawan_flash -- provision  # uses DEVEUI/APPEUI/APPKEY/END_DEVICE env vars
cargo run -p proto_gen                   # equivalent to `mise run proto:gen`
```

## Architecture
- **Board**: RAK4631 WisBlock Core on RAK19007 base
- **MCU**: nRF52840 (Cortex-M4F)
- **Target**: `thumbv7em-none-eabi`
- **Radio**: SX1262 via SPI
- **Region**: US915, sub-band 2
- **H-bridge**: RAK17001 (STSPIN250), PH/EN/PWM control. GPIO assignments are canonical in the sibling `flow_controller_hardware/` repo, authoritative for both firmware and schematic.
- **Load**: single Rain Bird latching solenoid (DC, two-lead)
- **Framework**: Embassy async runtime with lora-rs LoRaWAN stack
- **Logging**: defmt via RTT

## BLE Contract
- **Device name**: `flow_ctrl`
- **Service UUID**: `12345678-1234-5678-1234-56789abcdef0`
- **LoRaWAN Keys Characteristic UUID**: `12345678-1234-5678-1234-56789abcdef2` (write, 32 bytes: DevEUI[8] + AppEUI[8] + AppKey[16])

## Environment Variables
Copy `.env.example` to `.env` and fill in your LoRaWAN keys:
- `DEVEUI` — 16 hex chars (8 bytes)
- `APPEUI` — 16 hex chars (8 bytes)
- `APPKEY` — 32 hex chars (16 bytes)
- `END_DEVICE` — BLE device name (default: `flow_ctrl`)

## Key Patterns
- `#![no_std]`, `#![no_main]` — no standard library (firmware only)
- Async/await via Embassy executor
- OTAA join with jittered retry backoff
- `default-members = ["domain", "lorawan_flash", "proto_gen"]` — bare `cargo build` builds the host crates; firmware requires explicit `--target`
- LoRaWAN keys **and** valve state persisted to nRF52840 flash (last 4 KB page) in one `b"FCV1"` record; `Nvmc` is shared between the BLE and LoRaWAN tasks via an `embassy_sync` async `Mutex` (`shared::SharedNvmc`)
