import pytest
from skewered_protocol import (
    Card, Clock, DecodeError, Event, EventPacket,
    FencerCards, FencerScore, FencerStripInput, LatchedLight,
    MenuKey, Packetizer, Priority, Side, State, StripInput, Weapon,
    checksum, decode_event_data, decode_packet, decode_state_data,
    encode_event_data, encode_state_data, encode_state_packet,
    encode_event_packet, unwrap_packet, wrap_event_packet,
    wrap_state_packet,
    STATE_PACKET_TYPE, EVENT_PACKET_TYPE, PACKET_TERMINATOR,
    STATE_DATA_LEN, EVENT_DATA_LEN, STATE_PACKET_LEN, EVENT_PACKET_LEN,
)


# ── Envelope ──────────────────────────────────────────────────────────

class TestEnvelope:
    def test_wrap_unwrap_state_roundtrip(self):
        data = bytes(STATE_DATA_LEN)
        packet = wrap_state_packet(data)
        assert packet[0] == STATE_PACKET_TYPE
        assert packet[-1] == PACKET_TERMINATOR
        assert len(packet) == STATE_PACKET_LEN
        kind, unwrapped = unwrap_packet(packet)
        assert kind == "state"
        assert unwrapped == data

    def test_wrap_unwrap_event_roundtrip(self):
        data = bytes([0x22, 0x00, 0x0D])
        packet = wrap_event_packet(data)
        assert packet[0] == EVENT_PACKET_TYPE
        assert packet[-1] == PACKET_TERMINATOR
        assert len(packet) == EVENT_PACKET_LEN
        kind, unwrapped = unwrap_packet(packet)
        assert kind == "event"
        assert unwrapped == data

    def test_bad_checksum(self):
        data = bytes(STATE_DATA_LEN)
        packet = bytearray(wrap_state_packet(data))
        packet[STATE_DATA_LEN + 1] ^= 0x01
        with pytest.raises(DecodeError, match="checksum"):
            unwrap_packet(bytes(packet))

    def test_bad_terminator(self):
        data = bytes(STATE_DATA_LEN)
        packet = bytearray(wrap_state_packet(data))
        packet[-1] = 0xFE
        with pytest.raises(DecodeError, match="terminator"):
            unwrap_packet(bytes(packet))

    def test_bad_length(self):
        with pytest.raises(DecodeError, match="length"):
            unwrap_packet(bytes([0xEE, 0x00]))

    def test_bad_packet_type(self):
        data = bytes(STATE_DATA_LEN)
        packet = bytearray(wrap_state_packet(data))
        packet[0] = 0xAA
        with pytest.raises(DecodeError, match="packet type"):
            unwrap_packet(bytes(packet))

    def test_checksum_is_wrapping_sum(self):
        assert checksum(bytes([0xFF, 0x01])) == 0x00
        assert checksum(bytes([0x80, 0x80])) == 0x00
        raw = bytes([0xEE, 0x00, 0x01, 0x00, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
        assert checksum(raw) == 163


# ── State decode/encode ───────────────────────────────────────────────

class TestState:
    def test_default_state_roundtrip(self):
        state = State(clock=Clock(remaining_ms=180_000))
        encoded = encode_state_data(state)
        assert encoded == bytes([
            0x00,           # flags
            0x01,           # match info (period=1)
            0x00, 180, 0x00,  # clock (180s)
            0x00,           # strip
            0x00,           # latched lights
            0x00, 0x00, 0x00,  # timings
            0x00, 0x00,     # scores
            0x00,           # cards
        ])
        decoded = decode_state_data(encoded)
        assert state == decoded

    def test_clock_centiseconds_mode(self):
        state = State(
            clock=Clock(running=True, remaining_ms=3420, passivity_ms=52_000),
        )
        encoded = encode_state_data(state)
        # 342 centiseconds, centiseconds_mode=1, running=1
        cs_raw = 342
        assert encoded[2] == ((1 << 3) | (1 << 2) | (cs_raw >> 8))
        assert encoded[3] == (cs_raw & 0xFF)
        assert encoded[4] == 52
        decoded = decode_state_data(encoded)
        assert state == decoded

    def test_score_with_last_changed(self):
        state = State(
            clock=Clock(remaining_ms=180_000),
            left_score=FencerScore(score=5, last_changed=True),
            right_score=FencerScore(score=11, last_changed=False),
        )
        encoded = encode_state_data(state)
        assert encoded[10] == (5 | 0x80)
        assert encoded[11] == 11
        decoded = decode_state_data(encoded)
        assert state == decoded

    def test_latched_lights_and_timing(self):
        state = State(
            clock=Clock(remaining_ms=180_000),
            left_light=LatchedLight.valid(713),
            right_light=LatchedLight.late(417),
        )
        encoded = encode_state_data(state)
        assert encoded[6:10] == bytes([
            0b_00_001_100,   # left=valid(1), right=late(4)
            0b_0_1011001,    # left time high 7 bits: 713 = 0b10_1100_1001 -> top 7 = 1011001
            0b_001_00_011,   # left low 3 = 001, reserved 00, right high 3 = 011
            0b_0100001_0,    # right low 7 = 0100001, reserved 0
        ])
        decoded = decode_state_data(encoded)
        assert state == decoded

    def test_hex_vector_1(self):
        data = bytes.fromhex("00120053200000000000028481")
        s = decode_state_data(data)
        assert s.weapon == Weapon.EPEE
        assert s.period == 2
        assert s.priority == Priority.NONE
        assert s.clock.running is False
        assert s.clock.remaining_ms == 83_000
        assert s.clock.passivity_ms == 32_000
        assert s.left_score.score == 2
        assert s.right_score.score == 4
        assert s.right_score.last_changed is True
        assert s.left_cards.card == Card.YELLOW
        assert s.right_cards.p_card == Card.RED
        assert encode_state_data(s) == data

    def test_hex_vector_2(self):
        data = bytes.fromhex("00530422190000000000040916")
        s = decode_state_data(data)
        assert s.weapon == Weapon.EPEE
        assert s.period == 3
        assert s.priority == Priority.LEFT
        assert s.clock.running is True
        assert s.clock.remaining_ms == 34_000
        assert s.clock.passivity_ms == 25_000
        assert s.left_score.score == 4
        assert s.right_score.score == 9
        assert s.left_cards.card == Card.RED
        assert s.left_cards.p_card == Card.YELLOW
        assert s.right_cards.card == Card.YELLOW
        assert encode_state_data(s) == data

    def test_hex_vector_3(self):
        data = bytes.fromhex("026100B4000C12436436040916")
        s = decode_state_data(data)
        assert s.weapon == Weapon.FOIL
        assert s.period == 1
        assert s.priority == Priority.LEFT
        assert s.clock.remaining_ms == 180_000
        assert s.lockout_started is True
        assert s.left_light == LatchedLight.nonvalid(539)
        assert s.right_light == LatchedLight.nonvalid(539)
        assert s.strip.left.fault is True
        assert s.strip.right.fault is True
        assert s.hide_extra_hits is False
        assert encode_state_data(s) == data

    def test_hex_vector_4(self):
        data = bytes.fromhex("000109FA1B0C00000000000000")
        s = decode_state_data(data)
        assert s.weapon == Weapon.SABRE
        assert s.period == 1
        assert s.clock.remaining_ms == 5060  # 506 centiseconds
        assert s.clock.passivity_ms == 27_000
        assert s.strip.left.fault is True
        assert s.strip.right.fault is True
        assert encode_state_data(s) == data

    def test_hide_extra_hits_flag(self):
        state = State(
            clock=Clock(remaining_ms=180_000),
            hide_extra_hits=True,
            left_light=LatchedLight.late(250),
            right_light=LatchedLight.whipover(50),
        )
        encoded = encode_state_data(state)
        decoded = decode_state_data(encoded)
        assert decoded.hide_extra_hits is True
        assert decoded.left_light == LatchedLight.late(250)
        assert decoded.right_light == LatchedLight.whipover(50)

        state.hide_extra_hits = False
        encoded = encode_state_data(state)
        decoded = decode_state_data(encoded)
        assert decoded.hide_extra_hits is False

    def test_cards_roundtrip(self):
        state = State(
            clock=Clock(remaining_ms=180_000),
            left_cards=FencerCards(card=Card.YELLOW, p_card=Card.RED),
            right_cards=FencerCards(card=Card.RED, p_card=Card.NONE),
        )
        encoded = encode_state_data(state)
        decoded = decode_state_data(encoded)
        assert decoded.left_cards == state.left_cards
        assert decoded.right_cards == state.right_cards

    def test_clock_expired_and_on_break(self):
        state = State(
            clock=Clock(expired=True, on_break=True),
        )
        encoded = encode_state_data(state)
        assert encoded[2] & 0b0011_0000 == 0b0011_0000
        decoded = decode_state_data(encoded)
        assert decoded.clock.expired is True
        assert decoded.clock.on_break is True

    def test_invalid_period_zero(self):
        data = bytearray(13)
        data[1] = 0x00  # period = 0
        with pytest.raises(DecodeError, match="period"):
            decode_state_data(bytes(data))

    def test_invalid_weapon(self):
        data = bytearray(13)
        data[1] = 0x31  # weapon bits = 0b11
        with pytest.raises(DecodeError, match="weapon"):
            decode_state_data(bytes(data))

    def test_invalid_priority(self):
        data = bytearray(13)
        data[1] = 0xC1  # priority bits = 0b11
        with pytest.raises(DecodeError, match="priority"):
            decode_state_data(bytes(data))

    def test_all_lights_variants_roundtrip(self):
        variants = [
            (LatchedLight.off(), LatchedLight.off()),
            (LatchedLight.valid(0), LatchedLight.valid(999)),
            (LatchedLight.nonvalid(500), LatchedLight.late(100)),
            (LatchedLight.whipover(15), LatchedLight.nonvalid(0)),
            (LatchedLight.late(713), LatchedLight.whipover(417)),
        ]
        for left, right in variants:
            state = State(
                clock=Clock(remaining_ms=180_000),
                left_light=left,
                right_light=right,
            )
            decoded = decode_state_data(encode_state_data(state))
            assert decoded.left_light == left
            assert decoded.right_light == right

    def test_strip_input_roundtrip(self):
        state = State(
            clock=Clock(remaining_ms=180_000),
            strip=StripInput(
                blade_contact=True,
                left=FencerStripInput(valid=True, fault=False, short=True),
                right=FencerStripInput(valid=False, fault=True, short=False),
            ),
        )
        decoded = decode_state_data(encode_state_data(state))
        assert decoded.strip == state.strip


# ── Event decode/encode ───────────────────────────────────────────────

class TestEvent:
    def test_clock_start_stop_roundtrip(self):
        data = encode_event_data(Event.clock_start_stop(), 13)
        assert data == bytes([0x22, 0, 13])
        decoded = decode_event_data(data)
        assert decoded.event == Event.clock_start_stop()
        assert decoded.dropped_count == 13

    def test_clock_adj_sec_positive(self):
        data = encode_event_data(Event.clock_adj_sec(1))
        assert data == bytes([0x24, 1, 0])
        decoded = decode_event_data(data)
        assert decoded.event == Event.clock_adj_sec(1)

    def test_clock_adj_sec_negative(self):
        data = encode_event_data(Event.clock_adj_sec(-1))
        assert data == bytes([0x24, 255, 0])
        decoded = decode_event_data(data)
        assert decoded.event == Event.clock_adj_sec(-1)

    def test_all_events_roundtrip(self):
        events = [
            Event.set_weapon(Weapon.SABRE),
            Event.set_weapon(Weapon.EPEE),
            Event.set_weapon(Weapon.FOIL),
            Event.enter_menu(),
            Event.menu_key(MenuKey.UP),
            Event.menu_key(MenuKey.DOWN),
            Event.menu_key(MenuKey.LEFT),
            Event.menu_key(MenuKey.RIGHT),
            Event.menu_key(MenuKey.SELECT),
            Event.menu_key(MenuKey.EXIT),
            Event.menu_key(MenuKey.FUNC),
            Event.menu_key(MenuKey.OTHER),
            Event.sleep_now(),
            Event.set_remote_addr(42),
            Event.remote_battery_level(85),
            Event.clear_scores(),
            Event.score_up(Side.LEFT),
            Event.score_up(Side.RIGHT),
            Event.score_up(Side.BOTH),
            Event.score_down(Side.LEFT),
            Event.cycle_card(Side.RIGHT),
            Event.cycle_p_card(Side.BOTH),
            Event.cycle_priority(),
            Event.clock_reset(),
            Event.clock_enter_time(),
            Event.clock_start_stop(),
            Event.clock_start_break(),
            Event.clock_adj_sec(5),
            Event.clock_adj_sec(-3),
            Event.clock_adj_period(1),
            Event.clock_adj_period(-1),
            Event.review_timeline_back(),
            Event.undo(),
            Event.review_timeline_fwd(),
            Event.func(),
            Event.touche_occurred(),
        ]
        for event in events:
            data = encode_event_data(event)
            decoded = decode_event_data(data)
            assert decoded.event == event, f"roundtrip failed for {event}"

    def test_invalid_event_id(self):
        with pytest.raises(DecodeError, match="event ID"):
            decode_event_data(bytes([0xFF, 0x00, 0x00]))

    def test_invalid_weapon_in_event(self):
        with pytest.raises(DecodeError, match="event data"):
            decode_event_data(bytes([0x01, 0x00, 0x00]))

    def test_invalid_side_in_event(self):
        with pytest.raises(DecodeError, match="event data"):
            decode_event_data(bytes([0x11, 0x00, 0x00]))

    def test_dropped_count(self):
        data = encode_event_data(Event.clock_start_stop(), 250)
        decoded = decode_event_data(data)
        assert decoded.dropped_count == 250


# ── Convenience API ───────────────────────────────────────────────────

class TestConvenience:
    def test_decode_state_packet(self):
        state = State(clock=Clock(remaining_ms=180_000))
        packet = encode_state_packet(state)
        assert len(packet) == STATE_PACKET_LEN
        decoded = decode_packet(packet)
        assert isinstance(decoded, State)
        assert decoded == state

    def test_decode_event_packet(self):
        event = Event.clock_start_stop()
        packet = encode_event_packet(event, 5)
        assert len(packet) == EVENT_PACKET_LEN
        decoded = decode_packet(packet)
        assert isinstance(decoded, EventPacket)
        assert decoded.event == event
        assert decoded.dropped_count == 5


# ── Packetizer ────────────────────────────────────────────────────────

class TestPacketizer:
    def test_byte_by_byte_state_packet(self):
        state = State(clock=Clock(remaining_ms=180_000))
        packet = encode_state_packet(state)
        p = Packetizer()
        for b in packet[:-1]:
            assert p.feed(b) is None
        result = p.feed(packet[-1])
        assert isinstance(result, State)
        assert result == state

    def test_byte_by_byte_event_packet(self):
        event = Event.clock_start_stop()
        packet = encode_event_packet(event, 13)
        p = Packetizer()
        for b in packet[:-1]:
            assert p.feed(b) is None
        result = p.feed(packet[-1])
        assert isinstance(result, EventPacket)
        assert result.event == event
        assert result.dropped_count == 13

    def test_back_to_back_packets(self):
        state = State(clock=Clock(remaining_ms=180_000))
        event = Event.clock_start_stop()
        stream = encode_state_packet(state) + encode_event_packet(event)

        p = Packetizer()
        results = p.feed_bytes(stream)
        assert len(results) == 2
        assert isinstance(results[0], State)
        assert results[0] == state
        assert isinstance(results[1], EventPacket)
        assert results[1].event == event

    def test_garbage_before_packet_resync(self):
        state = State(clock=Clock(remaining_ms=180_000))
        packet = encode_state_packet(state)
        stream = bytes([0xAA, 0xBB, 0xCC]) + packet

        p = Packetizer()
        results = p.feed_bytes(stream)
        assert len(results) == 1
        assert isinstance(results[0], State)
        assert results[0] == state

    def test_corrupt_checksum_silently_dropped(self):
        state = State(clock=Clock(remaining_ms=180_000))
        packet = bytearray(encode_state_packet(state))
        packet[14] ^= 0x01  # corrupt checksum

        p = Packetizer()
        for b in packet:
            # Corrupt packet is silently dropped -- no result returned
            assert p.feed(b) is None

        # Packetizer recovers and decodes the next valid packet
        good_packet = encode_state_packet(state)
        results = p.feed_bytes(good_packet)
        assert len(results) == 1
        assert isinstance(results[0], State)
        assert results[0] == state

    def test_false_terminator_0xff_checksum(self):
        # Construct a state packet whose checksum is 0xFF.
        # We need checksum(TYPE || DATA) == 0xFF.
        # checksum([0xEE, d0..d12]) = 0xEE + sum(d0..d12) (wrapping)
        # We need 0xEE + sum(data) == 0xFF, so sum(data) == 0x11
        # Data must also be valid state (period >= 1), so we use:
        #   data[1] = 0x01 (period=1), data[3] = 0x10 (16s remaining)
        #   sum = 0x01 + 0x10 = 0x11
        data = bytearray(STATE_DATA_LEN)
        data[1] = 0x01  # match info: period=1, weapon=sabre, priority=none
        data[3] = 0x10  # clock remaining low byte = 16 seconds
        wrapped = wrap_state_packet(bytes(data))
        assert wrapped[14] == 0xFF, "checksum should be 0xFF"

        # The packet has two consecutive 0xFF bytes: checksum and terminator.
        # The packetizer should handle the false terminator (checksum) and
        # still decode the packet when the real terminator arrives.
        p = Packetizer()
        result = None
        for b in wrapped:
            r = p.feed(b)
            if r is not None:
                result = r
        assert result is not None, "should decode packet with 0xFF checksum"
        assert isinstance(result, State)

    def test_lone_terminator_ignored(self):
        p = Packetizer()
        # A lone 0xFF can't form a valid packet -- silently ignored
        assert p.feed(0xFF) is None

    def test_feed_bytes_api(self):
        state = State(clock=Clock(remaining_ms=180_000))
        event = Event.clock_start_stop()
        stream = encode_state_packet(state) + encode_event_packet(event)
        p = Packetizer()
        results = p.feed_bytes(stream)
        assert len(results) == 2

    def test_reset_clears_buffer(self):
        p = Packetizer()
        p.feed(0xEE)
        p.feed(0x00)
        p.reset()

        event = Event.clock_start_stop()
        packet = encode_event_packet(event)
        results = p.feed_bytes(packet)
        assert len(results) == 1
        assert isinstance(results[0], EventPacket)
