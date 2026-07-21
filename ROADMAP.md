# Roadmap: BLE-Provisioned Beacon → Class-A Polling Valve Controller

## Where You Are

A RAK4631 WisBlock firmware that joins LoRaWAN US915 (sub-band 2) via OTAA after BLE provisioning, then uplinks the literal bytes `"ponix"` on port 1 every 5 seconds. Downlinks are received and logged but not acted on. The host-side `lorawan_flash` (`lf`) CLI provisions OTAA keys over BLE; keys persist in the last 4 KB flash page and survive reboots. There is no actuator wired up, no protocol payload, and no command path — the device is a beacon, not a controller.

## Where You're Going

A Class-A device that polls the backend every 5 minutes and drives a single Rain Bird latching solenoid through a WisBlock RAK13002 H-bridge based on `Downlink` commands encoded with micropb, with hardware described by an in-tree KiCad schematic so the firmware is interchangeable between the WisBlock module and a future custom PCB.

---

## Phase 1: Workspace foundation ✅

Bring the existing standalone KiCad work into this repo and establish a single source of truth for pin assignments shared by schematic and firmware. No electronics work in this phase — paper and file moves only.

- [x] Copy `/Users/srall/development/kicad/rak4630-example/` into `flow_controller/hardware/` (renamed to `flow_controller.*`, internal refs rewritten, `.git`/history/backups dropped).
- [x] Identify hardware: RAK19007 base + RAK17001 H-bridge module (STSPIN250) on the dedicated IO slot; derive the four nRF52840 pins for PH/EN/PWM/FAULT (P0.04, P0.10, P0.21, P0.09) plus WB_IO2/P1.02 for IO-rail power. Cross-checked against SX1262 reserved pins.
- [x] Write `hardware/PINOUT.md` as the canonical pin mapping; reference it from a comment in `firmware/src/board.rs`. Schematic-side net labelling (DRV8837 → STSPIN250 reconciliation) deferred to Phase 4 with explicit deltas captured in PINOUT.md.
- [x] Top-level `README.md` created; `CLAUDE.md` Project Structure updated to include `hardware/`.

> **Why first:** every later phase either picks GPIOs (Phase 3 stub, Phase 4 real driver) or labels nets (Phase 4 schematic refinement). One source of truth up front prevents thrashing both files when a pin moves.

---

## Phase 2: Protocol foundation ✅

Define the wire format the backend and device share, and wire micropb codegen into the firmware build. Local generation only — no buf module push yet.

- [x] Add `proto/` directory with `buf.yaml` (v2, lint: STANDARD, breaking: FILE) and `flow_controller/v1/flow_controller.proto` containing `Uplink`, `Downlink`, and a single shared `ValveState` enum.
- [x] Pin `buf` and `protoc` in `.mise.toml`; add `proto:lint`/`proto:format`/`proto:gen`/`proto:check` tasks.
- [x] Wire micropb codegen via a host-only `proto_gen/` workspace crate that calls `micropb-gen`. Generated `.rs` is committed under `firmware/src/proto/`; `cargo build` of the firmware needs no `protoc`. **No `buf.gen.yaml`** — micropb has no `protoc-gen-micropb` plugin, so buf is lint/format-only here.
- [x] `proto/README.md`: workflow + deferred buf.build push.

> **Why second:** the codegen integration is the riskiest unknown in the protocol layer (micropb + buf + no_std + Embassy). Resolving it before any caller exists keeps the surprise contained. Phase 3 fails fast if this is wrong.

---

## Phase 3: End-to-end Class-A polling with stub actuator ✅

Replace the placeholder `"ponix"` payload with serialized `Uplink`, decode `Downlink`, and route commands into a stub valve module that updates flash state without touching hardware. Validates the entire command path before any wires move.

- [x] Change `UPLINK_INTERVAL_SECS` from `5` to `300` in `firmware/src/shared.rs`.
- [x] Add `firmware/src/valve.rs` with a `ValveState` enum and stub `open()` / `close()` that log via defmt and update an in-memory state — no GPIO yet.
- [x] Extend the flash record (was `MAGIC + DevEUI + AppEUI + AppKey`, 36 bytes) to include the persisted `ValveState` — magic bumped `LORA` → `FCV1`, 40-byte record, old records read as "no record". `firmware/src/flash.rs`.
- [x] Replace the hardcoded `b"ponix"` send in `lorawan.rs` with a serialized `Uplink` constructed from the persisted state.
- [x] Replace the existing `take_downlink` log-and-discard with: deserialize `Downlink` → if `desired_state` differs from `current_state`, call the stub `open()`/`close()`, persist new state, log the transition. Next uplink reflects it.
- [ ] Bench-test on the device: backend issues OPEN → device echoes `current_state = OPEN` in the next Uplink; same for CLOSED; same after a power cycle (state survives). _(pending hardware)_

