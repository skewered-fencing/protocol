from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum


class DecodeError(ValueError):
    """Raised when a packet cannot be decoded."""


class InvalidPacket:
    """Sentinel returned by the Packetizer when a terminator was seen but did
    not form a valid packet (corruption, or rarely a false terminator when the
    checksum equals 0xFF)."""

    _instance: InvalidPacket | None = None

    def __new__(cls) -> InvalidPacket:
        if cls._instance is None:
            cls._instance = super().__new__(cls)
        return cls._instance

    def __repr__(self) -> str:
        return "InvalidPacket"

    def __bool__(self) -> bool:
        return True


class Weapon(Enum):
    SABRE = "sabre"
    EPEE = "epee"
    FOIL = "foil"


class Side(Enum):
    LEFT = "left"
    RIGHT = "right"
    BOTH = "both"


class Priority(Enum):
    NONE = "none"
    LEFT = "left"
    RIGHT = "right"


class Card(Enum):
    NONE = "none"
    YELLOW = "yellow"
    RED = "red"


class MenuKey(Enum):
    OTHER = "other"
    UP = "up"
    DOWN = "down"
    LEFT = "left"
    RIGHT = "right"
    SELECT = "select"
    EXIT = "exit"
    FUNC = "func"


@dataclass(slots=True)
class FencerCards:
    card: Card = Card.NONE
    p_card: Card = Card.NONE


@dataclass(slots=True)
class FencerScore:
    score: int = 0
    last_changed: bool = False


@dataclass(slots=True)
class FencerStripInput:
    valid: bool = False
    fault: bool = False
    short: bool = False


@dataclass(slots=True)
class StripInput:
    blade_contact: bool = False
    left: FencerStripInput = field(default_factory=FencerStripInput)
    right: FencerStripInput = field(default_factory=FencerStripInput)


@dataclass(slots=True)
class Clock:
    running: bool = False
    expired: bool = False
    on_break: bool = False
    remaining_ms: int = 0
    passivity_ms: int = 0


@dataclass(slots=True)
class LatchedLight:
    """Latched light state for one fencer.

    kind: "off", "valid", "nonvalid", "whipover", "late"
    timing_ms: milliseconds, meaning depends on kind:
      - off: always 0
      - valid: time since the hit occurred (capped at 999ms on wire)
      - nonvalid: time since the hit occurred (capped at 999ms on wire)
      - whipover: duration of the short/whipover contact
      - late: time of the hit since lockout started
    """
    kind: str = "off"
    timing_ms: int = 0

    @classmethod
    def off(cls) -> LatchedLight:
        return cls(kind="off", timing_ms=0)

    @classmethod
    def valid(cls, ms: int) -> LatchedLight:
        return cls(kind="valid", timing_ms=ms)

    @classmethod
    def nonvalid(cls, ms: int) -> LatchedLight:
        return cls(kind="nonvalid", timing_ms=ms)

    @classmethod
    def whipover(cls, ms: int) -> LatchedLight:
        return cls(kind="whipover", timing_ms=ms)

    @classmethod
    def late(cls, ms: int) -> LatchedLight:
        return cls(kind="late", timing_ms=ms)


@dataclass(slots=True)
class State:
    # Config flags (byte 0)
    sleeping: bool = False
    lockout_started: bool = False
    live_video_view: bool = False
    reviewing: bool = False

    # Match info (byte 1)
    weapon: Weapon = Weapon.SABRE
    priority: Priority = Priority.NONE
    period: int = 1

    # Clock info (bytes 2-4)
    clock: Clock = field(default_factory=Clock)

    # Strip input (byte 5)
    strip: StripInput = field(default_factory=StripInput)

    # Latched lights (bytes 6-9)
    left_light: LatchedLight = field(default_factory=LatchedLight.off)
    right_light: LatchedLight = field(default_factory=LatchedLight.off)
    hide_extra_hits: bool = False

    # Scores (bytes 10-11)
    left_score: FencerScore = field(default_factory=FencerScore)
    right_score: FencerScore = field(default_factory=FencerScore)

    # Cards (byte 12)
    left_cards: FencerCards = field(default_factory=FencerCards)
    right_cards: FencerCards = field(default_factory=FencerCards)


@dataclass(slots=True)
class Event:
    """A flat event with a kind string and optional data.

    kind: snake_case variant name matching Rust Event enum
    data: variant-specific payload (Weapon, MenuKey, Side, int, or None)
    """
    kind: str
    data: Weapon | MenuKey | Side | int | None = None

    @classmethod
    def set_weapon(cls, weapon: Weapon) -> Event:
        return cls(kind="set_weapon", data=weapon)

    @classmethod
    def enter_menu(cls) -> Event:
        return cls(kind="enter_menu")

    @classmethod
    def menu_key(cls, key: MenuKey) -> Event:
        return cls(kind="menu_key", data=key)

    @classmethod
    def sleep_now(cls) -> Event:
        return cls(kind="sleep_now")

    @classmethod
    def set_remote_addr(cls, addr: int) -> Event:
        return cls(kind="set_remote_addr", data=addr)

    @classmethod
    def remote_battery_level(cls, level: int) -> Event:
        return cls(kind="remote_battery_level", data=level)

    @classmethod
    def clear_scores(cls) -> Event:
        return cls(kind="clear_scores")

    @classmethod
    def score_up(cls, side: Side) -> Event:
        return cls(kind="score_up", data=side)

    @classmethod
    def score_down(cls, side: Side) -> Event:
        return cls(kind="score_down", data=side)

    @classmethod
    def cycle_card(cls, side: Side) -> Event:
        return cls(kind="cycle_card", data=side)

    @classmethod
    def cycle_p_card(cls, side: Side) -> Event:
        return cls(kind="cycle_p_card", data=side)

    @classmethod
    def cycle_priority(cls) -> Event:
        return cls(kind="cycle_priority")

    @classmethod
    def clock_reset(cls) -> Event:
        return cls(kind="clock_reset")

    @classmethod
    def clock_enter_time(cls) -> Event:
        return cls(kind="clock_enter_time")

    @classmethod
    def clock_start_stop(cls) -> Event:
        return cls(kind="clock_start_stop")

    @classmethod
    def clock_start_break(cls) -> Event:
        return cls(kind="clock_start_break")

    @classmethod
    def clock_adj_sec(cls, amount: int) -> Event:
        return cls(kind="clock_adj_sec", data=amount)

    @classmethod
    def clock_adj_period(cls, amount: int) -> Event:
        return cls(kind="clock_adj_period", data=amount)

    @classmethod
    def review_timeline_back(cls) -> Event:
        return cls(kind="review_timeline_back")

    @classmethod
    def undo(cls) -> Event:
        return cls(kind="undo")

    @classmethod
    def review_timeline_fwd(cls) -> Event:
        return cls(kind="review_timeline_fwd")

    @classmethod
    def func(cls) -> Event:
        return cls(kind="func")

    @classmethod
    def touche_occurred(cls) -> Event:
        return cls(kind="touche_occurred")


@dataclass(slots=True)
class EventPacket:
    event: Event
    dropped_count: int = 0
