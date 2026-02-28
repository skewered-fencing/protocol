use crate::error::DecodeError;

pub const STATE_PACKET_TYPE: u8 = 0xEE;
pub const EVENT_PACKET_TYPE: u8 = 0xED;
pub const PACKET_TERMINATOR: u8 = 0xFF;

pub const STATE_DATA_LEN: usize = 13;
pub const EVENT_DATA_LEN: usize = 3;
pub const STATE_PACKET_LEN: usize = STATE_DATA_LEN + 3; // 16
pub const EVENT_PACKET_LEN: usize = EVENT_DATA_LEN + 3; // 6

/// Computes the checksum: wrapping sum of all bytes.
pub fn checksum(buf: &[u8]) -> u8 {
    buf.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

/// Wraps 13 bytes of state data into a 16-byte serial packet.
pub fn wrap_state_packet(data: &[u8; STATE_DATA_LEN]) -> [u8; STATE_PACKET_LEN] {
    let mut packet = [0u8; STATE_PACKET_LEN];
    packet[0] = STATE_PACKET_TYPE;
    packet[1..=STATE_DATA_LEN].copy_from_slice(data);
    packet[STATE_DATA_LEN + 1] = checksum(&packet[0..STATE_DATA_LEN + 1]);
    packet[STATE_DATA_LEN + 2] = PACKET_TERMINATOR;
    packet
}

/// Wraps 3 bytes of event data into a 6-byte serial packet.
pub fn wrap_event_packet(data: &[u8; EVENT_DATA_LEN]) -> [u8; EVENT_PACKET_LEN] {
    let mut packet = [0u8; EVENT_PACKET_LEN];
    packet[0] = EVENT_PACKET_TYPE;
    packet[1..=EVENT_DATA_LEN].copy_from_slice(data);
    packet[EVENT_DATA_LEN + 1] = checksum(&packet[0..EVENT_DATA_LEN + 1]);
    packet[EVENT_DATA_LEN + 2] = PACKET_TERMINATOR;
    packet
}

/// The data payload extracted from a packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketData {
    State([u8; STATE_DATA_LEN]),
    Event([u8; EVENT_DATA_LEN]),
}

/// A validated packet with its type and data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Packet {
    pub data: PacketData,
}

/// Validates and unwraps a serial packet (either 16-byte state or 6-byte
/// event).
pub fn unwrap_packet(buf: &[u8]) -> Result<Packet, DecodeError> {
    match buf.len() {
        STATE_PACKET_LEN => {
            if buf[0] != STATE_PACKET_TYPE {
                return Err(DecodeError::InvalidPacketType);
            }
            if buf[STATE_PACKET_LEN - 1] != PACKET_TERMINATOR {
                return Err(DecodeError::InvalidTerminator);
            }
            let expected = checksum(&buf[0..STATE_DATA_LEN + 1]);
            if buf[STATE_DATA_LEN + 1] != expected {
                return Err(DecodeError::ChecksumMismatch);
            }
            let mut data = [0u8; STATE_DATA_LEN];
            data.copy_from_slice(&buf[1..=STATE_DATA_LEN]);
            Ok(Packet {
                data: PacketData::State(data),
            })
        }
        EVENT_PACKET_LEN => {
            if buf[0] != EVENT_PACKET_TYPE {
                return Err(DecodeError::InvalidPacketType);
            }
            if buf[EVENT_PACKET_LEN - 1] != PACKET_TERMINATOR {
                return Err(DecodeError::InvalidTerminator);
            }
            let expected = checksum(&buf[0..EVENT_DATA_LEN + 1]);
            if buf[EVENT_DATA_LEN + 1] != expected {
                return Err(DecodeError::ChecksumMismatch);
            }
            let mut data = [0u8; EVENT_DATA_LEN];
            data.copy_from_slice(&buf[1..=EVENT_DATA_LEN]);
            Ok(Packet {
                data: PacketData::Event(data),
            })
        }
        _ => Err(DecodeError::InvalidLength),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_unwrap_state_roundtrip() {
        let data = [0u8; STATE_DATA_LEN];
        let packet = wrap_state_packet(&data);
        assert_eq!(packet[0], STATE_PACKET_TYPE);
        assert_eq!(packet[STATE_PACKET_LEN - 1], PACKET_TERMINATOR);
        let unwrapped = unwrap_packet(&packet).unwrap();
        assert_eq!(unwrapped.data, PacketData::State(data));
    }

    #[test]
    fn wrap_unwrap_event_roundtrip() {
        let data = [0x22, 0x00, 0x0D];
        let packet = wrap_event_packet(&data);
        assert_eq!(packet[0], EVENT_PACKET_TYPE);
        assert_eq!(packet[EVENT_PACKET_LEN - 1], PACKET_TERMINATOR);
        let unwrapped = unwrap_packet(&packet).unwrap();
        assert_eq!(unwrapped.data, PacketData::Event(data));
    }

    #[test]
    fn bad_checksum() {
        let data = [0u8; STATE_DATA_LEN];
        let mut packet = wrap_state_packet(&data);
        packet[STATE_DATA_LEN + 1] ^= 0x01; // corrupt checksum
        assert_eq!(unwrap_packet(&packet), Err(DecodeError::ChecksumMismatch));
    }

    #[test]
    fn bad_terminator() {
        let data = [0u8; STATE_DATA_LEN];
        let mut packet = wrap_state_packet(&data);
        packet[STATE_PACKET_LEN - 1] = 0xFE;
        assert_eq!(unwrap_packet(&packet), Err(DecodeError::InvalidTerminator));
    }

    #[test]
    fn bad_length() {
        assert_eq!(
            unwrap_packet(&[0xEE, 0x00]),
            Err(DecodeError::InvalidLength)
        );
    }

    #[test]
    fn bad_packet_type() {
        let data = [0u8; STATE_DATA_LEN];
        let mut packet = wrap_state_packet(&data);
        packet[0] = 0xAA;
        assert_eq!(unwrap_packet(&packet), Err(DecodeError::InvalidPacketType));
    }

    #[test]
    fn checksum_is_wrapping_sum() {
        assert_eq!(checksum(&[0xFF, 0x01]), 0x00);
        assert_eq!(checksum(&[0x80, 0x80]), 0x00);
        assert_eq!(
            checksum(&[
                0xEE, 0x00, 0x01, 0x00, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
            ]),
            163
        );
    }
}
