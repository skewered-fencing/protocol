use crate::error::DecodeError;
use crate::types::*;

/// Full decoded state from a 13-byte state data packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    // Config flags (byte 0)
    pub sleeping: bool,
    pub lockout_started: bool,
    pub live_video_view: bool,
    pub reviewing: bool,

    // Match info (byte 1)
    pub weapon: Weapon,
    pub priority: Priority,
    pub period: u8,

    // Clock info (bytes 2-4)
    pub clock: Clock,

    // Strip input (byte 5)
    pub strip: StripInput,

    // Latched lights (bytes 6-9)
    pub left_light: LatchedLight,
    pub right_light: LatchedLight,
    pub hide_extra_hits: bool,

    // Scores (bytes 10-11)
    pub left_score: FencerScore,
    pub right_score: FencerScore,

    // Cards (byte 12)
    pub left_cards: FencerCards,
    pub right_cards: FencerCards,
}

impl Default for State {
    fn default() -> Self {
        Self {
            sleeping: false,
            lockout_started: false,
            live_video_view: false,
            reviewing: false,
            weapon: Weapon::Sabre,
            priority: Priority::None,
            period: 1,
            clock: Clock::default(),
            strip: StripInput::default(),
            left_light: LatchedLight::Off,
            right_light: LatchedLight::Off,
            hide_extra_hits: false,
            left_score: FencerScore::default(),
            right_score: FencerScore::default(),
            left_cards: FencerCards::default(),
            right_cards: FencerCards::default(),
        }
    }
}

fn is_bit_set(val: u8, bit: u8) -> bool {
    val & (1 << bit) != 0
}

fn bit_if(condition: bool, bit: u8) -> u8 {
    if condition { 1 << bit } else { 0 }
}

/// Decodes 13 raw data bytes into a `State`.
pub fn decode_state_data(data: &[u8; 13]) -> Result<State, DecodeError> {
    let mut s = State::default();

    // Byte 0: config flags
    s.sleeping = is_bit_set(data[0], 0);
    s.lockout_started = is_bit_set(data[0], 1);
    s.live_video_view = is_bit_set(data[0], 2);
    s.reviewing = is_bit_set(data[0], 3);

    // Byte 1: match info
    let priority_bits = data[1] >> 6;
    let weapon_bits = (data[1] >> 4) & 0b11;
    let period = data[1] & 0x0F;

    s.priority = match priority_bits {
        0b00 => Priority::None,
        0b01 => Priority::Left,
        0b10 => Priority::Right,
        _ => return Err(DecodeError::InvalidPriority),
    };
    s.weapon = match weapon_bits {
        0b00 => Weapon::Sabre,
        0b01 => Weapon::Epee,
        0b10 => Weapon::Foil,
        _ => return Err(DecodeError::InvalidWeapon),
    };
    if period == 0 {
        return Err(DecodeError::InvalidPeriod);
    }
    s.period = period;

    // Bytes 2-4: clock info
    s.clock.running = is_bit_set(data[2], 2);
    let centiseconds_mode = is_bit_set(data[2], 3);
    s.clock.on_break = is_bit_set(data[2], 4);
    s.clock.expired = is_bit_set(data[2], 5);
    let raw_remaining = ((data[2] as u16 & 0b11) << 8) | data[3] as u16;
    s.clock.remaining = if centiseconds_mode {
        Millis(raw_remaining as u32 * 10)
    } else {
        Millis(raw_remaining as u32 * 1000)
    };
    s.clock.passivity = Millis(data[4] as u32 * 1000);

    // Byte 5: raw strip input
    s.strip.blade_contact = is_bit_set(data[5], 6);
    s.strip.left.short = is_bit_set(data[5], 5);
    s.strip.right.short = is_bit_set(data[5], 4);
    s.strip.left.fault = is_bit_set(data[5], 3);
    s.strip.right.fault = is_bit_set(data[5], 2);
    s.strip.left.valid = is_bit_set(data[5], 1);
    s.strip.right.valid = is_bit_set(data[5], 0);

    // Bytes 6-9: latched lights + timing
    s.hide_extra_hits = is_bit_set(data[6], 6);
    let lflags = (data[6] >> 3) & 0b111;
    let rflags = data[6] & 0b111;
    let ltime = ((data[7] as u16) << 3) | ((data[8] as u16) >> 5);
    let rtime = (((data[8] & 0b0_00_111) as u16) << 7) | ((data[9] as u16) >> 1);

    s.left_light = decode_latched_light(lflags, ltime)?;
    s.right_light = decode_latched_light(rflags, rtime)?;

    // Bytes 10-11: scores
    s.left_score.score = data[10] & 0x7F;
    s.left_score.last_changed = is_bit_set(data[10], 7);
    s.right_score.score = data[11] & 0x7F;
    s.right_score.last_changed = is_bit_set(data[11], 7);

    // Byte 12: cards
    s.left_cards.card = decode_card((data[12] >> 0) & 0b11)?;
    s.left_cards.p_card = decode_card((data[12] >> 2) & 0b11)?;
    s.right_cards.card = decode_card((data[12] >> 4) & 0b11)?;
    s.right_cards.p_card = decode_card((data[12] >> 6) & 0b11)?;

    Ok(s)
}

