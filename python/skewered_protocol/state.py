from __future__ import annotations

from .types import (
    Card, Clock, DecodeError, FencerCards, FencerScore, FencerStripInput,
    LatchedLight, Priority, State, StripInput, Weapon,
)


def _is_bit_set(val: int, bit: int) -> bool:
    return (val & (1 << bit)) != 0


def _bit_if(condition: bool, bit: int) -> int:
    return (1 << bit) if condition else 0


def _decode_card(bits: int) -> Card:
    if bits == 0b00:
        return Card.NONE
    elif bits == 0b01:
        return Card.YELLOW
    elif bits == 0b10:
        return Card.RED
    else:
        raise DecodeError("invalid card value")


def _encode_card(card: Card) -> int:
    if card == Card.NONE:
        return 0b00
    elif card == Card.YELLOW:
        return 0b01
    elif card == Card.RED:
        return 0b10
    raise DecodeError("invalid card")  # pragma: no cover


def _decode_latched_light(flags: int, time: int) -> LatchedLight:
    if flags == 0:
        return LatchedLight.off()
    elif flags == 1:
        return LatchedLight.valid(time)
    elif flags == 2:
        return LatchedLight.nonvalid(time)
    elif flags == 3:
        return LatchedLight.whipover(time)
    elif flags == 4:
        return LatchedLight.late(time)
    else:
        raise DecodeError("invalid latched light value")


def _encode_latched_light(light: LatchedLight) -> tuple[int, int]:
    kind = light.kind
    t = light.timing_ms
    if kind == "off":
        return (0, 0)
    elif kind == "valid":
        return (1, t)
    elif kind == "nonvalid":
        return (2, t)
    elif kind == "whipover":
        return (3, t)
    elif kind == "late":
        return (4, t)
    raise DecodeError("invalid latched light kind")  # pragma: no cover


def decode_state_data(data: bytes | bytearray) -> State:
    """Decodes 13 raw data bytes into a State."""
    if len(data) != 13:
        raise DecodeError(f"expected 13 data bytes, got {len(data)}")

    # Byte 0: config flags
    sleeping = _is_bit_set(data[0], 0)
    lockout_started = _is_bit_set(data[0], 1)
    live_video_view = _is_bit_set(data[0], 2)
    reviewing = _is_bit_set(data[0], 3)

    # Byte 1: match info
    priority_bits = data[1] >> 6
    weapon_bits = (data[1] >> 4) & 0b11
    period = data[1] & 0x0F

    if priority_bits == 0b00:
        priority = Priority.NONE
    elif priority_bits == 0b01:
        priority = Priority.LEFT
    elif priority_bits == 0b10:
        priority = Priority.RIGHT
    else:
        raise DecodeError("invalid priority value")

    if weapon_bits == 0b00:
        weapon = Weapon.SABRE
    elif weapon_bits == 0b01:
        weapon = Weapon.EPEE
    elif weapon_bits == 0b10:
        weapon = Weapon.FOIL
    else:
        raise DecodeError("invalid weapon value")

    if period == 0:
        raise DecodeError("invalid period value")

    # Bytes 2-4: clock info
    running = _is_bit_set(data[2], 2)
    centiseconds_mode = _is_bit_set(data[2], 3)
    on_break = _is_bit_set(data[2], 4)
    expired = _is_bit_set(data[2], 5)
    raw_remaining = ((data[2] & 0b11) << 8) | data[3]
    if centiseconds_mode:
        remaining_ms = raw_remaining * 10
    else:
        remaining_ms = raw_remaining * 1000
    passivity_ms = data[4] * 1000

    clock = Clock(
        running=running,
        expired=expired,
        on_break=on_break,
        remaining_ms=remaining_ms,
        passivity_ms=passivity_ms,
    )

    # Byte 5: raw strip input
    strip = StripInput(
        blade_contact=_is_bit_set(data[5], 6),
        left=FencerStripInput(
            valid=_is_bit_set(data[5], 1),
            fault=_is_bit_set(data[5], 3),
            short=_is_bit_set(data[5], 5),
        ),
        right=FencerStripInput(
            valid=_is_bit_set(data[5], 0),
            fault=_is_bit_set(data[5], 2),
            short=_is_bit_set(data[5], 4),
        ),
    )

    # Bytes 6-9: latched lights + timing
    hide_extra_hits = _is_bit_set(data[6], 6)
    lflags = (data[6] >> 3) & 0b111
    rflags = data[6] & 0b111
    ltime = (data[7] << 3) | (data[8] >> 5)
    rtime = ((data[8] & 0b0_00_111) << 7) | (data[9] >> 1)

    left_light = _decode_latched_light(lflags, ltime)
    right_light = _decode_latched_light(rflags, rtime)

    # Bytes 10-11: scores
    left_score = FencerScore(
        score=data[10] & 0x7F,
        last_changed=_is_bit_set(data[10], 7),
    )
    right_score = FencerScore(
        score=data[11] & 0x7F,
        last_changed=_is_bit_set(data[11], 7),
    )

    # Byte 12: cards
    left_cards = FencerCards(
        card=_decode_card((data[12] >> 0) & 0b11),
        p_card=_decode_card((data[12] >> 2) & 0b11),
    )
    right_cards = FencerCards(
        card=_decode_card((data[12] >> 4) & 0b11),
        p_card=_decode_card((data[12] >> 6) & 0b11),
    )

    return State(
        sleeping=sleeping,
        lockout_started=lockout_started,
        live_video_view=live_video_view,
        reviewing=reviewing,
        weapon=weapon,
        priority=priority,
        period=period,
        clock=clock,
        strip=strip,
        left_light=left_light,
        right_light=right_light,
        hide_extra_hits=hide_extra_hits,
        left_score=left_score,
        right_score=right_score,
        left_cards=left_cards,
        right_cards=right_cards,
    )


