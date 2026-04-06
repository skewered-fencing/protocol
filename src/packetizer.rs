use crate::envelope::{
    EVENT_PACKET_LEN, EVENT_PACKET_TYPE, PACKET_TERMINATOR, Packet, STATE_PACKET_LEN,
    STATE_PACKET_TYPE, unwrap_packet,
};

/// Result of feeding a byte into the [`Packetizer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedResult {
    /// More bytes needed; no packet boundary reached yet.
    Pending,
    /// A valid packet was decoded.
    Packet(Packet),
    /// A `0xFF` terminator was seen and a packet type marker was found at the
    /// expected position, but the checksum did not match. This indicates
    /// corruption on the wire.
    BadChecksum,
    /// A `0xFF` terminator was seen but no packet type marker (`0xEE` or
    /// `0xED`) was found at the expected position. This is typically a
    /// spurious `0xFF` byte in the data stream (e.g. event payloads or a
    /// checksum that happens to be `0xFF`).
    NoMarker,
}

/// Stream-based byte parser that reassembles serial bytes into validated
/// packets.
///
/// Uses a circular buffer internally. Synchronizes on `0xFF` terminator bytes,
/// then checks whether the accumulated buffer forms a valid state or event
/// packet. If decoding fails at a `0xFF` byte (e.g. because the checksum
/// happened to be `0xFF`), the buffer is preserved and parsing continues to the
/// next terminator.
pub struct Packetizer {
    buf: [u8; STATE_PACKET_LEN],
    write: usize,
    len: usize,
}

impl Packetizer {
    /// Creates a new packetizer with an empty buffer.
    pub const fn new() -> Self {
        Self {
            buf: [0; STATE_PACKET_LEN],
            write: 0,
            len: 0,
        }
    }

    /// Feeds a single byte into the packetizer.
    ///
    /// Returns:
    /// - [`FeedResult::Packet`] — a valid packet was decoded and the buffer
    ///   was reset.
    /// - [`FeedResult::BadChecksum`] — a terminator was seen with a matching
    ///   packet type marker, but the checksum failed. This signals corruption.
    ///   The buffer is preserved (call [`buffer()`](Packetizer::buffer) to
    ///   inspect it).
    /// - [`FeedResult::NoMarker`] — a `0xFF` terminator was seen but no packet
    ///   type marker was found at the expected position. Most likely a spurious
    ///   `0xFF` in the data stream. The buffer is preserved.
    /// - [`FeedResult::Pending`] — a non-terminator byte was buffered.
    pub fn feed(&mut self, byte: u8) -> FeedResult {
        self.buf[self.write] = byte;
        self.write = (self.write + 1) % STATE_PACKET_LEN;
        if self.len < STATE_PACKET_LEN {
            self.len += 1;
        }

        if byte != PACKET_TERMINATOR {
            return FeedResult::Pending;
        }

        // Terminator seen. Check for a packet type marker at the expected
        // position and attempt to decode if found.

        let mut marker_found = false;

        // STATE packet (16 bytes, type 0xEE)
        if self.has_marker_from_end(STATE_PACKET_LEN, STATE_PACKET_TYPE) {
            marker_found = true;
            self.linearize(STATE_PACKET_LEN);
            if let Ok(pkt) = unwrap_packet(&self.buf[..STATE_PACKET_LEN]) {
                self.reset();
                return FeedResult::Packet(pkt);
            }
        }

        // EVENT packet (6 bytes, type 0xED)
        if self.has_marker_from_end(EVENT_PACKET_LEN, EVENT_PACKET_TYPE) {
            marker_found = true;
            self.linearize(EVENT_PACKET_LEN);
            if let Ok(pkt) = unwrap_packet(&self.buf[..EVENT_PACKET_LEN]) {
                self.reset();
                return FeedResult::Packet(pkt);
            }
        }

        if !marker_found {
            FeedResult::NoMarker
        } else {
            // If we found a terminator and a matching marker but didn't return
            // a packet, the only other possibility is a bad checksum.
            //
            // This could change in the future if there are packet failure
            // conditions.
            FeedResult::BadChecksum
        }
    }