/// Encodes a `State` into 13 raw data bytes.
pub fn encode_state_data(state: &State) -> [u8; 13] {
    let mut data = [0u8; 13];

    // Byte 0: config flags
    data[0] = bit_if(state.sleeping, 0)
        | bit_if(state.lockout_started, 1)
        | bit_if(state.live_video_view, 2)
        | bit_if(state.reviewing, 3);

    // Byte 1: match info
    let priority_bits = match state.priority {
        Priority::None => 0b00,
        Priority::Left => 0b01,
        Priority::Right => 0b10,
    };
    let weapon_bits = match state.weapon {
        Weapon::Sabre => 0b00,
        Weapon::Epee => 0b01,
        Weapon::Foil => 0b10,
    };
    data[1] = (priority_bits << 6) | (weapon_bits << 4) | state.period;

    // Bytes 2-4: clock info
    let ms = state.clock.remaining.as_millis();
    let centiseconds_mode = ms < 10_000;
    let raw_remaining: u16 = if centiseconds_mode {
        (ms / 10) as u16
    } else {
        (ms / 1000) as u16
    } & 0x3FF;
    data[2] = bit_if(state.clock.expired, 5)
        | bit_if(state.clock.on_break, 4)
        | bit_if(centiseconds_mode, 3)
        | bit_if(state.clock.running, 2)
        | (raw_remaining >> 8) as u8;
    data[3] = (raw_remaining & 0xFF) as u8;
    let passivity_secs = state.clock.passivity.as_secs();
    data[4] = if passivity_secs > 99 {
        99
    } else {
        passivity_secs as u8
    };

    // Byte 5: raw strip input
    data[5] = bit_if(state.strip.blade_contact, 6)
        | bit_if(state.strip.left.short, 5)
        | bit_if(state.strip.right.short, 4)
        | bit_if(state.strip.left.fault, 3)
        | bit_if(state.strip.right.fault, 2)
        | bit_if(state.strip.left.valid, 1)
        | bit_if(state.strip.right.valid, 0);

    // Bytes 6-9: latched lights + timing
    let (lflags, ltime) = encode_latched_light(&state.left_light);
    let (rflags, rtime) = encode_latched_light(&state.right_light);

    data[6] = bit_if(state.hide_extra_hits, 6) | (lflags << 3) | rflags;
    data[7] = ((ltime >> 3) & 0b0111_1111) as u8;
    data[8] = (((ltime & 0b111) << 5) | ((rtime >> 7) & 0b111)) as u8;
    data[9] = ((rtime << 1) & 0b1111_1110) as u8;

    // Bytes 10-11: scores
    data[10] = state.left_score.score | bit_if(state.left_score.last_changed, 7);
    data[11] = state.right_score.score | bit_if(state.right_score.last_changed, 7);

    // Byte 12: cards
    data[12] = encode_card(state.left_cards.card)
        | (encode_card(state.left_cards.p_card) << 2)
        | (encode_card(state.right_cards.card) << 4)
        | (encode_card(state.right_cards.p_card) << 6);

    data
}

