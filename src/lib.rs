//! # cashcode
//!
//! A Rust driver for CashCode bill validators using the **CCNET** serial
//! protocol (RS-232, 9600 baud, 8N1).
//!
//! ## Quick start
//!
//! ```no_run
//! use cashcode::{CashcodeDevice, DeviceState};
//! use std::time::Duration;
//!
//! fn main() -> cashcode::Result<()> {
//!     let mut dev = CashcodeDevice::open("/dev/ttyUSB0", None, None)?;
//!
//!     dev.initialize()?;
//!
//!     loop {
//!         match dev.poll()? {
//!             DeviceState::EscrowPosition { bill_type } => {
//!                 let table = dev.bill_table()?;
//!
//!                 if let Some(entry) = table.get(bill_type) {
//!                     println!("Accepted: {entry}");
//!                 }
//!
//!                 dev.stack()?;
//!             }
//!             DeviceState::BillStacked { .. } => println!("Bill stacked."),
//!             _ => {}
//!         }
//!
//!         std::thread::sleep(Duration::from_millis(200));
//!     }
//! }
//! ```
//!
//! ## Protocol overview
//!
//! Every message uses the frame layout:
//!
//! ```text
//! ┌──────┬─────┬─────┬────────────┬───────┬───────┐
//! │ SYNC │ ADR │ LNG │  DATA ...  │ CRC_L │ CRC_H │
//! │ 0x02 │ 1 B │ 1 B │ 0–250 B    │ 1 B   │ 1 B   │
//! └──────┴─────┴─────┴────────────┴───────┴───────┘
//! ```
//!
//! - **LNG** is the total frame length (all 6+ bytes included).
//! - **CRC** is CRC-KERMIT (reflected poly `0x8408`, init `0x0000`),
//!   transmitted LSB-first, covering every byte from `SYNC` through the last
//!   `DATA` byte.

pub mod bill_table;
pub mod command;
pub mod device;
pub mod error;
pub mod frame;
pub mod status;

// Flatten the most-used types to the crate root for ergonomic imports.
pub use bill_table::{BillEntry, BillTable};
pub use command::{BillMask, Command, SecurityLevel};
pub use device::{CashcodeDevice, Identification, POLL_INTERVAL};
pub use error::{Error, Result};
pub use frame::{Frame, crc16};
pub use status::{DeviceState, FailureCode, RejectReason};
