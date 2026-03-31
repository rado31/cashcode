use std::time::Duration;

use log::{debug, warn};
use serialport::SerialPort;

use crate::{
    bill_table::BillTable,
    command::{BillMask, Command, SecurityLevel},
    error::{Error, Result},
    frame::{ACK, Frame, SYNC},
    status::DeviceState,
};

/// Default baud rate for CCNET bill validators.
pub const DEFAULT_BAUD_RATE: u32 = 9600;

/// Default read/write timeout.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2);

/// Recommended polling interval (send `POLL` at most this often).
pub const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Minimum bus silence between a response and the next command.
pub const BUS_SILENCE: Duration = Duration::from_millis(20);

/// Device identification returned by the `IDENTIFICATION` command.
#[derive(Debug, Clone)]
pub struct Identification {
    /// 7-byte part number string.
    pub part_number: String,
    /// 12-byte serial number string.
    pub serial_number: String,
    /// 7-byte asset number string.
    pub asset_number: String,
}

/// A handle to a CCNET bill validator connected over a serial port.
///
/// # Example
///
/// ```no_run
/// use cashcode::{CashcodeDevice, DeviceState};
/// use std::time::Duration;
///
/// let mut dev = CashcodeDevice::open("/dev/ttyUSB0", None, None)?;
///
/// dev.initialize()?;
///
/// loop {
///     match dev.poll()? {
///         DeviceState::EscrowPosition { bill_type } => {
///             println!("Bill type {} in escrow — stacking", bill_type);
///             dev.stack()?;
///         }
///         DeviceState::BillStacked { bill_type } => {
///             println!("Bill type {} stacked", bill_type);
///         }
///         _ => {}
///     }
///
///     std::thread::sleep(Duration::from_millis(200));
/// }
///
/// # Ok::<(), cashcode::Error>(())
/// ```
pub struct CashcodeDevice {
    port: Box<dyn SerialPort>,
    bill_table: Option<BillTable>,
}

impl CashcodeDevice {
    /// Open a serial port connected to a CCNET bill validator.
    ///
    /// - `path`: OS-specific port name (e.g. `/dev/ttyUSB0`, `COM3`).
    /// - `baud_rate`: defaults to [`DEFAULT_BAUD_RATE`] (9600).
    /// - `timeout`: per-read timeout, defaults to [`DEFAULT_TIMEOUT`] (2 s).
    pub fn open(path: &str, baud_rate: Option<u32>, timeout: Option<Duration>) -> Result<Self> {
        let port = serialport::new(path, baud_rate.unwrap_or(DEFAULT_BAUD_RATE))
            .data_bits(serialport::DataBits::Eight)
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .timeout(timeout.unwrap_or(DEFAULT_TIMEOUT))
            .open()?;

        Ok(Self {
            port,
            bill_table: None,
        })
    }

    // ── Low-level transport ───────────────────────────────────────────────

    /// Transmit a command frame and return the device's response frame.
    fn send(&mut self, command: Command) -> Result<Frame> {
        let frame = Frame::new(command.to_data());
        let bytes = frame.encode();

        debug!("TX → {:02X?}", bytes);

        self.port.write_all(&bytes)?;
        self.port.flush()?;

        // Minimum bus silence before reading the response.
        std::thread::sleep(BUS_SILENCE);

        self.read_frame()
    }

    /// Read exactly one complete frame from the serial port.
    ///
    /// Discards any stray bytes before the SYNC byte to tolerate noise on
    /// the line.
    fn read_frame(&mut self) -> Result<Frame> {
        // Scan for the SYNC byte, discarding garbage.
        let sync = loop {
            let mut buf = [0u8; 1];

            match self.port.read_exact(&mut buf) {
                Ok(()) => {
                    if buf[0] == SYNC {
                        break buf[0];
                    }

                    warn!("discarding non-SYNC byte: {:#04x}", buf[0]);
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    return Err(Error::Timeout);
                }
                Err(e) => return Err(Error::Io(e)),
            }
        };

        // Read ADR + LNG (2 bytes).
        let mut header = [0u8; 2];

        self.port.read_exact(&mut header)?;

        let addr = header[0];
        let lng = header[1] as usize;

        if lng < 6 {
            return Err(Error::InvalidFrame("LNG < 6 in received frame"));
        }

        // Read the remainder: DATA + CRC (lng - 3 bytes already read: SYNC+ADR+LNG).
        let remaining = lng - 3;
        let mut rest = vec![0u8; remaining];

        self.port.read_exact(&mut rest)?;

        // Reconstruct the complete raw frame for Frame::decode (which re-checks CRC).
        let mut raw = Vec::with_capacity(lng);

        raw.push(sync);
        raw.push(addr);
        raw.push(lng as u8);
        raw.extend_from_slice(&rest);

        debug!("RX ← {:02X?}", raw);

        Frame::decode(&raw)
    }

    // ── Command helpers ───────────────────────────────────────────────────

    /// Send a command and verify the device responded with ACK (0x00).
    fn ack_command(&mut self, command: Command) -> Result<()> {
        let response = self.send(command)?;

        match response.data.first().copied() {
            Some(ACK) => Ok(()),
            Some(0xFF) => Err(Error::Nak),
            _ => Ok(()), // some firmware versions respond with more data; treat as OK
        }
    }

    // ── Public API ────────────────────────────────────────────────────────

    /// Send a `RESET` command.
    ///
    /// Any bill currently in the validator will be returned to the customer.
    /// The device transitions through `Initializing` before reaching `Idling`.
    pub fn reset(&mut self) -> Result<()> {
        debug!("sending RESET");
        self.ack_command(Command::Reset)
    }