fn decode_latched_light(flags: u8, time: u16) -> Result<LatchedLight, DecodeError> {
    let ms = Millis(time as u32);
    match flags {
        0 => Ok(LatchedLight::Off),
        1 => Ok(LatchedLight::Valid(ms)),
        2 => Ok(LatchedLight::NonValid(ms)),
        3 => Ok(LatchedLight::Whipover(ms)),
        4 => Ok(LatchedLight::Late(ms)),
        _ => Err(DecodeError::InvalidLatchedLight),
    }
}

fn encode_latched_light(light: &LatchedLight) -> (u8, u16) {
    match *light {
        LatchedLight::Off => (0, 0),
        LatchedLight::Valid(t) => (1, t.as_millis() as u16),
        LatchedLight::NonValid(t) => (2, t.as_millis() as u16),
        LatchedLight::Whipover(t) => (3, t.as_millis() as u16),
        LatchedLight::Late(t) => (4, t.as_millis() as u16),
    }
}

fn decode_card(bits: u8) -> Result<Card, DecodeError> {
    match bits {
        0b00 => Ok(Card::None),
        0b01 => Ok(Card::Yellow),
        0b10 => Ok(Card::Red),
        _ => Err(DecodeError::InvalidCard),
    }
}

fn encode_card(card: Card) -> u8 {
    match card {
        Card::None => 0b00,
        Card::Yellow => 0b01,
        Card::Red => 0b10,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_roundtrip() {
        let state = State {
            period: 1,
            clock: Clock {
                remaining: Millis::from_secs(180),
                ..Clock::default()
            },
            ..State::default()
        };
        let encoded = encode_state_data(&state);
        assert_eq!(
            encoded,
            [
                0x00, // flags
                0x01, // match info (period=1)
                0x00, 180, 0x00, // clock (180s)
                0x00, // strip
                0x00, // latched lights
                0x00, 0x00, 0x00, // timings
                0x00, 0x00, // scores
                0x00, // cards
            ]
        );
        let decoded = decode_state_data(&encoded).unwrap();
        assert_eq!(state, decoded);
    }

    #[test]
    fn clock_centiseconds_mode() {
        // 3420ms < 10000ms, so encoder uses centiseconds mode (342 cs)
        let state = State {
            period: 1,
            clock: Clock {
                running: true,
                remaining: Millis(3420),
                passivity: Millis::from_secs(52),
                ..Clock::default()
            },
            ..State::default()
        };
        let encoded = encode_state_data(&state);
        assert_eq!(
            encoded[2..=4],
            [
                (1 << 3) | (1 << 2) | (342u16 >> 8) as u8,
                (342 & 0xFF) as u8,
                52,
            ]
        );
        let decoded = decode_state_data(&encoded).unwrap();
        assert_eq!(state, decoded);
    }

    #[test]
    fn score_with_last_changed() {
        let state = State {
            period: 1,
            left_score: FencerScore {
                score: 5,
                last_changed: true,
            },
            right_score: FencerScore {
                score: 11,
                last_changed: false,
            },
            clock: Clock {
                remaining: Millis::from_secs(180),
                ..Clock::default()
            },
            ..State::default()
        };
        let encoded = encode_state_data(&state);
        assert_eq!(encoded[10..=11], [5 | 0x80, 11]);
        let decoded = decode_state_data(&encoded).unwrap();
        assert_eq!(state, decoded);
    }

    #[test]
    fn latched_lights_and_timing() {
        let state = State {
            period: 1,
            clock: Clock {
                remaining: Millis::from_secs(180),
                ..Clock::default()
            },
            left_light: LatchedLight::Valid(Millis(713)),
            right_light: LatchedLight::Late(Millis(417)),
            ..State::default()
        };
        let encoded = encode_state_data(&state);
        assert_eq!(
            encoded[6..=9],
            [
                0b_00_001_100, // left=valid(1), right=late(4)
                0b_0_1011001,  // left time high 7 bits: 713 = 0b10_1100_1001 -> top 7 = 1011001
                0b_001_00_011, // left low 3 = 001, reserved 00, right high 3 = 011
                0b_0100001_0,  // right low 7 = 0100001, reserved 0
            ]
        );
        let decoded = decode_state_data(&encoded).unwrap();
        assert_eq!(state, decoded);
    }

    #[test]
    fn hex_test_vector_1() {
        let input = hex_literal::hex!("00120053200000000000028481");
        let s = decode_state_data(&input).unwrap();
        assert_eq!(s.weapon, Weapon::Epee);
        assert_eq!(s.period, 2);
        assert_eq!(s.priority, Priority::None);
        assert_eq!(s.clock.running, false);
        assert_eq!(s.clock.remaining, Millis::from_secs(83));
        assert_eq!(s.clock.passivity, Millis::from_secs(32));
        assert_eq!(s.left_score.score, 2);
        assert_eq!(s.right_score.score, 4);
        assert_eq!(s.right_score.last_changed, true);
        assert_eq!(s.left_cards.card, Card::Yellow);
        assert_eq!(s.right_cards.p_card, Card::Red);
        assert_eq!(encode_state_data(&s), input);
    }

    #[test]
    fn hex_test_vector_2() {
        let input = hex_literal::hex!("00530422190000000000040916");
        let s = decode_state_data(&input).unwrap();
        assert_eq!(s.weapon, Weapon::Epee);
        assert_eq!(s.period, 3);
        assert_eq!(s.priority, Priority::Left);
        assert_eq!(s.clock.running, true);
        assert_eq!(s.clock.remaining, Millis::from_secs(34));
        assert_eq!(s.clock.passivity, Millis::from_secs(25));
        assert_eq!(s.left_score.score, 4);
        assert_eq!(s.right_score.score, 9);
        assert_eq!(s.left_cards.card, Card::Red);
        assert_eq!(s.left_cards.p_card, Card::Yellow);
        assert_eq!(s.right_cards.card, Card::Yellow);
        assert_eq!(encode_state_data(&s), input);
    }

    #[test]
    fn hex_test_vector_3() {
        let input = hex_literal::hex!("026100B4000C12436436040916");
        let s = decode_state_data(&input).unwrap();
        assert_eq!(s.weapon, Weapon::Foil);
        assert_eq!(s.period, 1);
        assert_eq!(s.priority, Priority::Left);
        assert_eq!(s.clock.remaining, Millis::from_secs(180));
        assert_eq!(s.lockout_started, true);
        assert_eq!(s.left_light, LatchedLight::NonValid(Millis(539)));
        assert_eq!(s.right_light, LatchedLight::NonValid(Millis(539)));
        assert_eq!(s.strip.left.fault, true);
        assert_eq!(s.strip.right.fault, true);
        assert_eq!(s.hide_extra_hits, false);
        assert_eq!(encode_state_data(&s), input);
    }

    #[test]
    fn hex_test_vector_4() {
        let input = hex_literal::hex!("000109FA1B0C00000000000000");
        let s = decode_state_data(&input).unwrap();
        assert_eq!(s.weapon, Weapon::Sabre);
        assert_eq!(s.period, 1);
        // 506 centiseconds = 5060ms
        assert_eq!(s.clock.remaining, Millis(5060));
        assert_eq!(s.clock.passivity, Millis::from_secs(27));
        assert_eq!(s.strip.left.fault, true);
        assert_eq!(s.strip.right.fault, true);
        assert_eq!(encode_state_data(&s), input);
    }

    #[test]
    fn hide_extra_hits_flag() {
        let mut state = State {
            period: 1,
            clock: Clock {
                remaining: Millis::from_secs(180),
                ..Clock::default()
            },
            hide_extra_hits: true,
            left_light: LatchedLight::Late(Millis(250)),
            right_light: LatchedLight::Whipover(Millis(50)),
            ..State::default()
        };
        let encoded = encode_state_data(&state);
        let decoded = decode_state_data(&encoded).unwrap();
        assert_eq!(decoded.hide_extra_hits, true);
        assert_eq!(decoded.left_light, LatchedLight::Late(Millis(250)));
        assert_eq!(decoded.right_light, LatchedLight::Whipover(Millis(50)));

        state.hide_extra_hits = false;
        let encoded = encode_state_data(&state);
        let decoded = decode_state_data(&encoded).unwrap();
        assert_eq!(decoded.hide_extra_hits, false);
    }

    #[test]
    fn cards_roundtrip() {
        let state = State {
            period: 1,
            clock: Clock {
                remaining: Millis::from_secs(180),
                ..Clock::default()
            },
            left_cards: FencerCards {
                card: Card::Yellow,
                p_card: Card::Red,
            },
            right_cards: FencerCards {
                card: Card::Red,
                p_card: Card::None,
            },
            ..State::default()
        };
        let encoded = encode_state_data(&state);
        let decoded = decode_state_data(&encoded).unwrap();
        assert_eq!(decoded.left_cards, state.left_cards);
        assert_eq!(decoded.right_cards, state.right_cards);
    }

    #[test]
    fn clock_expired_and_on_break() {
        let state = State {
            period: 1,
            clock: Clock {
                expired: true,
                on_break: true,
                ..Clock::default()
            },
            ..State::default()
        };
        let encoded = encode_state_data(&state);
        assert_eq!(encoded[2] & 0b0011_0000, 0b0011_0000);
        let decoded = decode_state_data(&encoded).unwrap();
        assert_eq!(decoded.clock.expired, true);
        assert_eq!(decoded.clock.on_break, true);
    }

    #[test]
    fn invalid_period_zero() {
        let mut data = [0u8; 13];
        data[1] = 0x00;
        assert_eq!(decode_state_data(&data), Err(DecodeError::InvalidPeriod));
    }

    #[test]
    fn invalid_weapon() {
        let mut data = [0u8; 13];
        data[1] = 0x31;
        assert_eq!(decode_state_data(&data), Err(DecodeError::InvalidWeapon));
    }

    #[test]
    fn invalid_priority() {
        let mut data = [0u8; 13];
        data[1] = 0xC1;
        assert_eq!(decode_state_data(&data), Err(DecodeError::InvalidPriority));
    }

    #[test]
    fn all_lights_variants_roundtrip() {
        let variants = [
            (LatchedLight::Off, LatchedLight::Off),
            (
                LatchedLight::Valid(Millis(0)),
                LatchedLight::Valid(Millis(999)),
            ),
            (
                LatchedLight::NonValid(Millis(500)),
                LatchedLight::Late(Millis(100)),
            ),
            (
                LatchedLight::Whipover(Millis(15)),
                LatchedLight::NonValid(Millis(0)),
            ),
            (
                LatchedLight::Late(Millis(713)),
                LatchedLight::Whipover(Millis(417)),
            ),
        ];
        for (left, right) in variants {
            let state = State {
                period: 1,
                clock: Clock {
                    remaining: Millis::from_secs(180),
                    ..Clock::default()
                },
                left_light: left,
                right_light: right,
                ..State::default()
            };
            let decoded = decode_state_data(&encode_state_data(&state)).unwrap();
            assert_eq!(decoded.left_light, left);
            assert_eq!(decoded.right_light, right);
        }
    }

    #[test]
    fn strip_input_roundtrip() {
        let state = State {
            period: 1,
            clock: Clock {
                remaining: Millis::from_secs(180),
                ..Clock::default()
            },
            strip: StripInput {
                blade_contact: true,
                left: FencerStripInput {
                    valid: true,
                    fault: false,
                    short: true,
                },
                right: FencerStripInput {
                    valid: false,
                    fault: true,
                    short: false,
                },
            },
            ..State::default()
        };
        let decoded = decode_state_data(&encode_state_data(&state)).unwrap();
        assert_eq!(decoded.strip, state.strip);
    }

    #[test]
    fn decode_score_bits() {
        let cases: [(u8, u8, bool); 4] = [
            (0b1000_0101, 5, true),
            (0b0000_1010, 10, false),
            (0b0110_0011, 99, false),
            (0b1110_0011, 99, true),
        ];
        for (encoded, expected_score, expected_last) in cases {
            let score_val = encoded & 0x7F;
            let last = (encoded & 0x80) != 0;
            assert_eq!(score_val, expected_score);
            assert_eq!(last, expected_last);
        }
    }
}
