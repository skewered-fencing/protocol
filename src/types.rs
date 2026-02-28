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
