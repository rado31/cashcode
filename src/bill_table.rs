use crate::error::{Error, Result};

/// The number of bill slots in the validator's bill table.
pub const BILL_TABLE_SIZE: usize = 24;

/// Raw size of one bill table entry as transmitted over the wire.
const ENTRY_BYTES: usize = 5;

/// Expected total response size for the `GET_BILL_TABLE` command.
pub const BILL_TABLE_RESPONSE_LEN: usize = BILL_TABLE_SIZE * ENTRY_BYTES;

/// A single entry in the bill denomination table.
///
/// Each bill type known to the validator is described by a face-value
/// denomination and a three-character ISO-4217 currency code.
///
/// An all-zero entry (denomination = 0, country = `\0\0\0`) indicates an
/// unused slot in the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillEntry {
    /// Face value of the bill (e.g., `5` for a 5 USD note).
    pub denomination: u16,
    /// ISO 4217 currency/country code as raw ASCII bytes (e.g., `b"USD"`).
    pub country_code: [u8; 3],
}

impl BillEntry {
    /// Returns `true` if this slot is unused (denomination is zero).
    pub fn is_empty(&self) -> bool {
        self.denomination == 0
    }

    /// Returns the country code as a `&str`, or `"???"` if not valid UTF-8.
    pub fn country_str(&self) -> &str {
        std::str::from_utf8(&self.country_code).unwrap_or("???")
    }

    fn from_bytes(raw: &[u8; ENTRY_BYTES]) -> Self {
        // Bytes 0–1: denomination (little-endian u16)
        // Bytes 2–4: ISO country/currency code (ASCII)
        let denomination = u16::from_le_bytes([raw[0], raw[1]]);
        let country_code = [raw[2], raw[3], raw[4]];
        Self {
            denomination,
            country_code,
        }
    }
}

impl std::fmt::Display for BillEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            write!(f, "(empty)")
        } else {
            write!(f, "{} {}", self.denomination, self.country_str())
        }
    }
}

/// The complete bill table returned by the `GET_BILL_TABLE` command.
///
/// Contains up to 24 bill entries; unused entries have a denomination of 0.
#[derive(Debug, Clone)]
pub struct BillTable {
    entries: [BillEntry; BILL_TABLE_SIZE],
}

impl BillTable {
    /// Parse a bill table from the 120-byte `GET_BILL_TABLE` response payload.
    pub fn from_response_data(data: &[u8]) -> Result<Self> {
        if data.len() < BILL_TABLE_RESPONSE_LEN {
            return Err(Error::InvalidFrame("GET_BILL_TABLE response too short"));
        }

        // SAFETY: we're constructing via Default + repeated assignment, which is safe.
        let empty = BillEntry {
            denomination: 0,
            country_code: [0; 3],
        };
        let mut entries = std::array::from_fn(|_| empty.clone());

        for i in 0..BILL_TABLE_SIZE {
            let offset = i * ENTRY_BYTES;
            let raw: &[u8; ENTRY_BYTES] = data[offset..offset + ENTRY_BYTES]
                .try_into()
                .expect("slice length is exact");
            entries[i] = BillEntry::from_bytes(raw);
        }

        Ok(Self { entries })
    }

    /// Return a reference to an entry by its 0-based index.
    ///
    /// Returns `None` when `index >= 24`.
    pub fn get(&self, index: u8) -> Option<&BillEntry> {
        self.entries.get(index as usize)
    }

    /// Iterate over all 24 entries (including empty slots).
    pub fn iter(&self) -> impl Iterator<Item = (u8, &BillEntry)> {
        self.entries.iter().enumerate().map(|(i, e)| (i as u8, e))
    }

    /// Iterate over non-empty entries only.
    pub fn active_entries(&self) -> impl Iterator<Item = (u8, &BillEntry)> {
        self.iter().filter(|(_, e)| !e.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry_bytes(denomination: u16, code: &[u8; 3]) -> [u8; 5] {
        let d = denomination.to_le_bytes();
        [d[0], d[1], code[0], code[1], code[2]]
    }

    #[test]
    fn parse_single_entry() {
        let raw = make_entry_bytes(5, b"USD");
        let entry = BillEntry::from_bytes(&raw);
        assert_eq!(entry.denomination, 5);
        assert_eq!(&entry.country_code, b"USD");
        assert_eq!(entry.country_str(), "USD");
    }

    #[test]
    fn parse_full_table() {
        let mut data = vec![0u8; BILL_TABLE_RESPONSE_LEN];
        // Put a $5 USD entry in slot 0
        let entry = make_entry_bytes(5, b"USD");
        data[..5].copy_from_slice(&entry);

        let table = BillTable::from_response_data(&data).unwrap();
        assert_eq!(table.get(0).unwrap().denomination, 5);
        assert!(table.get(1).unwrap().is_empty());
        assert_eq!(table.active_entries().count(), 1);
    }

    #[test]
    fn short_response_is_error() {
        let data = vec![0u8; 10];
        assert!(matches!(
            BillTable::from_response_data(&data),
            Err(Error::InvalidFrame(_))
        ));
    }
}
