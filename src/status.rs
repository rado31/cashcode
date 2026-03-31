use crate::error::{Error, Result};

/// The reason a bill was rejected, carried in a `POLL` response when the
/// device state is [`DeviceState::Rejecting`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectReason {
    Insertion,
    Magnetic,
    RemainingBillInHead,
    Multiplying,
    Conveying,
    Identification1,
    Verification,
    Optic,
    Inhibit,
    Capacity,
    Operation,
    Length,
    Uv,
    Barcode,
    /// An unrecognised rejection sub-code.
    Unknown(u8),
}

impl RejectReason {
    fn from_byte(b: u8) -> Self {
        match b {
            0x60 => Self::Insertion,
            0x61 => Self::Magnetic,
            0x62 => Self::RemainingBillInHead,
            0x63 => Self::Multiplying,
            0x64 => Self::Conveying,
            0x65 => Self::Identification1,
            0x66 => Self::Verification,
            0x67 => Self::Optic,
            0x68 => Self::Inhibit,
            0x69 => Self::Capacity,
            0x6A => Self::Operation,
            0x6C => Self::Length,
            0x6D => Self::Uv,
            0x92 => Self::Barcode,
            other => Self::Unknown(other),
        }
    }
}

/// Hardware failure sub-codes, carried when the device state is
/// [`DeviceState::Failure`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureCode {
    StackMotorFailure,
    TransportMotorSpeed,
    TransportMotorFailure,
    AligningMotorFailure,
    InitialCassetteStatus,
    OpticCanal,
    MagneticCanal,
    CapacitanceCanal,
    /// An unrecognised failure sub-code.
    Unknown(u8),
}

impl FailureCode {
    fn from_byte(b: u8) -> Self {
        match b {
            0x41 => Self::StackMotorFailure,
            0x42 => Self::TransportMotorSpeed,
            0x43 => Self::TransportMotorFailure,
            0x44 => Self::AligningMotorFailure,
            0x45 => Self::InitialCassetteStatus,
            0x46 => Self::OpticCanal,
            0x47 => Self::MagneticCanal,
            0x48 => Self::CapacitanceCanal,
            other => Self::Unknown(other),
        }
    }
}

/// The complete set of states a CCNET validator can report via the `POLL`
/// response.
///
/// # Bill lifecycle
///
/// ```text
/// Idling → Accepting → EscrowPosition(bill_type)
///                             ↓               ↓
///                      (send Stack)    (send Return)
///                             ↓               ↓
///                        Stacking        Returning
///                             ↓               ↓
///                    BillStacked(t)    BillReturned(t)
///                             ↓               ↓
///                           Idling          Idling
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceState {
    // ── Power-up states ──────────────────────────────────────────────────
    /// Device has just powered on with no bill present.
    PowerUp,
    /// Device has powered on with a bill already inside the validator head.
    PowerUpBillInValidator,
    /// Device has powered on with a bill already sitting in the stacker.
    PowerUpBillInStacker,

    // ── Normal operational states ─────────────────────────────────────────
    /// Running self-tests and initialising firmware. Not yet ready.
    Initializing,
    /// Idle and ready to accept bills.
    Idling,
    /// A bill is being mechanically transported through the validator.
    Accepting,
    /// An accepted bill is being moved from the validator head to the stacker.
    Stacking,
    /// An escrowed bill is being returned to the customer.
    Returning,
    /// The validator has been disabled (e.g., no bill types enabled).
    UnitDisabled,
    /// A bill is being held in the escrow position after a `HOLD` command.
    Holding,
    /// The device is busy processing a previous command.
    Busy,
    /// A bill was rejected; the sub-code explains why.
    Rejecting(RejectReason),

    // ── Hardware / cassette errors ────────────────────────────────────────
    /// The cashbox stacker is full.
    CassetteFull,
    /// The cashbox is not properly seated in the validator.
    CassetteOutOfPosition,
    /// A bill is jammed inside the validator head.
    ValidatorJammed,
    /// A bill is jammed inside the cashbox transport.
    CassetteJammed,
    /// A tamper / cheat attempt was detected.
    Cheated,
    /// Operation is temporarily paused.
    Paused,
    /// A hardware component has failed; the sub-code identifies which one.
    Failure(FailureCode),

    // ── Bill events ───────────────────────────────────────────────────────
    /// A bill has been validated and is now in the escrow position.
    ///
    /// The host **must** respond with [`crate::command::Command::Stack`] or
    /// [`crate::command::Command::Return`] (or
    /// [`crate::command::Command::Hold`]) before the hold timer expires (~10 s).
    EscrowPosition {
        /// 0-based index into the bill table.
        bill_type: u8,
    },
    /// A bill has been successfully moved to the stacker.
    BillStacked {
        /// 0-based index into the bill table.
        bill_type: u8,
    },
    /// A bill has been returned to the customer.
    BillReturned {
        /// 0-based index into the bill table.
        bill_type: u8,
    },
}