> **Why third:** the full networking path — encoding, decoding, persistence, state echo — is exercised end-to-end with zero hardware risk. If this works, Phase 4 reduces to "make the GPIOs do what the stub claims."

---

## Phase 4: Real H-bridge actuator

Swap the Phase 3 stub for a driver that pulses the H-bridge to flip the latching solenoid. The protocol layer above is untouched.

- [ ] Bench-characterize the Rain Bird latching solenoid: minimum reliable pulse duration at the available rail voltage, measured across at least 10 actuations in each direction. Capture as constants `OPEN_PULSE_MS`, `CLOSE_PULSE_MS`.
- [ ] Implement the real driver in `firmware/src/valve.rs` using STSPIN250 PH/EN/PWM control: open = `(PH=H, PWM=H, EN=H for OPEN_PULSE_MS, then EN=L)`; close = `(PH=L, PWM=H, EN=H for CLOSE_PULSE_MS, then EN=L)`; coast otherwise. Drive `WB_IO2` high at boot to power the IO rail.
- [ ] **Reconcile schematic with `hardware/PINOUT.md`**: replace DRV8837 with STSPIN250, label the four control nets (PWM/PH/EN/FAULT), add the FAULT pull-up + REF current-limit resistor + sense resistor per STSPIN250 reference design. See `hardware/PINOUT.md` "Custom PCB schematic — required deltas".
- [ ] Add a local decoupling cap (≥100 µF) at the STSPIN250 VS pin in the schematic to absorb the inrush spike — verify on hardware that the rail doesn't droop enough to brown out the nRF52840 mid-pulse.
- [ ] End-to-end hardware validation: backend issues OPEN → physical valve clicks open → next Uplink echoes OPEN. Repeat across power cycles.
- [ ] Refresh `hardware/PINOUT.md` with any pin/voltage facts learned during characterization.

> **Why fourth:** by this point the protocol path has already been proven on a stub. Any failure here is isolated to driver/pulse/wiring, not networking, which makes debugging dramatically easier.

---

## Phase 5: Scheduling semantics

The backend may treat the device as either (i) a stateless executor that's told "right now, you should be X" on every poll, or (ii) a stateful agent that receives future-dated schedules and runs them locally. Default assumption is (i); this phase confirms or expands.

- [ ] Confirm the Phase 2 `Downlink` shape against the backend repo. If it sends `desired_state` only, this phase is closed — Phase 3 already implemented it.
- [ ] **Only if** the backend insists on schedule push: extend `Downlink` with start/duration fields; add LoRaWAN `DeviceTimeReq` clock sync; persist the active schedule alongside `ValveState`; add a periodic check that fires the open/close at the scheduled moment.

> **Why last:** the work in this phase is gated by a single decision that lives outside this repo. Sequencing it last lets that decision settle before any code is written.

---

## Open Questions to Resolve Along the Way

- **Backend scheduling model (stateless vs. stateful)** — assumed (i) stateless executor. Confirm against the backend repo before doing any work in Phase 5.
- **Confirmed vs. unconfirmed downlinks** — assumed best-effort with state echo (Class A natural mode). Each Uplink reports `last_commanded_state` so the backend can detect drops by absence of echo and resend.
- **Buf module publishing** — explicitly local-only for now. When the backend also consumes these protos in earnest, push to buf.build and dedupe.
- **Flow sensing** — out of scope for this roadmap (a SW3L-LS sensor exists in your notes). Worth a future roadmap if closed-loop control is desired.
- **Per-device BLE address and name, power management, fleet provisioning UX** — explicitly deferred ("fleet stuff later"). Revisit when more than one unit is in the field.
- **Single vs. dual valve** — single this round; the RAK13002 supports a second H-bridge, so doubling later is additive (one new pin pair, one new enum field) rather than disruptive.
