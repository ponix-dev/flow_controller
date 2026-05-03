# Phase 1 — Hardware Foundation

**Roadmap reference**: ROADMAP.md, Phase 1
**Branch**: `feat/hardware-foundation`
**Created**: 2026-05-03

## Goal

Bring the existing standalone KiCad work into this repo and establish a single source of truth for the GPIO pin assignment shared by the schematic and the firmware. No electronics work in this phase — file moves and documentation only.

## Context

A standalone repo at `/Users/srall/development/kicad/rak4630-example/` already has a real schematic (`rak4630-example.kicad_sch`, ~65 KB) plus libraries and an empty PCB file. It has its own `.git` (single commit, no remote — disposable history) and its own `CLAUDE.md`/`ROADMAP.md`/`README.md`/`mise.toml` scoped to that standalone project.

The user is currently driving a Rain Bird latching solenoid via a WisBlock H-bridge IO module. Exact RAK part number, base board, and IO slot are TBC — gathering those is a step in this plan, not an assumption.

The "interchangeability" requirement: firmware GPIO pin numbers must match whatever the WisBlock module exposes by default, **and** the schematic in `hardware/` must use the same pins so a future custom PCB layout produces a board the same firmware will work with unmodified.

## Steps

### 1. Import the KiCad project into `hardware/`

Copy from `/Users/srall/development/kicad/rak4630-example/` into `flow_controller/hardware/`. Excluded paths:

- `.git/` — drop, single-commit history, no remote
- `.history/` — local IDE history, no value across machines
- `*-backups/` — KiCad-generated, regenerable
- `build/` — regenerable
- `.vscode/` — workspace IDE config, may not match this repo's setup

Kept paths:

- `*.kicad_sch`, `*.kicad_pcb`, `*.kicad_pro`, `*.kicad_prl`
- `libraries/`, `fp-lib-table`, `sym-lib-table`
- `*-erc.json` (KiCad ERC report — useful baseline)
- `.gitignore` — merge into the parent's `.gitignore` (don't keep a nested one)

Drop these files (their content was scoped to the standalone project; the parent repo has equivalents):

- `CLAUDE.md`, `ROADMAP.md`, `README.md`, `mise.toml`

Validation: `kicad-cli sch erc hardware/rak4630-example.kicad_sch` (or open in KiCad and confirm no broken library references).

**Open decision**: rename project files from `rak4630-example` → `flow_controller` now, or defer? **Recommendation: defer** — renaming KiCad project files touches the `.kicad_pro` and library paths and risks breaking the schematic open. Do it in a follow-up if at all; the directory is `hardware/` regardless of internal filename.

### 2. Identify the bench hardware

Need from the user (capture in `hardware/PINOUT.md`):

- WisBlock base board RAK part number (most common: RAK19007).
- WisBlock H-bridge module RAK part number.
- Which IO slot the H-bridge is plugged into (IO_1 / IO_2 / etc.).
- Confirm: single H-bridge channel in use (per Round 3 decision — single valve).

### 3. Derive the nRF52840 pin numbers

For the chosen base board, look up the IO-slot pinout (RAKwireless publishes per-base "Datasheet" pages with the slot pin maps). Map:

- IO-slot pin → nRF52840 GPIO (e.g. `IO_1.SDA` → `P0.13`)
- H-bridge module's IN1/IN2 → IO-slot pin (from the H-bridge module's datasheet)
- Compose: H-bridge IN1/IN2 → nRF52840 pin numbers

Cross-check against `firmware/src/main.rs` and `firmware/src/board.rs` for conflicts with already-used pins (BLE/SPI/UART/SX1262 wiring). The SX1262 already uses P1_05/06/07/10/11/12/13/14/15 — make sure the chosen IN1/IN2 don't collide.

### 4. Write `hardware/PINOUT.md`

Single source of truth. Structure:

```markdown
# Pin Assignments

This file is the **single source of truth** for GPIO assignments. The firmware
in `firmware/src/board.rs` and the schematic in `hardware/*.kicad_sch` MUST
agree with this table. When changing pins, update all three together.

## Hardware
- Base: RAK<xxxxx>
- H-bridge: RAK<xxxxx>, IO slot: IO_<n>
- MCU: nRF52840 (Cortex-M4F) on RAK4631

## Pin Table
| nRF52840 | WisBlock label | Function     | Direction | Notes |
| -------- | -------------- | ------------ | --------- | ----- |
| P0.xx    | IO_<n>.<lbl>   | H-bridge IN1 | output    | OPEN pulse polarity |
| P0.yy    | IO_<n>.<lbl>   | H-bridge IN2 | output    | CLOSE pulse polarity |

## Reserved (in use elsewhere)
- SX1262 SPI: P1_05 (RF_RX), P1_06 (RST), P1_07 (RF_TX), P1_10 (NSS),
  P1_11 (SCK), P1_12 (MISO), P1_13 (MOSI), P1_14 (BUSY), P1_15 (DIO1)
- BLE address: hardcoded random `f0:10:42:d0:cb:ee` (deferred per fleet roadmap)
```

### 5. Wire the references

- `firmware/src/board.rs`: add a top-of-file comment: `// Pin assignments — see hardware/PINOUT.md (single source of truth).`
- Schematic: ensure the H-bridge IN1/IN2 nets are labelled with the WisBlock pin labels and/or nRF52840 pin numbers from the table. (Visual check — actual schematic editing if labels are missing.)
- New top-level `README.md`: brief workspace overview pointing at `firmware/`, `lorawan_flash/`, `hardware/`, plus future `proto/`. CLAUDE.md already exists; add `hardware/` to its Project Structure section.

## Acceptance criteria

- [ ] `hardware/` directory exists in this repo with the schematic and supporting KiCad files; opens cleanly in KiCad.
- [ ] `hardware/PINOUT.md` exists with the pin table populated (real pin numbers, not placeholders).
- [ ] `firmware/src/board.rs` contains a comment pointing at `hardware/PINOUT.md`.
- [ ] `CLAUDE.md` Project Structure section mentions `hardware/`.
- [ ] Top-level `README.md` exists (currently absent) and references all three directories.
- [ ] Schematic net labels visually agree with the pin table.
- [ ] The standalone repo at `/Users/srall/development/kicad/rak4630-example/` is left untouched (don't delete the source).

## Open questions captured during planning

- Project rename (deferred — see Step 1).
- Buf module publishing remains deferred from ROADMAP.md.
- This plan does **not** modify `firmware/src/board.rs` beyond a single comment — no GPIO peripheral construction yet. That happens in Phase 3 (stub) and Phase 4 (real driver).