def encode_state_data(state: State) -> bytes:
    """Encodes a State into 13 raw data bytes."""
    data = bytearray(13)

    # Byte 0: config flags
    data[0] = (
        _bit_if(state.sleeping, 0)
        | _bit_if(state.lockout_started, 1)
        | _bit_if(state.live_video_view, 2)
        | _bit_if(state.reviewing, 3)
    )

    # Byte 1: match info
    priority_bits = {
        Priority.NONE: 0b00,
        Priority.LEFT: 0b01,
        Priority.RIGHT: 0b10,
    }[state.priority]
    weapon_bits = {
        Weapon.SABRE: 0b00,
        Weapon.EPEE: 0b01,
        Weapon.FOIL: 0b10,
    }[state.weapon]
    data[1] = (priority_bits << 6) | (weapon_bits << 4) | state.period

    # Bytes 2-4: clock info
    ms = state.clock.remaining_ms
    centiseconds_mode = ms < 10_000
    if centiseconds_mode:
        raw_remaining = (ms // 10) & 0x3FF
    else:
        raw_remaining = (ms // 1000) & 0x3FF
    data[2] = (
        _bit_if(state.clock.expired, 5)
        | _bit_if(state.clock.on_break, 4)
        | _bit_if(centiseconds_mode, 3)
        | _bit_if(state.clock.running, 2)
        | (raw_remaining >> 8)
    )
    data[3] = raw_remaining & 0xFF
    passivity_secs = state.clock.passivity_ms // 1000
    data[4] = min(passivity_secs, 99)

    # Byte 5: raw strip input
    data[5] = (
        _bit_if(state.strip.blade_contact, 6)
        | _bit_if(state.strip.left.short, 5)
        | _bit_if(state.strip.right.short, 4)
        | _bit_if(state.strip.left.fault, 3)
        | _bit_if(state.strip.right.fault, 2)
        | _bit_if(state.strip.left.valid, 1)
        | _bit_if(state.strip.right.valid, 0)
    )

    # Bytes 6-9: latched lights + timing
    lflags, ltime = _encode_latched_light(state.left_light)
    rflags, rtime = _encode_latched_light(state.right_light)

    data[6] = _bit_if(state.hide_extra_hits, 6) | (lflags << 3) | rflags
    data[7] = (ltime >> 3) & 0b0111_1111
    data[8] = ((ltime & 0b111) << 5) | ((rtime >> 7) & 0b111)
    data[9] = (rtime << 1) & 0b1111_1110

    # Bytes 10-11: scores
    data[10] = state.left_score.score | _bit_if(state.left_score.last_changed, 7)
    data[11] = state.right_score.score | _bit_if(state.right_score.last_changed, 7)

    # Byte 12: cards
    data[12] = (
        _encode_card(state.left_cards.card)
        | (_encode_card(state.left_cards.p_card) << 2)
        | (_encode_card(state.right_cards.card) << 4)
        | (_encode_card(state.right_cards.p_card) << 6)
    )

    return bytes(data)
