from __future__ import annotations

from .types import DecodeError, EventPacket, InvalidPacket, State
from .envelope import (
    unwrap_packet, PACKET_TERMINATOR,
    STATE_PACKET_LEN, EVENT_PACKET_LEN,
)
from .state import decode_state_data
from .event import decode_event_data


class Packetizer:
    """Stream-based byte parser that reassembles serial bytes into decoded messages.

    Uses a circular buffer internally. Synchronizes on 0xFF terminator bytes,
    then checks whether the accumulated buffer forms a valid state or event
    packet. If decoding fails at a 0xFF byte (e.g. because the checksum
    happened to be 0xFF), the buffer is preserved and parsing continues to the
    next terminator.
    """

    def __init__(self):
        self._buf = bytearray(STATE_PACKET_LEN)
        self._write = 0
        self._len = 0

    def _linearize(self, n: int) -> bytes:
        """Copies the last `n` bytes from the circular buffer."""
        start = (self._write + STATE_PACKET_LEN - n) % STATE_PACKET_LEN
        return bytes(self._buf[(start + i) % STATE_PACKET_LEN] for i in range(n))

    def feed(self, byte: int) -> State | EventPacket | InvalidPacket | None:
        """Feeds a single byte into the packetizer.

        Returns a decoded State or EventPacket when a valid packet is completed,
        :class:`InvalidPacket` when a terminator was seen but did not form a
        valid packet, or ``None`` when more bytes are needed.

        If a 0xFF byte is encountered but does not terminate a valid packet
        (e.g. a checksum that happens to be 0xFF), the buffer is preserved
        and parsing continues.
        """
        self._buf[self._write] = byte
        self._write = (self._write + 1) % STATE_PACKET_LEN
        if self._len < STATE_PACKET_LEN:
            self._len += 1

        if byte != PACKET_TERMINATOR:
            return None

        # Terminator seen -- try state packet (16 bytes), then event packet (6 bytes)
        if self._len >= STATE_PACKET_LEN:
            try:
                kind, data = unwrap_packet(self._linearize(STATE_PACKET_LEN))
                result = decode_state_data(data)
                self.reset()
                return result
            except DecodeError:
                pass

        if self._len >= EVENT_PACKET_LEN:
            try:
                kind, data = unwrap_packet(self._linearize(EVENT_PACKET_LEN))
                result = decode_event_data(data)
                self.reset()
                return result
            except DecodeError:
                pass

        # Failed to decode -- could be a false terminator (e.g. 0xFF checksum).
        # Don't clear buffer; continue accumulating.
        return InvalidPacket()

    def feed_bytes(self, data: bytes | bytearray) -> list[State | EventPacket | InvalidPacket]:
        """Feeds a slice of bytes, returning a list of decoded messages and invalid markers."""
        results = []
        for b in data:
            msg = self.feed(b)
            if msg is not None:
                results.append(msg)
        return results

    def reset(self):
        """Resets the packetizer, discarding any buffered data."""
        self._len = 0
        self._write = 0