    /// Returns true if the buffer has `marker` at `n` bytes from the end.
    ///
    /// Used to check for packet type markers (STATE=0xEE, EVENT=0xED) before
    /// attempting to decode, avoiding unnecessary work.
    fn has_marker_from_end(&self, n: usize, marker: u8) -> bool {
        if self.len < n {
            return false;
        }
        let idx = (self.write + STATE_PACKET_LEN - n) % STATE_PACKET_LEN;
        self.buf[idx] == marker
    }

    /// Rotates the circular buffer in place so that the oldest byte is at
    /// index 0 and the most recent `n` buffered bytes occupy `buf[0..n]`.
    fn linearize(&mut self, n: usize) {
        let start = (self.write + STATE_PACKET_LEN - n) % STATE_PACKET_LEN;
        if start == 0 {
            return;
        }
        // Rotate the buffer so `start` becomes index 0.
        self.buf.rotate_left(start);
        self.write = self.len % STATE_PACKET_LEN;
    }

    /// Returns the current buffer contents as a slice.
    ///
    /// This rotates the internal circular buffer so the data is contiguous,
    /// then returns a slice of the buffered bytes. The returned slice is valid
    /// until the next call to [`feed()`](Packetizer::feed),
    /// [`feed_bytes()`](Packetizer::feed_bytes), or
    /// [`reset()`](Packetizer::reset).
    ///
    /// Useful for inspecting the buffer after [`FeedResult::BadChecksum`] or
    /// [`FeedResult::NoMarker`].
    pub fn buffer(&mut self) -> &[u8] {
        self.linearize(self.len);
        &self.buf[..self.len]
    }

    /// Returns the number of bytes currently in the buffer.
    pub fn buffered_len(&self) -> usize {
        self.len
    }

