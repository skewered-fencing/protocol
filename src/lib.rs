#![no_std]
#![forbid(unsafe_code)]

pub mod envelope;
pub mod error;
pub mod event;
pub mod state;
pub mod types;

#[cfg(feature = "serial")]
pub mod packetizer;

pub use envelope::{
    EVENT_DATA_LEN, EVENT_PACKET_LEN, EVENT_PACKET_TYPE, PACKET_TERMINATOR, Packet, PacketData,
    STATE_DATA_LEN, STATE_PACKET_LEN, STATE_PACKET_TYPE, checksum, unwrap_packet,
    wrap_event_packet, wrap_state_packet,
};
pub use error::DecodeError;
pub use event::{Event, EventPacket, decode_event_data, encode_event_data};
pub use state::{State, decode_state_data, encode_state_data};
pub use types::*;

#[cfg(feature = "serial")]
pub use packetizer::{FeedResult, Packetizer};

/// A fully decoded message: either a state update or an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    State(State),
    Event(EventPacket),
}

/// Decodes a complete serial packet (16-byte state or 6-byte event) into a
/// [`Message`].
///
/// Validates the envelope (type byte, checksum, terminator) and decodes the
/// payload in one step.
pub fn decode_packet(buf: &[u8]) -> Result<Message, DecodeError> {
    unwrap_packet(buf)?.decode()
}

impl Packet {
    /// Decodes the raw packet payload into a [`Message`].
    pub fn decode(self) -> Result<Message, DecodeError> {
        match self.data {
            PacketData::State(data) => Ok(Message::State(decode_state_data(&data)?)),
            PacketData::Event(data) => Ok(Message::Event(decode_event_data(&data)?)),
        }
    }
}

/// Encodes a [`State`] into a complete 16-byte serial packet (with envelope).
pub fn encode_state_packet(state: &State) -> [u8; STATE_PACKET_LEN] {
    wrap_state_packet(&encode_state_data(state))
}

/// Encodes an [`Event`] into a complete 6-byte serial packet (with envelope).
pub fn encode_event_packet(event: &Event, dropped_count: u8) -> [u8; EVENT_PACKET_LEN] {
    wrap_event_packet(&encode_event_data(event, dropped_count))
}