impl DeviceState {
    /// Parse a `DeviceState` from the raw DATA bytes of a `POLL` response.
    ///
    /// The first byte of `data` is the status byte; a second byte, when
    /// present, carries sub-codes for `Rejecting` and `Failure`.
    pub fn from_poll_data(data: &[u8]) -> Result<Self> {
        if data.is_empty() {
            return Err(Error::InvalidFrame("empty POLL response"));
        }

        let status = data[0];
        let sub = data.get(1).copied().unwrap_or(0);

        match status {
            0x10 => Ok(Self::PowerUp),
            0x11 => Ok(Self::PowerUpBillInValidator),
            0x12 => Ok(Self::PowerUpBillInStacker),
            0x13 => Ok(Self::Initializing),
            0x14 => Ok(Self::Idling),
            0x15 => Ok(Self::Accepting),
            0x17 => Ok(Self::Stacking),
            0x18 => Ok(Self::Returning),
            0x19 => Ok(Self::UnitDisabled),
            0x1A => Ok(Self::Holding),
            0x1B => Ok(Self::Busy),
            0x1C => Ok(Self::Rejecting(RejectReason::from_byte(sub))),

            0x41 => Ok(Self::CassetteFull),
            0x42 => Ok(Self::CassetteOutOfPosition),
            0x43 => Ok(Self::ValidatorJammed),
            0x44 => Ok(Self::CassetteJammed),
            0x45 => Ok(Self::Cheated),
            0x46 => Ok(Self::Paused),
            0x47 => Ok(Self::Failure(FailureCode::from_byte(sub))),

            // Bill events use three fixed single-byte codes.
            // The bill type is carried in byte[1] of the POLL response (1-based
            // device numbering); we convert to 0-based for table indexing.
            0x80 => Ok(Self::EscrowPosition {
                bill_type: sub.saturating_sub(1),
            }),
            0x81 => Ok(Self::BillStacked {
                bill_type: sub.saturating_sub(1),
            }),
            0x82 => Ok(Self::BillReturned {
                bill_type: sub.saturating_sub(1),
            }),

            other => Err(Error::UnknownStatus(other)),
        }
    }

    /// Returns `true` if the device has a bill in escrow awaiting a
    /// `Stack` or `Return` decision.
    pub fn is_escrow(&self) -> bool {
        matches!(self, Self::EscrowPosition { .. })
    }

    /// Returns `true` if the device is in a fault / error state.
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            Self::CassetteFull
                | Self::CassetteOutOfPosition
                | Self::ValidatorJammed
                | Self::CassetteJammed
                | Self::Cheated
                | Self::Paused
                | Self::Failure(_)
        )
    }

    /// Returns `true` if the device is ready to accept bills.
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Idling)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_idling() {
        let state = DeviceState::from_poll_data(&[0x14]).unwrap();

        assert_eq!(state, DeviceState::Idling);
        assert!(state.is_ready());
    }

    #[test]
    fn parse_escrow_bill_type_5() {
        // device reports 1-based type 6 in byte[1] → 0-based type 5
        let state = DeviceState::from_poll_data(&[0x80, 0x06]).unwrap();

        assert_eq!(state, DeviceState::EscrowPosition { bill_type: 5 });
        assert!(state.is_escrow());
    }

    #[test]
    fn parse_rejection_with_subreason() {
        let state = DeviceState::from_poll_data(&[0x1C, 0x60]).unwrap();

        assert_eq!(state, DeviceState::Rejecting(RejectReason::Insertion));
    }

    #[test]
    fn parse_failure_with_subcode() {
        let state = DeviceState::from_poll_data(&[0x47, 0x43]).unwrap();

        assert_eq!(
            state,
            DeviceState::Failure(FailureCode::TransportMotorFailure)
        );
    }

    #[test]
    fn parse_bill_stacked() {
        // device reports 1-based type 1 in byte[1] → 0-based type 0
        let state = DeviceState::from_poll_data(&[0x81, 0x01]).unwrap();

        assert_eq!(state, DeviceState::BillStacked { bill_type: 0 });
    }

    #[test]
    fn unknown_status_is_error() {
        assert!(matches!(
            DeviceState::from_poll_data(&[0x20]),
            Err(Error::UnknownStatus(0x20))
        ));
    }
}