    /// Feeds a slice of bytes, returning when a valid packet or bad checksum
    /// is found, or all bytes are consumed.
    ///
    /// Returns `(result, remaining)` where `remaining` is the unconsumed tail
    /// of the input:
    ///
    /// - [`FeedResult::Packet`] — a valid packet was decoded; `remaining`
    ///   contains the bytes after it. Call again to extract more packets.
    /// - [`FeedResult::BadChecksum`] — a terminator with a matching type marker
    ///   but bad checksum was found; `remaining` contains the bytes after it.
    ///   This signals corruption and the caller should inspect or reset.
    /// - [`FeedResult::NoMarker`] — all bytes were consumed without finding a
    ///   valid packet, but at least one `0xFF` terminator was seen without a
    ///   corresponding type marker. This distinguishes "waiting for more data"
    ///   from "receiving spurious terminators."
    /// - [`FeedResult::Pending`] — all bytes consumed, no terminator seen.
    ///
    /// `NoMarker` results are accumulated (not returned immediately) because
    /// they commonly occur with false terminators (e.g. a `0xFF` checksum byte
    /// just before the real `0xFF` terminator). `BadChecksum` returns
    /// immediately because it signals real corruption.
    ///
    /// ```ignore
    /// let mut data = &serial_data[..];
    /// loop {
    ///     let (result, rest) = packetizer.feed_bytes(data);
    ///     data = rest;
    ///     match result {
    ///         FeedResult::Packet(packet) => handle(packet),
    ///         FeedResult::BadChecksum => log_corruption(packetizer.buffer()),
    ///         FeedResult::NoMarker | FeedResult::Pending => break,
    ///     }
    /// }
    /// ```
    pub fn feed_bytes<'a>(&mut self, bytes: &'a [u8]) -> (FeedResult, &'a [u8]) {
        let mut saw_no_marker = false;
        for (i, &b) in bytes.iter().enumerate() {
            match self.feed(b) {
                FeedResult::Pending => {}
                FeedResult::Packet(pkt) => return (FeedResult::Packet(pkt), &bytes[i + 1..]),
                FeedResult::BadChecksum => return (FeedResult::BadChecksum, &bytes[i + 1..]),
                FeedResult::NoMarker => saw_no_marker = true,
            }
        }
        if saw_no_marker {
            (FeedResult::NoMarker, &[])
        } else {
            (FeedResult::Pending, &[])
        }
    }

    /// Resets the packetizer, discarding any buffered data.
    pub fn reset(&mut self) {
        self.len = 0;
        self.write = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::{PacketData, wrap_event_packet, wrap_state_packet};

    #[test]
    fn byte_by_byte_state_packet() {
        let data = [0u8; 13];
        let packet = wrap_state_packet(&data);

        let mut p = Packetizer::new();
        for &b in &packet[..packet.len() - 1] {
            assert_eq!(p.feed(b), FeedResult::Pending);
        }
        assert_eq!(
            p.feed(packet[packet.len() - 1]),
            FeedResult::Packet(Packet {
                data: PacketData::State(data),
            }),
        );
    }

    #[test]
    fn byte_by_byte_event_packet() {
        let data = [0x22, 0x00, 0x0D];
        let packet = wrap_event_packet(&data);

        let mut p = Packetizer::new();
        for &b in &packet[..packet.len() - 1] {
            assert_eq!(p.feed(b), FeedResult::Pending);
        }
        assert_eq!(
            p.feed(packet[packet.len() - 1]),
            FeedResult::Packet(Packet {
                data: PacketData::Event(data),
            }),
        );
    }

    #[test]
    fn back_to_back_packets() {
        let state_data = [0u8; 13];
        let event_data = [0x22, 0x00, 0x00];
        let state_pkt = wrap_state_packet(&state_data);
        let event_pkt = wrap_event_packet(&event_data);

        let mut stream = [0u8; 22]; // 16 + 6
        stream[..16].copy_from_slice(&state_pkt);
        stream[16..].copy_from_slice(&event_pkt);

        let mut p = Packetizer::new();
        let (result, rest) = p.feed_bytes(&stream);
        assert_eq!(
            result,
            FeedResult::Packet(Packet {
                data: PacketData::State(state_data),
            }),
        );
        assert_eq!(rest.len(), 6);

        let (result, rest) = p.feed_bytes(rest);
        assert_eq!(
            result,
            FeedResult::Packet(Packet {
                data: PacketData::Event(event_data),
            }),
        );
        assert!(rest.is_empty());
    }

    #[test]
    fn garbage_before_packet_resync() {
        let data = [0u8; 13];
        let packet = wrap_state_packet(&data);

        let mut stream = [0u8; 19]; // 3 garbage + 16 packet
        stream[0] = 0xAA;
        stream[1] = 0xBB;
        stream[2] = 0xCC;
        stream[3..].copy_from_slice(&packet);

        let mut p = Packetizer::new();
        let (result, rest) = p.feed_bytes(&stream);
        assert!(matches!(result, FeedResult::Packet(_)));
        assert!(rest.is_empty());
    }

    #[test]
    fn corrupt_checksum_returns_bad_checksum() {
        let data = [0u8; 13];
        let mut packet = wrap_state_packet(&data);
        packet[14] ^= 0x01; // corrupt checksum

        let mut p = Packetizer::new();
        let mut saw_bad_checksum = false;
        for &b in &packet {
            match p.feed(b) {
                FeedResult::BadChecksum => saw_bad_checksum = true,
                FeedResult::Packet(_) => panic!("corrupt packet should not decode"),
                FeedResult::Pending | FeedResult::NoMarker => {}
            }
        }
        assert!(
            saw_bad_checksum,
            "should report BadChecksum for corrupt packet"
        );

        // Packetizer recovers and decodes the next valid packet
        let good_packet = wrap_state_packet(&data);
        let (result, _) = p.feed_bytes(&good_packet);
        assert_eq!(
            result,
            FeedResult::Packet(Packet {
                data: PacketData::State(data),
            }),
        );
    }

    #[test]
    fn corrupt_checksum_via_feed_bytes() {
        let data = [0u8; 13];
        let mut packet = wrap_state_packet(&data);
        packet[14] ^= 0x01; // corrupt checksum

        let mut p = Packetizer::new();
        let (result, rest) = p.feed_bytes(&packet);
        assert_eq!(result, FeedResult::BadChecksum);
        assert!(rest.is_empty());
    }

    #[test]
    fn bad_checksum_buffer_inspection() {
        let data = [0u8; 13];
        let mut packet = wrap_state_packet(&data);
        packet[14] ^= 0x01; // corrupt checksum

        let mut p = Packetizer::new();
        let (result, _) = p.feed_bytes(&packet);
        assert_eq!(result, FeedResult::BadChecksum);

        let buf = p.buffer();
        assert_eq!(buf.len(), STATE_PACKET_LEN);
        assert_eq!(buf[0], 0xEE); // type byte
        assert_eq!(buf[15], 0xFF); // terminator
    }

    #[test]
    fn false_terminator_0xff_checksum_feed() {
        // Construct a state packet whose checksum is 0xFF.
        // checksum([0xEE, d0..d12]) = 0xEE + sum(d0..d12) (wrapping)
        // We need 0xEE + sum(data) == 0xFF, so sum(data) == 0x11
        let mut data = [0u8; 13];
        data[0] = 0x11;
        let packet = wrap_state_packet(&data);
        assert_eq!(packet[14], 0xFF, "checksum should be 0xFF");

        // The packet has two consecutive 0xFF bytes: checksum and terminator.
        // feed() returns NoMarker at the false terminator (the checksum byte),
        // then Packet at the real one.
        let mut p = Packetizer::new();
        let mut decoded = None;
        let mut saw_no_marker = false;
        for &b in &packet {
            match p.feed(b) {
                FeedResult::Packet(pkt) => decoded = Some(pkt),
                FeedResult::NoMarker => saw_no_marker = true,
                FeedResult::BadChecksum | FeedResult::Pending => {}
            }
        }
        assert!(saw_no_marker, "should see NoMarker at false terminator");
        let pkt = decoded.expect("should decode packet with 0xFF checksum");
        assert_eq!(pkt.data, PacketData::State(data));
    }

    #[test]
    fn false_terminator_0xff_checksum_feed_bytes() {
        // Same 0xFF-checksum packet, but via feed_bytes(). The NoMarker at
        // the false terminator is suppressed; only Packet is returned.
        let mut data = [0u8; 13];
        data[0] = 0x11;
        let packet = wrap_state_packet(&data);
        assert_eq!(packet[14], 0xFF, "checksum should be 0xFF");

        let mut p = Packetizer::new();
        let (result, rest) = p.feed_bytes(&packet);
        assert_eq!(
            result,
            FeedResult::Packet(Packet {
                data: PacketData::State(data),
            }),
        );
        assert!(rest.is_empty());
    }

    #[test]
    fn feed_bytes_api() {
        let state_data = [0u8; 13];
        let event_data = [0x22, 0x00, 0x00];
        let state_pkt = wrap_state_packet(&state_data);
        let event_pkt = wrap_event_packet(&event_data);

        let mut stream = [0u8; 22]; // 16 + 6
        stream[..16].copy_from_slice(&state_pkt);
        stream[16..].copy_from_slice(&event_pkt);

        let mut p = Packetizer::new();
        let (result, rest) = p.feed_bytes(&stream);
        assert!(matches!(result, FeedResult::Packet(_)));

        let (result, rest) = p.feed_bytes(rest);
        assert!(matches!(result, FeedResult::Packet(_)));

        let (result, _) = p.feed_bytes(rest);
        assert_eq!(result, FeedResult::Pending);
    }

    #[test]
    fn lone_terminator_returns_no_marker() {
        let mut p = Packetizer::new();
        assert_eq!(p.feed(0xFF), FeedResult::NoMarker);
    }

    #[test]
    fn lone_terminator_returns_no_marker_via_feed_bytes() {
        let mut p = Packetizer::new();
        let (result, rest) = p.feed_bytes(&[0xFF]);
        assert_eq!(result, FeedResult::NoMarker);
        assert!(rest.is_empty());
    }

    #[test]
    fn garbage_with_terminators_returns_no_marker_via_feed_bytes() {
        let mut p = Packetizer::new();
        let garbage = [0xAA, 0xBB, 0xFF, 0xCC, 0xDD, 0xFF];
        let (result, rest) = p.feed_bytes(&garbage);
        assert_eq!(result, FeedResult::NoMarker);
        assert!(rest.is_empty());
    }

    #[test]
    fn garbage_without_terminators_returns_pending() {
        let mut p = Packetizer::new();
        let garbage = [0xAA, 0xBB, 0xCC, 0xDD];
        let (result, rest) = p.feed_bytes(&garbage);
        assert_eq!(result, FeedResult::Pending);
        assert!(rest.is_empty());
    }

    #[test]
    fn event_with_0xff_in_payload_left_score_up() {
        // Left score up event: ed 11 01 00 ff ff
        // The 5th byte (0xff) is data, not a terminator.
        let packet = [0xed, 0x11, 0x01, 0x00, 0xff, 0xff];
        let mut p = Packetizer::new();

        for (i, &b) in packet.iter().enumerate() {
            let result = p.feed(b);
            if i < packet.len() - 1 {
                // All bytes except the last should return Pending or NoMarker
                assert!(
                    matches!(result, FeedResult::Pending | FeedResult::NoMarker),
                    "byte {i} (0x{b:02x}) should be Pending or NoMarker, got {result:?}"
                );
            } else {
                assert!(
                    matches!(result, FeedResult::Packet(_)),
                    "final byte should produce Packet, got {result:?}"
                );
            }
        }
    }

    #[test]
    fn event_with_0xff_in_payload_adj_clock_minus_1() {
        // Adjust clock by -1 event: ed 24 ff 00 10 ff
        // The 3rd byte (0xff) is data, not a terminator.
        let packet = [0xed, 0x24, 0xff, 0x00, 0x10, 0xff];
        let mut p = Packetizer::new();

        for (i, &b) in packet.iter().enumerate() {
            let result = p.feed(b);
            if i < packet.len() - 1 {
                assert!(
                    matches!(result, FeedResult::Pending | FeedResult::NoMarker),
                    "byte {i} (0x{b:02x}) should be Pending or NoMarker, got {result:?}"
                );
            } else {
                assert!(
                    matches!(result, FeedResult::Packet(_)),
                    "final byte should produce Packet, got {result:?}"
                );
            }
        }
    }

    #[test]
    fn buffer_returns_buffered_bytes() {
        let mut p = Packetizer::new();
        p.feed(0xAA);
        p.feed(0xBB);
        p.feed(0xCC);
        assert_eq!(p.buffer(), &[0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn buffer_after_wrap() {
        let mut p = Packetizer::new();
        // Fill the ring buffer past its capacity to force wrapping
        for i in 0..20u8 {
            p.feed(i);
        }
        // Buffer should contain the last STATE_PACKET_LEN bytes
        let buf = p.buffer();
        assert_eq!(buf.len(), STATE_PACKET_LEN);
        assert_eq!(buf[0], 4); // 20 - 16
        assert_eq!(buf[15], 19);
    }

    #[test]
    fn reset_clears_buffer() {
        let mut p = Packetizer::new();
        p.feed(0xEE);
        p.feed(0x00);
        p.reset();
        assert_eq!(p.buffered_len(), 0);
        assert!(p.buffer().is_empty());

        // After reset, feeding a complete packet should work
        let data = [0x22, 0x00, 0x00];
        let packet = wrap_event_packet(&data);
        let mut decoded = None;
        for &b in &packet {
            if let FeedResult::Packet(pkt) = p.feed(b) {
                decoded = Some(pkt);
            }
        }
        assert_eq!(decoded.unwrap().data, PacketData::Event(data));
    }
}
