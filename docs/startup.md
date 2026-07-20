# Firmware Startup

What happens from power-on until the device is advertising over BLE and waiting
to join a LoRaWAN network. All of this lives in [`src/main.rs`](../firmware/src/main.rs).

## Why the order matters

The nRF52840 has **one** physical radio that BLE and LoRaWAN both use. Nordic's
Multiprotocol Service Layer (MPSL) time-slices it, and the SoftDevice Controller
(SDC) sits on top for the BLE link layer. Because of this, bring-up follows a
fixed order: clocks and MPSL first, then SDC, then the application radios and
tasks. SDC also takes ownership of the hardware RNG, so we must capture an
entropy seed for LoRaWAN *before* SDC is built.

## Boot sequence

```mermaid
flowchart TD
    A[power on / reset] --> B[embassy_nrf::init]
    B --> C[MPSL init<br/>RTC0, TIMER0, TEMP, PPI]
    C --> D[spawn mpsl_task]
    D --> E[create hardware RNG]
    E --> F[grab 8-byte entropy seed<br/>for SoftwareRng]
    F --> G[build SDC<br/>SDC takes the RNG]
    G --> H[configure SX1262 SPI pins<br/>TWISPI1 @ 16 MHz]
    H --> I[read keys from flash]
    I -->|found| J[signal LORAWAN_KEYS]
    I -->|absent| K[log: wait for BLE provisioning]
    J --> L[spawn lorawan_task]
    K --> L
    L --> M[run_ble in main context<br/>advertise as 'flow_ctrl']
```

## Init steps in detail

| Step | Code | Peripherals / resources | Notes |
| --- | --- | --- | --- |
| Peripheral access | `embassy_nrf::init` | all | Default config. |
| MPSL | `MultiprotocolServiceLayer::new` | `RTC0`, `TIMER0`, `TEMP`, `PPI_CH19/30/31` | Low-freq clock = internal RC oscillator (`MPSL_CLOCK_LF_SRC_RC`). Spawned as `mpsl_task`. |
| RNG seed | `rng.blocking_fill_bytes` | `RNG` | 8 bytes read into a `u64` **before** SDC claims the RNG. Seeds `SoftwareRng` for LoRaWAN. |
| SDC | `build_sdc` | `PPI_CH17..29`, `Mem<4720>` | BLE link-layer controller. Supports advertising + one peripheral connection. |
| SX1262 SPI | `spim::Spim::new` | `TWISPI1`, `P1_11/12/13` | 16 MHz. Control pins: NSS `P1_10`, reset `P1_06`, DIO1 `P1_15`, busy `P1_14`, RF switch RX `P1_05` / TX `P1_07`. |
| Flash load | `flash::read_keys` | `NVMC` | See [flash-storage.md](flash-storage.md). Signals keys immediately if present. |
| LoRaWAN task | `spawner.spawn(lorawan_task)` | SPI + control pins + `SoftwareRng` | See [uplink-downlink.md](uplink-downlink.md). |
| BLE | `ble::run_ble` | SDC controller, `NVMC` | Runs inline in `main` (not spawned) because it owns non-`'static` SDC resources. See [provisioning.md](provisioning.md). |

## The two subsystems and how they meet

After startup there are effectively two concurrent flows:

- **BLE** (`run_ble`) — advertises, accepts a connection, receives keys.
- **LoRaWAN** (`lorawan_task`) — waits for keys, joins, uplinks on a timer.

They never call each other directly. They communicate through a single
`Signal<CriticalSectionRawMutex, LorawanKeys>` named `LORAWAN_KEYS`
(in [`src/shared.rs`](../firmware/src/shared.rs)):

```mermaid
flowchart LR
    subgraph producers
      A[boot: flash::read_keys]
      B[BLE: keys written]
    end
    A -->|signal| S[(LORAWAN_KEYS)]
    B -->|signal| S
    S -->|wait| C[lorawan_task]
```

At boot, if flash already holds keys, `LORAWAN_KEYS` is signalled right away and
the LoRaWAN task can join without any BLE interaction. Otherwise the task blocks
on `LORAWAN_KEYS.wait()` until provisioning happens.

## Related docs
- [provisioning.md](provisioning.md) — how keys get onto the device.
- [flash-storage.md](flash-storage.md) — what persists across reboots.
- [uplink-downlink.md](uplink-downlink.md) — the join + uplink/downlink loop.
