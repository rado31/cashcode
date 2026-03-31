/// Synchronization byte that starts every CCNET frame.
pub const SYNC: u8 = 0x02;

/// Default address for a bill validator peripheral.
pub const DEFAULT_ADDR: u8 = 0x03;

/// ACK response byte sent by the device to acknowledge a command.
pub const ACK: u8 = 0x00;

/// NAK response byte sent by the device to reject a command.
pub const NAK: u8 = 0xFF;

use crate::error::{Error, Result};

/// A CCNET protocol frame.
///
/// Wire format: `[SYNC][ADR][LNG][DATA...][CRC_L][CRC_H]`
///
/// - `SYNC` is always `0x02`
/// - `LNG` is the total frame length (SYNC through CRC inclusive)
/// - `DATA` includes the command byte as its first byte for outgoing frames
/// - CRC-CCITT (poly `0x1021`, init `0xFFFF`) covers SYNC through end of DATA
#[derive(Debug, Clone)]
pub struct Frame {
    pub addr: u8,
    /// The DATA payload: for command frames, `data[0]` is the command byte.
    pub data: Vec<u8>,
}

impl Frame {
    /// Construct a frame for the default bill validator address.
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            addr: DEFAULT_ADDR,
            data,
        }
    }

    /// Encode this frame into bytes ready for transmission.
    ///
    /// # Panics
    /// Panics if the total frame length would exceed 255 bytes.
    pub fn encode(&self) -> Vec<u8> {
        // LNG = SYNC(1) + ADR(1) + LNG(1) + DATA(n) + CRC(2) = n + 5
        let lng = self.data.len() + 5;
        assert!(lng <= 255, "frame too large");

        let mut bytes = Vec::with_capacity(lng);
        bytes.push(SYNC);
        bytes.push(self.addr);
        bytes.push(lng as u8);
        bytes.extend_from_slice(&self.data);

        let crc = crc16(&bytes);
        bytes.push((crc & 0xFF) as u8); // LSB first
        bytes.push((crc >> 8) as u8);

        bytes
    }

    /// Decode a frame from a raw byte slice.
    ///
    /// The slice must contain at least `LNG` bytes starting at the SYNC byte.
    pub fn decode(raw: &[u8]) -> Result<Self> {
        if raw.len() < 6 {
            return Err(Error::InvalidFrame("frame shorter than minimum 6 bytes"));
        }
        if raw[0] != SYNC {
            return Err(Error::InvalidFrame("missing SYNC byte (0x02)"));
        }

        let lng = raw[2] as usize;
        if lng < 6 {
            return Err(Error::InvalidFrame("LNG field is below minimum (6)"));
        }
        if raw.len() < lng {
            return Err(Error::InvalidFrame(
                "buffer shorter than LNG field indicates",
            ));
        }

        let payload = &raw[..lng];
        let crc_calculated = crc16(&payload[..lng - 2]);
        let crc_received = (payload[lng - 2] as u16) | ((payload[lng - 1] as u16) << 8);

        if crc_calculated != crc_received {
            return Err(Error::CrcMismatch {
                expected: crc_calculated,
                actual: crc_received,
            });
        }

        Ok(Frame {
            addr: raw[1],
            data: payload[3..lng - 2].to_vec(),
        })
    }
}

/// CRC-KERMIT (polynomial `0x1021`, initial value `0x0000`, reflected).
///
/// Covers all bytes from SYNC through the last DATA byte (not including
/// the two CRC bytes themselves). The result is transmitted LSB-first.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0x0000;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0x8408;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let original = Frame::new(vec![0x33]); // POLL command
        let encoded = original.encode();
        let decoded = Frame::decode(&encoded).expect("decode failed");
        assert_eq!(decoded.addr, DEFAULT_ADDR);
        assert_eq!(decoded.data, vec![0x33]);
    }

    #[test]
    fn encode_sets_correct_lng() {
        // POLL: 1 data byte → LNG = 1 + 5 = 6
        let frame = Frame::new(vec![0x33]);
        let encoded = frame.encode();
        assert_eq!(encoded[2], 6);
    }

    #[test]
    fn decode_rejects_bad_crc() {
        let mut encoded = Frame::new(vec![0x33]).encode();
        let last = encoded.len() - 1;
        encoded[last] ^= 0xFF; // corrupt the CRC high byte
        assert!(matches!(
            Frame::decode(&encoded),
            Err(Error::CrcMismatch { .. })
        ));
    }

    #[test]
    fn decode_rejects_bad_sync() {
        let mut encoded = Frame::new(vec![0x33]).encode();
        encoded[0] = 0x00;
        assert!(matches!(
            Frame::decode(&encoded),
            Err(Error::InvalidFrame(_))
        ));
    }

    #[test]
    fn crc16_known_value() {
        // CRC-KERMIT over empty slice with init 0x0000 should give 0x0000
        assert_eq!(crc16(&[]), 0x0000);
        // IDENTIFICATION frame [02 03 06 37] → CRC = 0xC7FE (transmitted FE C7)
        assert_eq!(crc16(&[0x02, 0x03, 0x06, 0x37]), 0xC7FE);
    }
}
