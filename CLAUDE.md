# CLAUDE.md

## Overview
LoRaWAN flow controller for RAK4631 WisBlock (nRF52840 + SX1262) with BLE provisioning. The firmware advertises as a BLE peripheral; a host-side CLI connects as a BLE central to provision LoRaWAN OTAA keys (DevEUI, AppEUI, AppKey). Keys are persisted to flash and survive reboots.

## Project Structure
Cargo workspace with two crates plus a hardware directory:
- **`firmware/`** — embedded firmware (`#![no_std]`, `thumbv7em-none-eabi`)
- **`lorawan_flash/`** — host-side BLE CLI (`lf` binary). See [`lorawan_flash/README.md`](lorawan_flash/README.md) for subcommand details and env-var contract.
- **`hardware/`** — KiCad schematic and PCB sources. [`hardware/PINOUT.md`](hardware/PINOUT.md) is the **single source of truth** for GPIO assignments — firmware and schematic must agree with it.

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

# Explicit cargo commands
cargo build --release -p flow_controller --target thumbv7em-none-eabi
cargo build --release -p lorawan_flash
cargo run -p lorawan_flash -- scan
cargo run -p lorawan_flash -- provision  # uses DEVEUI/APPEUI/APPKEY/END_DEVICE env vars
```

## Architecture
- **Board**: RAK4631 WisBlock
- **MCU**: nRF52840 (Cortex-M4F)
- **Target**: `thumbv7em-none-eabi`
- **Radio**: SX1262 via SPI
- **Region**: US915, sub-band 2
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
- `default-members = ["lorawan_flash"]` — bare `cargo build` only builds the CLI; firmware requires explicit `--target`
- LoRaWAN keys provisioned via BLE and persisted to nRF52840 flash (last 4 KB page)
