use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Serial port error: {0}")]
    Serial(#[from] serialport::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid frame: {0}")]
    InvalidFrame(&'static str),

    #[error("CRC mismatch (expected {expected:#06x}, got {actual:#06x})")]
    CrcMismatch { expected: u16, actual: u16 },

    #[error("Device returned NAK")]
    Nak,

    #[error("Timeout waiting for device response")]
    Timeout,

    #[error("Unknown status byte: {0:#04x}")]
    UnknownStatus(u8),

    #[error("Device not ready: current state is {0:?}")]
    NotReady(String),
}

pub type Result<T> = std::result::Result<T, Error>;
