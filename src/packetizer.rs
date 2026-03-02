use crate::envelope::{
    EVENT_PACKET_LEN, PACKET_TERMINATOR, Packet, STATE_PACKET_LEN, unwrap_packet,
};

/// Result of feeding a byte into the [`Packetizer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedResult {
    /// More bytes needed; no packet boundary reached.
    Pending,
    /// A valid packet was decoded.
    Packet(Packet),
    /// A terminator was seen but didn't form a valid packet (corruption,
    /// or rarely a false terminator when the checksum equals `0xFF`).
    Invalid,
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
    /// Returns [`FeedResult::Packet`] when a valid packet is completed,
    /// [`FeedResult::Invalid`] when a terminator was seen but did not form a
    /// valid packet, or [`FeedResult::Pending`] when more bytes are needed.
    ///
    /// If a `0xFF` byte is encountered but does not terminate a valid packet
    /// (e.g. a checksum that happens to be `0xFF`), the buffer is preserved
    /// and parsing continues.
    pub fn feed(&mut self, byte: u8) -> FeedResult {
        self.buf[self.write] = byte;
        self.write = (self.write + 1) % STATE_PACKET_LEN;
        if self.len < STATE_PACKET_LEN {
            self.len += 1;
        }

        if byte != PACKET_TERMINATOR {
            return FeedResult::Pending;
        }

        // Terminator seen -- try state packet (16 bytes), then event packet (6
        // bytes)
        if self.len >= STATE_PACKET_LEN {
            let mut tmp = [0u8; STATE_PACKET_LEN];
            self.linearize(&mut tmp, STATE_PACKET_LEN);
            if let Ok(pkt) = unwrap_packet(&tmp) {
                self.reset();
                return FeedResult::Packet(pkt);
            }
        }

        if self.len >= EVENT_PACKET_LEN {
            let mut tmp = [0u8; EVENT_PACKET_LEN];
            self.linearize(&mut tmp, EVENT_PACKET_LEN);
            if let Ok(pkt) = unwrap_packet(&tmp) {
                self.reset();
                return FeedResult::Packet(pkt);
            }
        }

        // Failed to decode -- could be a false terminator (e.g. 0xFF checksum).
        // Don't clear buffer; continue accumulating.
        FeedResult::Invalid
    }

    /// Copies the last `n` bytes from the circular buffer into `out`.
    fn linearize(&self, out: &mut [u8], n: usize) {
        let start = (self.write + STATE_PACKET_LEN - n) % STATE_PACKET_LEN;
        for i in 0..n {
            out[i] = self.buf[(start + i) % STATE_PACKET_LEN];
        }
    }

    /// Feeds a slice of bytes, processing until a packet or invalid terminator
    /// is found, or all bytes are consumed.
    ///
    /// Returns `(result, remaining)` where `remaining` is the unconsumed tail
    /// of the input. The result is [`FeedResult::Pending`] only when all bytes
    /// are consumed without reaching a packet boundary.
    ///
    /// Call repeatedly with the returned remaining slice to extract multiple
    /// packets:
    /// ```ignore
    /// let mut data = &serial_data[..];
    /// loop {
    ///     let (result, rest) = packetizer.feed_bytes(data);
    ///     data = rest;
    ///     match result {
    ///         FeedResult::Packet(packet) => handle(packet),
    ///         FeedResult::Invalid => count_corruption(),
    ///         FeedResult::Pending => break,
    ///     }
    /// }
    /// ```
    pub fn feed_bytes<'a>(&mut self, bytes: &'a [u8]) -> (FeedResult, &'a [u8]) {
        for (i, &b) in bytes.iter().enumerate() {
            match self.feed(b) {
                FeedResult::Pending => {}
                result => return (result, &bytes[i + 1..]),
            }
        }
        (FeedResult::Pending, &[])
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
    fn corrupt_checksum_returns_invalid() {
        let data = [0u8; 13];
        let mut packet = wrap_state_packet(&data);
        packet[14] ^= 0x01; // corrupt checksum

        let mut p = Packetizer::new();
        let mut saw_invalid = false;
        for &b in &packet {
            match p.feed(b) {
                FeedResult::Invalid => saw_invalid = true,
                FeedResult::Packet(_) => panic!("corrupt packet should not decode"),
                FeedResult::Pending => {}
            }
        }
        assert!(saw_invalid, "should report Invalid for corrupt packet");

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
    fn false_terminator_0xff_checksum() {
        // Construct a state packet whose checksum is 0xFF.
        // Packet format: [TYPE, DATA[13], CHECKSUM, TERMINATOR]
        // We need checksum(TYPE || DATA) == 0xFF.
        // checksum([0xEE, d0..d12]) = 0xEE + sum(d0..d12) (wrapping)
        // We need 0xEE + sum(data) == 0xFF, so sum(data) == 0x11
        let mut data = [0u8; 13];
        data[0] = 0x11; // sum of data = 0x11
        let packet = wrap_state_packet(&data);
        assert_eq!(packet[14], 0xFF, "checksum should be 0xFF");

        // The packet has two consecutive 0xFF bytes: checksum and terminator.
        // The packetizer should handle the false terminator (checksum) and
        // still decode the packet when the real terminator arrives.
        let mut p = Packetizer::new();
        let mut decoded = None;
        for &b in &packet {
            match p.feed(b) {
                FeedResult::Packet(pkt) => decoded = Some(pkt),
                FeedResult::Invalid | FeedResult::Pending => {}
            }
        }
        let pkt = decoded.expect("should decode packet with 0xFF checksum");
        assert_eq!(pkt.data, PacketData::State(data));
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
    fn lone_terminator_returns_invalid() {
        let mut p = Packetizer::new();
        // A lone 0xFF can't form a valid packet
        assert_eq!(p.feed(0xFF), FeedResult::Invalid);
    }

    #[test]
    fn reset_clears_buffer() {
        let mut p = Packetizer::new();
        p.feed(0xEE);
        p.feed(0x00);
        p.reset();

        // After reset, feeding a complete packet should work
        let data = [0x22, 0x00, 0x00];
        let packet = wrap_event_packet(&data);
        let mut decoded = None;
        for &b in &packet {
            if let FeedResult::Packet(pkt) = p.feed(b) {
                decoded = Some(pkt);
            }
        }
        assert_eq!(decoded.unwrap().data, PacketData::Event(data),);
    }
}
