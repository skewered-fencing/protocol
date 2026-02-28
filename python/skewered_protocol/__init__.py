"""skewered_protocol -- encoder/decoder for the Skewered Fencing scoring box protocol."""

from .types import (
    DecodeError,
    Weapon, Side, Priority, Card, MenuKey,
    FencerCards, FencerScore, FencerStripInput, StripInput,
    Clock, LatchedLight, State, Event, EventPacket,
)
from .envelope import (
    checksum,
    wrap_state_packet, wrap_event_packet, unwrap_packet,
    STATE_PACKET_TYPE, EVENT_PACKET_TYPE, PACKET_TERMINATOR,
    STATE_DATA_LEN, EVENT_DATA_LEN, STATE_PACKET_LEN, EVENT_PACKET_LEN,
)
from .state import decode_state_data, encode_state_data
from .event import decode_event_data, encode_event_data
from .packetizer import Packetizer


def decode_packet(buf: bytes) -> State | EventPacket:
    """Decode a complete serial packet (16-byte state or 6-byte event) into a State or EventPacket."""
    kind, data = unwrap_packet(buf)
    if kind == "state":
        return decode_state_data(data)
    else:
        return decode_event_data(data)


def encode_state_packet(state: State) -> bytes:
    """Encode a State into a complete 16-byte serial packet."""
    return wrap_state_packet(encode_state_data(state))


def encode_event_packet(event: Event, dropped_count: int = 0) -> bytes:
    """Encode an Event into a complete 6-byte serial packet."""
    return wrap_event_packet(encode_event_data(event, dropped_count))


__all__ = [
    "DecodeError",
    "Weapon", "Side", "Priority", "Card", "MenuKey",
    "FencerCards", "FencerScore", "FencerStripInput", "StripInput",
    "Clock", "LatchedLight", "State", "Event", "EventPacket",
    "checksum",
    "wrap_state_packet", "wrap_event_packet", "unwrap_packet",
    "STATE_PACKET_TYPE", "EVENT_PACKET_TYPE", "PACKET_TERMINATOR",
    "STATE_DATA_LEN", "EVENT_DATA_LEN", "STATE_PACKET_LEN", "EVENT_PACKET_LEN",
    "decode_state_data", "encode_state_data",
    "decode_event_data", "encode_event_data",
    "Packetizer",
    "decode_packet", "encode_state_packet", "encode_event_packet",
]
