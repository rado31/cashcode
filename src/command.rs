/// CCNET command codes.
mod codes {
    pub const RESET: u8 = 0x30;
    pub const GET_STATUS: u8 = 0x31;
    pub const SET_SECURITY: u8 = 0x32;
    pub const POLL: u8 = 0x33;
    pub const ENABLE_BILL_TYPES: u8 = 0x34;
    pub const STACK: u8 = 0x35;
    pub const RETURN: u8 = 0x36;
    pub const IDENTIFICATION: u8 = 0x37;
    pub const HOLD: u8 = 0x38;
    pub const GET_BILL_TABLE: u8 = 0x41;
    pub const GET_CRC32: u8 = 0x51;
    pub const REQUEST_STATISTICS: u8 = 0x60;
}

/// A bill acceptance bitmask for [`Command::EnableBillTypes`].
///
/// Each bit in the 6-byte array represents one bill type (bill type `N`
/// maps to bit `N % 8` of byte `N / 8`). Up to 48 bill types are
/// addressable; the bill table returned by `GET_BILL_TABLE` defines
/// up to 24 active types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BillMask(pub [u8; 6]);

impl BillMask {
    /// Enable all bill types.
    pub const ALL: Self = Self([0xFF; 6]);

    /// Disable all bill types.
    pub const NONE: Self = Self([0x00; 6]);

    /// Enable a single bill type by its 0-based index (0–47).
    pub fn single(index: u8) -> Self {
        let mut mask = [0u8; 6];
        mask[(index / 8) as usize] |= 1 << (index % 8);
        Self(mask)
    }

    /// Test whether a given bill type index is enabled.
    pub fn is_enabled(self, index: u8) -> bool {
        (self.0[(index / 8) as usize] >> (index % 8)) & 1 == 1
    }
}

impl Default for BillMask {
    fn default() -> Self {
        Self::ALL
    }
}

/// Security level for the `SET_SECURITY` command.
///
/// Higher levels apply stricter validation criteria.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SecurityLevel {
    Low = 0x00,
    Normal = 0x01,
    High = 0x02,
    VeryHigh = 0x03,
}

/// All commands that can be sent to a CCNET bill validator.
#[derive(Debug, Clone)]
pub enum Command {
    /// Restart the device; any escrowed bill is returned to the customer.
    Reset,

    /// Query current device status.
    GetStatus,

    /// Set the security level applied to all bill types.
    SetSecurity(SecurityLevel),

    /// Poll the device for state changes and bill events. Must be sent
    /// periodically (every 200 ms is typical).
    Poll,

    /// Enable or disable individual bill types. The device will only
    /// accept bill types that are set in `mask`.
    EnableBillTypes(BillMask),

    /// Move an escrowed bill into the cashbox stacker.
    Stack,

    /// Return an escrowed bill to the customer.
    Return,

    /// Request device identification (part number, serial number, asset number).
    Identification,

    /// Hold the escrowed bill for an additional ~10 s without a decision.
    /// Must be followed by `Stack` or `Return`.
    Hold,

    /// Retrieve the bill denomination table (24 entries × 5 bytes).
    GetBillTable,

    /// Request the CRC-32 of the firmware.
    GetCrc32,

    /// Request device statistics.
    RequestStatistics,
}

impl Command {
    /// Serialise the command into the DATA bytes placed inside a frame.
    ///
    /// The first byte is always the command code; additional bytes carry
    /// command-specific parameters.
    pub fn to_data(&self) -> Vec<u8> {
        match self {
            Command::Reset => vec![codes::RESET],
            Command::GetStatus => vec![codes::GET_STATUS],
            Command::SetSecurity(level) => vec![codes::SET_SECURITY, *level as u8],
            Command::Poll => vec![codes::POLL],
            Command::EnableBillTypes(mask) => {
                let mut data = vec![codes::ENABLE_BILL_TYPES];
                data.extend_from_slice(&mask.0);
                data
            }
            Command::Stack => vec![codes::STACK],
            Command::Return => vec![codes::RETURN],
            Command::Identification => vec![codes::IDENTIFICATION],
            Command::Hold => vec![codes::HOLD],
            Command::GetBillTable => vec![codes::GET_BILL_TABLE],
            Command::GetCrc32 => vec![codes::GET_CRC32],
            Command::RequestStatistics => vec![codes::REQUEST_STATISTICS],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bill_mask_single() {
        let mask = BillMask::single(0);
        assert!(mask.is_enabled(0));
        assert!(!mask.is_enabled(1));
        assert_eq!(mask.0[0], 0x01);
    }

    #[test]
    fn bill_mask_all_enabled() {
        let mask = BillMask::ALL;
        for i in 0..48u8 {
            assert!(mask.is_enabled(i));
        }
    }

    #[test]
    fn enable_bill_types_data_len() {
        let data = Command::EnableBillTypes(BillMask::ALL).to_data();
        assert_eq!(data.len(), 7); // 1 cmd byte + 6 mask bytes
        assert_eq!(data[0], 0x34);
    }
}
