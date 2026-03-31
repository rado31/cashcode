use std::time::Duration;

use cashcode::{BillMask, CashcodeDevice, DeviceState, SecurityLevel};

fn main() -> cashcode::Result<()> {
    env_logger::init();

    let port = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/ttyUSB0".into());

    let mut dev = CashcodeDevice::open(&port, None, None)?;

    dev.reset()?;
    dev.wait_for_ready(Duration::from_secs(30))?;

    let table = dev.get_bill_table()?;

    println!("\nBill table:");

    for (idx, entry) in table.active_entries() {
        println!("  [{idx:2}] {entry}");
    }

    dev.enable_bill_types(BillMask::ALL)?;

    println!("\nReady. Insert a bill …\n");

    loop {
        let state = dev.poll()?;

        match &state {
            DeviceState::Idling => {}
            DeviceState::Initializing => dev.set_security(SecurityLevel::Low)?,
            DeviceState::Accepting => println!("Accepting …"),
            DeviceState::Stacking => println!("Stacking …"),
            DeviceState::Returning => println!("Returning …"),
            DeviceState::Holding => println!("Holding …"),
            DeviceState::EscrowPosition { bill_type } => match table.get(*bill_type) {
                Some(entry) if !entry.is_empty() => {
                    println!("Escrow: {entry} (type {bill_type}) — stacking");
                    dev.stack()?;
                }
                _ => {
                    println!("Escrow: unrecognised type {bill_type} — returning");
                    dev.return_bill()?;
                }
            },
            DeviceState::BillStacked { bill_type } => {
                let label = table
                    .get(*bill_type)
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| format!("type {bill_type}"));

                println!("✓ Bill stacked: {label}");
            }
            DeviceState::BillReturned { bill_type } => {
                println!("Bill returned (type {bill_type})");
            }
            DeviceState::UnitDisabled => {
                println!("Unit disabled — re-enabling …");
                dev.enable_bill_types(BillMask::ALL)?;
                println!("Re-enabled. Ready for next bill.");
            }
            DeviceState::Rejecting(reason) => {
                println!("Bill rejected: {reason:?}");
            }
            DeviceState::CassetteFull => {
                eprintln!("ERROR: Cassette is full — please empty it.");
            }
            DeviceState::CassetteOutOfPosition => {
                eprintln!("ERROR: Cassette is out of position.");
            }
            DeviceState::ValidatorJammed => {
                eprintln!("ERROR: Validator jammed.");
            }
            DeviceState::CassetteJammed => {
                eprintln!("ERROR: Cassette jammed.");
            }
            DeviceState::Cheated => {
                eprintln!("ERROR: Tamper/cheat detected.");
            }
            DeviceState::Failure(code) => {
                eprintln!("ERROR: Hardware failure — {code:?}");
            }
            other => {
                println!("State: {other:?}");
            }
        }

        std::thread::sleep(cashcode::POLL_INTERVAL);
    }
}
