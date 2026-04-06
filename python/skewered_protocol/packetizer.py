from __future__ import annotations

from .types import BadChecksum, DecodeError, EventPacket, NoMarker, State
from .envelope import (
    unwrap_packet, PACKET_TERMINATOR,
    STATE_PACKET_TYPE, EVENT_PACKET_TYPE,
    STATE_PACKET_LEN, EVENT_PACKET_LEN,
)
from .state import decode_state_data
from .event import decode_event_data


_FEED_RESULT = State | EventPacket | BadChecksum | NoMarker | None


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

    def _has_marker_from_end(self, n: int, marker: int) -> bool:
        """Returns True if ``marker`` is at ``n`` bytes from the write head."""
        if self._len < n:
            return False
        idx = (self._write + STATE_PACKET_LEN - n) % STATE_PACKET_LEN
        return self._buf[idx] == marker

    def _linearize(self, n: int) -> None:
        """Rotates the circular buffer in place so that the last ``n`` bytes
        start at index 0."""
        start = (self._write + STATE_PACKET_LEN - n) % STATE_PACKET_LEN
        if start == 0:
            return
        tmp = bytearray(STATE_PACKET_LEN)
        tmp[:] = self._buf
        for i in range(STATE_PACKET_LEN):
            self._buf[i] = tmp[(start + i) % STATE_PACKET_LEN]
        self._write = self._len % STATE_PACKET_LEN

    def feed(self, byte: int) -> _FEED_RESULT:
        """Feeds a single byte into the packetizer.

        Returns:
            - A decoded :class:`State` or :class:`EventPacket` when a valid
              packet is completed.
            - :class:`BadChecksum` when a terminator was seen with a matching
              packet type marker but the checksum failed (corruption).
            - :class:`NoMarker` when a ``0xFF`` terminator was seen but no
              packet type marker was at the expected position (spurious byte).
            - ``None`` when more bytes are needed.
        """
        self._buf[self._write] = byte
        self._write = (self._write + 1) % STATE_PACKET_LEN
        if self._len < STATE_PACKET_LEN:
            self._len += 1

        if byte != PACKET_TERMINATOR:
            return None

        marker_found = False

        if self._has_marker_from_end(STATE_PACKET_LEN, STATE_PACKET_TYPE):
            marker_found = True
            self._linearize(STATE_PACKET_LEN)
            try:
                kind, data = unwrap_packet(bytes(self._buf[:STATE_PACKET_LEN]))
                result = decode_state_data(data)
                self.reset()
                return result
            except DecodeError:
                pass

        if self._has_marker_from_end(EVENT_PACKET_LEN, EVENT_PACKET_TYPE):
            marker_found = True
            self._linearize(EVENT_PACKET_LEN)
            try:
                kind, data = unwrap_packet(bytes(self._buf[:EVENT_PACKET_LEN]))
                result = decode_event_data(data)
                self.reset()
                return result
            except DecodeError:
                pass

        if not marker_found:
            return NoMarker()

        # If we found a terminator and a matching marker but didn't return
        # a packet, the only other possibility is a bad checksum.
        #
        # This could change in the future if there are packet failure
        # conditions.
        return BadChecksum()

    def buffer(self) -> bytes:
        """Returns the current buffer contents.

        Rotates the internal circular buffer so the data is contiguous, then
        returns a copy. Useful for inspecting the buffer after
        :class:`BadChecksum` or :class:`NoMarker`.
        """
        self._linearize(self._len)
        return bytes(self._buf[:self._len])

    @property
    def buffered_len(self) -> int:
        """Returns the number of bytes currently in the buffer."""
        return self._len

    def feed_bytes(self, data: bytes | bytearray) -> tuple[_FEED_RESULT, bytes | bytearray]:
        """Feeds a slice of bytes, returning when a packet or bad checksum is
        found, or all bytes are consumed.

        Returns ``(result, remaining)`` where ``remaining`` is the unconsumed
        tail of the input:

        - A :class:`State` or :class:`EventPacket` — a valid packet was
          decoded; ``remaining`` contains the bytes after it.
        - :class:`BadChecksum` — a terminator with a matching type marker but
          bad checksum was found; ``remaining`` contains the bytes after it.
        - :class:`NoMarker` — all bytes were consumed without finding a valid
          packet, but at least one ``0xFF`` terminator was seen without a
          corresponding type marker.
        - ``None`` — all bytes consumed, no terminator seen.

        :class:`NoMarker` results are accumulated (not returned immediately)
        because they commonly occur with false terminators (e.g. a ``0xFF``
        checksum byte just before the real ``0xFF`` terminator).
        """
        saw_no_marker = False
        for i, b in enumerate(data):
            msg = self.feed(b)
            if msg is None:
                continue
            if isinstance(msg, NoMarker):
                saw_no_marker = True
            elif isinstance(msg, BadChecksum):
                return (BadChecksum(), data[i + 1:])
            else:
                return (msg, data[i + 1:])
        if saw_no_marker:
            return (NoMarker(), data[0:0])
        return (None, data[0:0])

    def reset(self):
        """Resets the packetizer, discarding any buffered data."""
        self._len = 0
        self._write = 0
