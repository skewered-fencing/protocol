use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    InvalidLength,
    InvalidPacketType,
    InvalidTerminator,
    ChecksumMismatch,
    InvalidWeapon,
    InvalidPriority,
    InvalidPeriod,
    InvalidCard,
    InvalidLatchedLight,
    InvalidEventId,
    InvalidEventData,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength => f.write_str("invalid packet length"),
            Self::InvalidPacketType => f.write_str("invalid packet type"),
            Self::InvalidTerminator => f.write_str("invalid terminator byte"),
            Self::ChecksumMismatch => f.write_str("checksum mismatch"),
            Self::InvalidWeapon => f.write_str("invalid weapon value"),
            Self::InvalidPriority => f.write_str("invalid priority value"),
            Self::InvalidPeriod => f.write_str("invalid period value"),
            Self::InvalidCard => f.write_str("invalid card value"),
            Self::InvalidLatchedLight => f.write_str("invalid latched light value"),
            Self::InvalidEventId => f.write_str("invalid event ID"),
            Self::InvalidEventData => f.write_str("invalid event data"),
        }
    }
}
