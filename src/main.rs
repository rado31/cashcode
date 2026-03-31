use std::time::Duration;

use cashcode::{BillMask, CashcodeDevice, DeviceState};

fn main() -> cashcode::Result<()> {
    env_logger::init();

    // Adjust the port name for your OS:
    //   Linux  → "/dev/ttyUSB0"  or  "/dev/ttyS0"
    //   macOS  → "/dev/cu.usbserial-XXXX"
    //   Windows → "COM3"
    let port = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/ttyUSB0".into());
    println!("Opening {port} …");

    let mut dev = CashcodeDevice::open(&port, None, None)?;

    // ── Identification ────────────────────────────────────────────────────
    match dev.identify() {
        Ok(id) => {
            println!("Part number  : {}", id.part_number);
            println!("Serial number: {}", id.serial_number);
            println!("Asset number : {}", id.asset_number);
        }
        Err(e) => eprintln!("Warning: could not read identification — {e}"),
    }

    // ── Initialise ────────────────────────────────────────────────────────
    println!("Initialising …");
    dev.reset()?;
    dev.wait_for_ready(Duration::from_secs(30))?;

    let table = dev.get_bill_table()?;
    println!("\nBill table:");
    for (idx, entry) in table.active_entries() {
        println!("  [{idx:2}] {entry}");
    }

    // Enable all bill types.
    dev.enable_bill_types(BillMask::ALL)?;
    println!("\nReady. Insert a bill …\n");

    // ── Poll loop ─────────────────────────────────────────────────────────
    loop {
        let state = dev.poll()?;

        match &state {
            DeviceState::Idling => {}

            DeviceState::Accepting => println!("Accepting …"),
            DeviceState::Stacking => println!("Stacking …"),
            DeviceState::Returning => println!("Returning …"),
            DeviceState::Holding => println!("Holding …"),

            DeviceState::EscrowPosition { bill_type } => {
                if let Some(entry) = table.get(*bill_type) {
                    println!("Escrow: {entry} (type {bill_type}) — stacking");
                } else {
                    println!("Escrow: unknown bill type {bill_type} — returning");
                    dev.return_bill()?;
                    continue;
                }
                dev.stack()?;
            }

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
