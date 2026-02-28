from __future__ import annotations

from .types import DecodeError, Event, EventPacket, MenuKey, Side, Weapon


def _decode_side(byte: int) -> Side:
    if byte == 0x01:
        return Side.LEFT
    elif byte == 0x02:
        return Side.RIGHT
    elif byte == 0x03:
        return Side.BOTH
    else:
        raise DecodeError("invalid event data")


def _encode_side(side: Side) -> int:
    if side == Side.LEFT:
        return 0x01
    elif side == Side.RIGHT:
        return 0x02
    elif side == Side.BOTH:
        return 0x03
    raise DecodeError("invalid side")  # pragma: no cover


# Event ID -> kind string, and the reverse mapping
_EVENT_ID_TO_KIND: dict[int, str] = {
    0x01: "set_weapon",
    0x02: "enter_menu",
    0x03: "menu_key",
    0x04: "sleep_now",
    0x05: "set_remote_addr",
    0x06: "remote_battery_level",
    0x10: "clear_scores",
    0x11: "score_up",
    0x12: "score_down",
    0x13: "cycle_card",
    0x14: "cycle_p_card",
    0x15: "cycle_priority",
    0x20: "clock_reset",
    0x21: "clock_enter_time",
    0x22: "clock_start_stop",
    0x23: "clock_start_break",
    0x24: "clock_adj_sec",
    0x25: "clock_adj_period",
    0x30: "review_timeline_back",
    0x31: "undo",
    0x32: "review_timeline_fwd",
    0x33: "func",
    0x34: "touche_occurred",
}

_KIND_TO_EVENT_ID: dict[str, int] = {v: k for k, v in _EVENT_ID_TO_KIND.items()}

# Events that carry a Side argument
_SIDE_EVENTS = {"score_up", "score_down", "cycle_card", "cycle_p_card"}

# Events with no extra data (extra byte = 0)
_NO_DATA_EVENTS = {
    "enter_menu", "sleep_now", "clear_scores", "cycle_priority",
    "clock_reset", "clock_enter_time", "clock_start_stop", "clock_start_break",
    "review_timeline_back", "undo", "review_timeline_fwd", "func", "touche_occurred",
}


def decode_event_data(data: bytes | bytearray) -> EventPacket:
    """Decodes 3 raw data bytes into an EventPacket."""
    if len(data) != 3:
        raise DecodeError(f"expected 3 data bytes, got {len(data)}")

    event_id = data[0]
    extra = data[1]
    dropped_count = data[2]

    kind = _EVENT_ID_TO_KIND.get(event_id)
    if kind is None:
        raise DecodeError("invalid event ID")

    if kind == "set_weapon":
        if extra == 0x01:
            event_data = Weapon.SABRE
        elif extra == 0x02:
            event_data = Weapon.EPEE
        elif extra == 0x03:
            event_data = Weapon.FOIL
        else:
            raise DecodeError("invalid event data")
        event = Event(kind=kind, data=event_data)
    elif kind == "menu_key":
        _MENU_KEY_MAP = {
            0x00: MenuKey.OTHER,
            0x01: MenuKey.UP,
            0x02: MenuKey.DOWN,
            0x03: MenuKey.LEFT,
            0x04: MenuKey.RIGHT,
            0x05: MenuKey.SELECT,
            0x06: MenuKey.EXIT,
            0x07: MenuKey.FUNC,
        }
        key = _MENU_KEY_MAP.get(extra)
        if key is None:
            raise DecodeError("invalid event data")
        event = Event(kind=kind, data=key)
    elif kind in _SIDE_EVENTS:
        event = Event(kind=kind, data=_decode_side(extra))
    elif kind == "set_remote_addr":
        event = Event(kind=kind, data=extra)
    elif kind == "remote_battery_level":
        event = Event(kind=kind, data=extra)
    elif kind == "clock_adj_sec":
        # Interpret as signed i8
        signed = extra if extra < 128 else extra - 256
        event = Event(kind=kind, data=signed)
    elif kind == "clock_adj_period":
        signed = extra if extra < 128 else extra - 256
        event = Event(kind=kind, data=signed)
    elif kind in _NO_DATA_EVENTS:
        event = Event(kind=kind)
    else:
        raise DecodeError("invalid event ID")  # pragma: no cover

    return EventPacket(event=event, dropped_count=dropped_count)


def encode_event_data(event: Event, dropped_count: int = 0) -> bytes:
    """Encodes an Event and dropped count into 3 raw data bytes."""
    event_id = _KIND_TO_EVENT_ID.get(event.kind)
    if event_id is None:
        raise DecodeError(f"unknown event kind: {event.kind}")

    extra = 0

    if event.kind == "set_weapon":
        assert isinstance(event.data, Weapon)
        extra = {Weapon.SABRE: 0x01, Weapon.EPEE: 0x02, Weapon.FOIL: 0x03}[event.data]
    elif event.kind == "menu_key":
        assert isinstance(event.data, MenuKey)
        extra = {
            MenuKey.OTHER: 0x00, MenuKey.UP: 0x01, MenuKey.DOWN: 0x02,
            MenuKey.LEFT: 0x03, MenuKey.RIGHT: 0x04, MenuKey.SELECT: 0x05,
            MenuKey.EXIT: 0x06, MenuKey.FUNC: 0x07,
        }[event.data]
    elif event.kind in _SIDE_EVENTS:
        assert isinstance(event.data, Side)
        extra = _encode_side(event.data)
    elif event.kind in ("set_remote_addr", "remote_battery_level"):
        assert isinstance(event.data, int)
        extra = event.data
    elif event.kind in ("clock_adj_sec", "clock_adj_period"):
        assert isinstance(event.data, int)
        extra = event.data & 0xFF

    return bytes([event_id, extra, dropped_count])
