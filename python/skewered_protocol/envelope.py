from __future__ import annotations

from .types import DecodeError

STATE_PACKET_TYPE = 0xEE
EVENT_PACKET_TYPE = 0xED
PACKET_TERMINATOR = 0xFF

STATE_DATA_LEN = 13
EVENT_DATA_LEN = 3
STATE_PACKET_LEN = STATE_DATA_LEN + 3  # 16
EVENT_PACKET_LEN = EVENT_DATA_LEN + 3  # 6


def checksum(buf: bytes | bytearray) -> int:
    """Wrapping sum of all bytes, truncated to one byte."""
    return sum(buf) & 0xFF


def wrap_state_packet(data: bytes) -> bytes:
    """Wraps 13 bytes of state data into a 16-byte serial packet."""
    if len(data) != STATE_DATA_LEN:
        raise DecodeError(f"expected {STATE_DATA_LEN} data bytes, got {len(data)}")
    packet = bytearray(STATE_PACKET_LEN)
    packet[0] = STATE_PACKET_TYPE
    packet[1:STATE_DATA_LEN + 1] = data
    packet[STATE_DATA_LEN + 1] = checksum(packet[:STATE_DATA_LEN + 1])
    packet[STATE_DATA_LEN + 2] = PACKET_TERMINATOR
    return bytes(packet)


def wrap_event_packet(data: bytes) -> bytes:
    """Wraps 3 bytes of event data into a 6-byte serial packet."""
    if len(data) != EVENT_DATA_LEN:
        raise DecodeError(f"expected {EVENT_DATA_LEN} data bytes, got {len(data)}")
    packet = bytearray(EVENT_PACKET_LEN)
    packet[0] = EVENT_PACKET_TYPE
    packet[1:EVENT_DATA_LEN + 1] = data
    packet[EVENT_DATA_LEN + 1] = checksum(packet[:EVENT_DATA_LEN + 1])
    packet[EVENT_DATA_LEN + 2] = PACKET_TERMINATOR
    return bytes(packet)


def unwrap_packet(buf: bytes | bytearray) -> tuple[str, bytes]:
    """Validates and unwraps a serial packet.

    Returns ("state", 13-byte data) or ("event", 3-byte data).
    Raises DecodeError on invalid packets.
    """
    n = len(buf)
    if n == STATE_PACKET_LEN:
        if buf[0] != STATE_PACKET_TYPE:
            raise DecodeError("invalid packet type")
        if buf[-1] != PACKET_TERMINATOR:
            raise DecodeError("invalid terminator byte")
        expected = checksum(buf[:STATE_DATA_LEN + 1])
        if buf[STATE_DATA_LEN + 1] != expected:
            raise DecodeError("checksum mismatch")
        return ("state", bytes(buf[1:STATE_DATA_LEN + 1]))
    elif n == EVENT_PACKET_LEN:
        if buf[0] != EVENT_PACKET_TYPE:
            raise DecodeError("invalid packet type")
        if buf[-1] != PACKET_TERMINATOR:
            raise DecodeError("invalid terminator byte")
        expected = checksum(buf[:EVENT_DATA_LEN + 1])
        if buf[EVENT_DATA_LEN + 1] != expected:
            raise DecodeError("checksum mismatch")
        return ("event", bytes(buf[1:EVENT_DATA_LEN + 1]))
    else:
        raise DecodeError("invalid packet length")