    /// Send a `POLL` command and return the current [`DeviceState`].
    ///
    /// After reading the device's response an ACK frame is sent back so the
    /// device advances its internal state machine.  Without this ACK the
    /// device stays frozen in the current state indefinitely.
    pub fn poll(&mut self) -> Result<DeviceState> {
        let response = self.send(Command::Poll)?;

        // Send ACK back to the device — required by CCNET after every POLL.
        let ack = Frame::new(vec![ACK]).encode();

        debug!("TX ACK → {:02X?}", ack);

        let _ = self.port.write_all(&ack);
        let _ = self.port.flush();

        DeviceState::from_poll_data(&response.data)
    }

    /// Enable bill types described by `mask`.
    ///
    /// Calling this with [`BillMask::ALL`] accepts every denomination in
    /// the bill table.
    pub fn enable_bill_types(&mut self, mask: BillMask) -> Result<()> {
        debug!("enabling bill types: {:?}", mask.0);
        self.ack_command(Command::EnableBillTypes(mask))
    }

    /// Disable all bill types (the validator will not accept any bills).
    pub fn disable(&mut self) -> Result<()> {
        self.ack_command(Command::EnableBillTypes(BillMask::NONE))
    }

    /// Set the validation security level.
    ///
    /// The device requires ~2 seconds to apply the new settings.
    pub fn set_security(&mut self, level: SecurityLevel) -> Result<()> {
        debug!("setting security level: {:?}", level);

        self.ack_command(Command::SetSecurity(level))?;

        std::thread::sleep(Duration::from_secs(2));

        Ok(())
    }

    /// Move the escrowed bill into the cashbox stacker.
    ///
    /// Only valid when the device is in [`DeviceState::EscrowPosition`] or
    /// [`DeviceState::Holding`].
    pub fn stack(&mut self) -> Result<()> {
        debug!("sending STACK");
        self.ack_command(Command::Stack)
    }

    /// Return the escrowed bill to the customer.
    ///
    /// Only valid when the device is in [`DeviceState::EscrowPosition`] or
    /// [`DeviceState::Holding`].
    pub fn return_bill(&mut self) -> Result<()> {
        debug!("sending RETURN");
        self.ack_command(Command::Return)
    }

    /// Extend the escrow hold timer by ~10 seconds.
    ///
    /// Must be followed by [`stack`](Self::stack) or
    /// [`return_bill`](Self::return_bill) before the timer expires.
    pub fn hold(&mut self) -> Result<()> {
        debug!("sending HOLD");
        self.ack_command(Command::Hold)
    }

    /// Fetch and cache the device's bill denomination table.
    pub fn get_bill_table(&mut self) -> Result<BillTable> {
        let response = self.send(Command::GetBillTable)?;
        let table = BillTable::from_response_data(&response.data)?;

        self.bill_table = Some(table.clone());

        Ok(table)
    }

    /// Return the cached [`BillTable`], fetching it from the device if not
    /// yet loaded.
    pub fn bill_table(&mut self) -> Result<&BillTable> {
        if self.bill_table.is_none() {
            self.get_bill_table()?;
        }

        Ok(self.bill_table.as_ref().unwrap())
    }

    /// Query device identification (part number, serial number, asset number).
    pub fn identify(&mut self) -> Result<Identification> {
        let response = self.send(Command::Identification)?;
        let data = &response.data;

        if data.len() < 26 {
            return Err(Error::InvalidFrame("IDENTIFICATION response too short"));
        }

        Ok(Identification {
            part_number: String::from_utf8_lossy(&data[0..7])
                .trim_end_matches('\0')
                .to_string(),
            serial_number: String::from_utf8_lossy(&data[7..19])
                .trim_end_matches('\0')
                .to_string(),
            asset_number: String::from_utf8_lossy(&data[19..26])
                .trim_end_matches('\0')
                .to_string(),
        })
    }

    /// Perform the full device initialisation sequence:
    ///
    /// 1. Send `RESET`.
    /// 2. Poll until the device reaches `Initializing` or `Idling`.
    /// 3. Fetch the bill table.
    /// 4. Enable all bill types.
    ///
    /// `max_wait` caps the total time spent polling (default: 30 s).
    pub fn initialize(&mut self) -> Result<()> {
        self.reset()?;
        self.wait_for_ready(Duration::from_secs(30))?;
        self.get_bill_table()?;
        self.enable_bill_types(BillMask::ALL)?;

        Ok(())
    }

    /// Poll the device until it reports [`DeviceState::Idling`] or until
    /// `timeout` elapses.
    pub fn wait_for_ready(&mut self, timeout: Duration) -> Result<()> {
        let deadline = std::time::Instant::now() + timeout;

        loop {
            if std::time::Instant::now() >= deadline {
                return Err(Error::Timeout);
            }

            let state = self.poll()?;

            debug!("wait_for_ready: state = {:?}", state);

            match state {
                // These states mean the device is ready for the next command.
                DeviceState::Idling | DeviceState::UnitDisabled => return Ok(()),
                // Still booting — send SET_SECURITY to advance initialisation.
                DeviceState::Initializing => {
                    debug!("device initializing — sending SET_SECURITY");
                    let _ = self.set_security(SecurityLevel::Low);
                }
                DeviceState::PowerUp
                | DeviceState::PowerUpBillInValidator
                | DeviceState::PowerUpBillInStacker
                | DeviceState::Busy => {}
                other if other.is_error() => {
                    return Err(Error::NotReady(format!("{:?}", other)));
                }
                _ => {}
            }

            std::thread::sleep(POLL_INTERVAL);
        }
    }
}
