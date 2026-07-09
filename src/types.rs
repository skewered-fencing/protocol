#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weapon {
    Sabre,
    Epee,
    Foil,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    None,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Card {
    None,
    Yellow,
    Red,
}

/// Latched light state for one fencer.
///
/// The `Millis` value meaning depends on the variant:
/// - `Valid` / `NonValid`: time since the hit occurred (capped at 999ms on
///   wire)
/// - `Whipover`: duration of the short/whipover hit
/// - `Late`: time of the hit since lockout started
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatchedLight {
    Off,
    Valid(Millis),
    NonValid(Millis),
    Whipover(Millis),
    Late(Millis),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FencerCards {
    pub card: Card,
    pub p_card: Card,
}

impl Default for FencerCards {
    fn default() -> Self {
        Self {
            card: Card::None,
            p_card: Card::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FencerScore {
    pub score: u8,
    pub last_changed: bool,
}

impl Default for FencerScore {
    fn default() -> Self {
        Self {
            score: 0,
            last_changed: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FencerStripInput {
    pub valid: bool,
    pub fault: bool,
    pub short: bool,
}

impl Default for FencerStripInput {
    fn default() -> Self {
        Self {
            valid: false,
            fault: false,
            short: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StripInput {
    pub blade_contact: bool,
    pub left: FencerStripInput,
    pub right: FencerStripInput,
}

impl Default for StripInput {
    fn default() -> Self {
        Self {
            blade_contact: false,
            left: FencerStripInput::default(),
            right: FencerStripInput::default(),
        }
    }
}

/// Clock state. All durations are decoded to milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    pub running: bool,
    pub expired: bool,
    pub on_break: bool,
    pub remaining: Millis,
    pub passivity: Millis,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            running: false,
            expired: false,
            on_break: false,
            remaining: Millis::ZERO,
            passivity: Millis::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKey {
    Other,
    Up,
    Down,
    Left,
    Right,
    Select,
    Exit,
    Func,
}

/// A physical button on the IR handheld remote, identified by its NEC data
/// code (the discriminant). Sent via [`crate::Event::RemoteKey`] so a
/// controller can emulate an exact remote keypress: the receiver runs the
/// press through its normal IR pipeline, inheriting all mode-dependent
/// behavior (menu navigation, weapon show-then-advance, running-clock guards,
/// numeric time entry) instead of a fixed semantic action.
///
/// Battery-level report frames (NEC codes 128–148) are deliberately not
/// representable: they are telemetry from the physical remote, not buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RemoteKey {
    LeftScoreUp = 68,
    LeftCard = 7,
    LeftPCard = 17,
    LeftScoreDown = 22,
    RightScoreUp = 67,
    RightCard = 9,
    RightPCard = 19,
    RightScoreDown = 13,
    Clear = 28,

    TimeStartStop = 69,
    TimeOneMinPause = 71,
    TimeAdjUp = 90,
    TimeAdjDown = 8,
    TimeSet = 21,
    TimeSetCustom = 31,
    TimePeriodUp = 102,
    TimePeriodDown = 103,

    Priority = 72,
    TimelineBack = 70,
    Undo = 73,
    NextWeapon = 25,
    TimelineForward = 64,
    Func = 74,

    Configure = 100,
    Sleep = 101,
}

impl RemoteKey {
    /// Every remote button, for iteration and code lookup.
    pub const ALL: [RemoteKey; 25] = [
        RemoteKey::LeftScoreUp,
        RemoteKey::LeftCard,
        RemoteKey::LeftPCard,
        RemoteKey::LeftScoreDown,
        RemoteKey::RightScoreUp,
        RemoteKey::RightCard,
        RemoteKey::RightPCard,
        RemoteKey::RightScoreDown,
        RemoteKey::Clear,
        RemoteKey::TimeStartStop,
        RemoteKey::TimeOneMinPause,
        RemoteKey::TimeAdjUp,
        RemoteKey::TimeAdjDown,
        RemoteKey::TimeSet,
        RemoteKey::TimeSetCustom,
        RemoteKey::TimePeriodUp,
        RemoteKey::TimePeriodDown,
        RemoteKey::Priority,
        RemoteKey::TimelineBack,
        RemoteKey::Undo,
        RemoteKey::NextWeapon,
        RemoteKey::TimelineForward,
        RemoteKey::Func,
        RemoteKey::Configure,
        RemoteKey::Sleep,
    ];

    /// The NEC data code transmitted for this button.
    pub fn code(self) -> u8 {
        self as u8
    }

    /// Looks up a button by its NEC data code. Returns `None` for codes that
    /// are not remote buttons (including battery-level frames).
    pub fn from_code(code: u8) -> Option<RemoteKey> {
        RemoteKey::ALL.iter().copied().find(|k| k.code() == code)
    }
}

/// A duration in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Millis(pub u32);

impl Millis {
    pub const ZERO: Millis = Millis(0);

    pub fn from_secs(s: u32) -> Self {
        Millis(s * 1000)
    }

    pub fn as_millis(self) -> u32 {
        self.0
    }

    pub fn as_secs(self) -> u32 {
        self.0 / 1000
    }
}

impl Default for Millis {
    fn default() -> Self {
        Self::ZERO
    }
}
