# Pin Assignments

This file is the **single source of truth** for GPIO assignments on this board.
The firmware in `firmware/src/board.rs` and the schematic in
`hardware/flow_controller.kicad_sch` MUST agree with the table below. **When
changing any pin, update all three together** (this doc, the firmware, the
schematic net labels).

The point of pinning these here is that the firmware should be **interchangeable
between the off-the-shelf WisBlock module setup and a future custom PCB** — both
must follow this map.

## Targets

The firmware is meant to be byte-identical across two boards:

1. **WisBlock prototype**: RAK4631 Core + RAK19007 base + RAK17001 H-bridge module.
2. **Custom PCB**: RAK4630 module wired chip-level on the in-tree schematic.

**Both targets use the same H-bridge chip — STSPIN250.** The RAK17001 is an
STSPIN250 carrier, and the custom PCB schematic must use a discrete STSPIN250
(not the DRV8837 currently drawn). Same control pins, same firmware. See
"Custom PCB schematic — required deltas" below.

## Hardware configuration (WisBlock prototype)

- **Core**: RAK4631 (nRF52840 + SX1262)
- **Base**: RAK19007 WisBlock Base 2nd Gen
- **H-bridge**: RAK17001 (STSPIN250) — single-channel H-bridge
- **Slot used**: the dedicated **IO slot** on the RAK19007 (not sensor slots A–D), which exposes WB_IO2 through WB_IO6
- **Load**: one Rain Bird latching solenoid (DC, two leads)

## H-bridge pin map (RAK17001 ↔ RAK4631)

| Function   | WisBlock macro | nRF52840 GPIO | Direction | Notes                                                                                       |
| ---------- | -------------- | ------------- | --------- | ------------------------------------------------------------------------------------------- |
| IO power   | `WB_IO2`       | **P1.02**     | output    | Drive HIGH at boot to enable the IO-rail LDO that powers the RAK17001                       |
| PWM        | `WB_IO3`       | **P0.21**     | output    | Current/speed input on STSPIN250. HIGH during pulse, LOW otherwise                          |
| PH (phase) | `WB_IO4`       | **P0.04**     | output    | Direction. HIGH = OPEN polarity, LOW = CLOSE polarity (calibrate sign in Phase 4 if needed) |
| FAULT      | `WB_IO5`       | **P0.09**     | input     | Active-low fault from STSPIN250 (overcurrent / undervoltage / thermal). Optional telemetry  |
| EN         | `WB_IO6`       | **P0.10**     | output    | Bridge enable. HIGH for the duration of the actuation pulse, LOW to coast (high-Z outputs)  |

### Note on `WB_IO4` (P0.04)

P0.04 is also `AIN2` / `WB_A0`. Using it as a digital output for PH means
`WB_A0` is unavailable for analog reads on this board. Acceptable here — we have
no analog needs.

## Reserved pins (in use elsewhere — do not assign)

SX1262 LoRa radio (see `firmware/src/main.rs`):

| nRF52840 | Function          |
| -------- | ----------------- |
| P1.05    | RF switch RX      |
| P1.06    | SX1262 RESET      |
| P1.07    | RF switch TX      |
| P1.10    | SX1262 NSS (CS)   |
| P1.11    | SPI SCK           |
| P1.12    | SPI MISO          |
| P1.13    | SPI MOSI          |
| P1.14    | SX1262 BUSY       |
| P1.15    | SX1262 DIO1 (IRQ) |

BLE: uses internal soft-device controller (no external pins beyond the on-module antenna).
BLE address is currently a hardcoded random value `f0:10:42:d0:cb:ee` —
deferred to the fleet roadmap.

## Actuation sequence (firmware reference)

This is the pulse sequence Phase 4 will implement against the pins above.
Pulse durations are placeholders; bench-calibrate the actual values.

```
boot:
  WB_IO2 (P1.02) ← HIGH               # power the IO rail
  WB_IO3 (PWM)   ← LOW
  WB_IO4 (PH)    ← LOW
  WB_IO6 (EN)    ← LOW                # coast

open():
  WB_IO4 (PH)    ← HIGH               # set direction
  WB_IO3 (PWM)   ← HIGH
  WB_IO6 (EN)    ← HIGH               # bridge active
  delay(OPEN_PULSE_MS)                # ~50ms target, calibrate on bench
  WB_IO6 (EN)    ← LOW                # coast
  WB_IO3 (PWM)   ← LOW

close():
  WB_IO4 (PH)    ← LOW                # opposite direction
  WB_IO3 (PWM)   ← HIGH
  WB_IO6 (EN)    ← HIGH
  delay(CLOSE_PULSE_MS)
  WB_IO6 (EN)    ← LOW
  WB_IO3 (PWM)   ← LOW
```

Never drive PH while EN is HIGH and PWM is HIGH for longer than the calibrated
pulse — that runs continuous current through a latching solenoid that's already
latched, wasting power.

## Custom PCB schematic — required deltas (Phase 4 work)

The current `flow_controller.kicad_sch` is inherited from an earlier
`rak4630-example`. It uses a **DRV8837** (IN1/IN2-style H-bridge), which has a
different control protocol from the STSPIN250 the firmware targets. Phase 4
reconciliation must:

1. **Replace U2 (DRV8837) with STSPIN250** (TSSOP-16 or VFQFPN-16). KiCad ships
   the symbol/footprint in `Driver_Motor`; preferable to adding to the project
   library. Repackage choice noted in BOM.
2. **Add a 10 kΩ pull-up from FAULT to +3V3** — STSPIN250's FAULT is open-drain.
3. **Add the REF current-limit resistor** to the REF pin (sets peak current —
   spec a value that allows ~500 mA peak for the latching solenoid pulse, with
   margin).
4. **Add a current-sense resistor** on the SENSE pin per STSPIN250 reference
   design, and a small RC filter on REF if instability is observed.
5. **Wire VS = +9V** (battery rail) and **VCC = +3V3** (logic). Both rails
   already exist in the schematic.
6. **Label the four control nets with functional names** so the schematic and
   firmware speak the same vocabulary:
   - `PWM` ← from RAK4630 `P0.21` (= WB_IO3 on prototype)
   - `PH` ← from RAK4630 `P0.04` (= WB_IO4 on prototype)
   - `EN` ← from RAK4630 `P0.10` (= WB_IO6 on prototype)
   - `FAULT` → to RAK4630 `P0.09` (= WB_IO5 on prototype)
7. **Confirm `BATT_SENSE` ADC pin** — current schematic wires the divider
   output to an unlabelled RAK4630 pin; document which one and add it to the
   reserved-pins table (battery monitoring is deferred per "fleet stuff later"
   but the wire shouldn't conflict with the four control pins above).

The PCB layout (`flow_controller.kicad_pcb`) is currently empty — no tracks to
rework, so the chip swap is essentially redrawing U2 in the schematic and
re-running ERC.

## References

- [RAK17001 Datasheet](https://docs.rakwireless.com/product-categories/wisblock/rak17001/datasheet/)
- [RAK17001 Arduino example (`master`)](https://github.com/RAKWireless/WisBlock/blob/master/examples/RAK4630/IO/RAK17001_HBridge_STSPIN250/RAK17001_HBridge_STSPIN250.ino)
- [RAK19007 Datasheet](https://docs.rakwireless.com/product-categories/wisblock/rak19007/datasheet/)
- [RAK4631 WB_IO macro mapping (Meshtastic)](https://meshtastic.org/docs/hardware/devices/rak-wireless/wisblock/core-module/)
- [STSPIN250 product page](https://www.st.com/en/motor-drivers/stspin250.html)
