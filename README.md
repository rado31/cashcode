# cashcode

A Rust driver for **CashCode bill validators** using the [CCNET](https://en.wikipedia.org/wiki/CCNET) serial protocol.

[![CI](https://github.com/rado31/cashcode/actions/workflows/ci.yml/badge.svg)](https://github.com/rado31/cashcode/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/cashcode.svg)](https://crates.io/crates/cashcode)
[![docs.rs](https://docs.rs/cashcode/badge.svg)](https://docs.rs/cashcode)

---

## Features

- Full CCNET command set: `RESET`, `POLL`, `ENABLE_BILL_TYPES`, `STACK`, `RETURN`, `HOLD`, `IDENTIFICATION`, `GET_BILL_TABLE`
- Typed `DeviceState` enum covering all device states, bill events, rejection reasons, and hardware failures
- CRC-KERMIT validation on every frame (reflected poly `0x8408`, init `0x0000`)
- `BillTable` parsing — denomination and ISO currency code for up to 24 bill types
- `BillMask` type for fine-grained per-denomination enable/disable control
- Synchronous API — no async runtime required; wrap in a thread when needed
- Cross-platform: Linux, Windows (macOS for development)

## Installation

```toml
[dependencies]
cashcode = "0.1"
```

## Quick start

```rust
use cashcode::{BillMask, CashcodeDevice, DeviceState};
use std::time::Duration;

fn main() -> cashcode::Result<()> {
    let mut dev = CashcodeDevice::open("/dev/ttyUSB0", None, None)?;

    // Reset → wait for idle → fetch bill table → enable all bills
    dev.initialize()?;

    // Print the bill denomination table
    let table = dev.bill_table()?;
    for (idx, entry) in table.active_entries() {
        println!("[{idx}] {entry}");
    }

    // Poll loop
    loop {
        match dev.poll()? {
            DeviceState::EscrowPosition { bill_type } => {
                println!("Bill in escrow: {}", table.get(bill_type).unwrap());
                dev.stack()?; // accept
            }
            DeviceState::BillStacked { bill_type } => {
                println!("Stacked bill type {bill_type}");
            }
            DeviceState::Rejecting(reason) => {
                eprintln!("Rejected: {reason:?}");
            }
            _ => {}
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}
```

## Serial port

| Parameter | Value |
|-----------|-------|
| Baud rate | 9600 (default) or 19200 |
| Data bits | 8 |
| Parity    | None |
| Stop bits | 1 |

Port name examples:

| OS      | Example |
|---------|---------|
| Linux   | `/dev/ttyUSB0`, `/dev/ttyS0` |
| Windows | `COM3` |
| macOS   | `/dev/cu.usbserial-XXXX` |

## API overview

```rust
// Open
let mut dev = CashcodeDevice::open("/dev/ttyUSB0", None, None)?;

// Initialisation
dev.reset()?;
dev.wait_for_ready(Duration::from_secs(30))?;
dev.get_bill_table()?;
dev.enable_bill_types(BillMask::ALL)?;

// — or all at once —
dev.initialize()?;

// Fine-grained control: enable only bill type 0 and 2
let mut mask = BillMask::NONE;
mask.0[0] = 0b0000_0101;
dev.enable_bill_types(mask)?;

// Escrow decisions
dev.stack()?;       // accept the bill
dev.return_bill()?; // return to customer
dev.hold()?;        // extend hold timer (~10 s)

// Device info
let id = dev.identify()?;
println!("{} / {}", id.part_number, id.serial_number);
```

## Device states

```
PowerUp ──► Initializing ──► Idling ──► Accepting
                 │               ▲            │
            SET_SECURITY         │      EscrowPosition
                                 │       ┌────┴────┐
                                 │    Stack()   Return()
                                 │       │          │
                                 │   Stacking   Returning
                                 │       │          │
                                 │  BillStacked  BillReturned
                                 │       └────┬────┘
                                 │       UnitDisabled
                                 └──── enable_bill_types()
```

After every bill stacked or returned the device disables itself (`UnitDisabled`). The host must call `enable_bill_types()` to return to `Idling`.

Error states: `CassetteFull`, `CassetteOutOfPosition`, `ValidatorJammed`, `CassetteJammed`, `Cheated`, `Paused`, `Failure`.

## Logging

The crate uses the [`log`](https://crates.io/crates/log) facade. Enable debug output with any compatible backend:

```bash
RUST_LOG=debug cargo run -- /dev/ttyUSB0
```

## Platform notes

**Linux** — requires `libudev-dev`:
```bash
sudo apt-get install libudev-dev
```

**Windows** — no extra dependencies; uses the Win32 serial API.

## License

MIT
