use crate::error::DecodeError;
use crate::types::*;

/// A flat event enum representing all wire-level events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    SetWeapon(Weapon),
    EnterMenu,
    MenuKey(MenuKey),
    SleepNow,
    SetRemoteAddr(u8),
    RemoteBatteryLevel(u8),
    /// Emulates the remote's MODE/SEL button: the receiver shows the current
    /// weapon if it is idle, or advances to the next weapon if the weapon is
    /// already being shown. The show-vs-advance decision is made by the
    /// receiver (it is time-sensitive), so this event carries no argument.
    NextWeapon,
    /// Emulates a keypress of the physical IR remote. The receiver feeds the
    /// key through its normal IR pipeline, so the press behaves exactly like
    /// the paired remote's button — including mode-dependent routing (config
    /// menu navigation, numeric time entry) and guards (operations blocked
    /// while the clock runs).
    RemoteKey(RemoteKey),

    ClearScores,
    ScoreUp(Side),
    ScoreDown(Side),
    CycleCard(Side),
    CyclePCard(Side),
    CyclePriority,

    ClockReset,
    ClockEnterTime,
    ClockStartStop,
    ClockStartBreak,
    ClockAdjSec(i8),
    ClockAdjPeriod(i8),

    ReviewTimelineBack,
    Undo,
    ReviewTimelineFwd,
    Func,
    ToucheOccurred,
}

/// A decoded event packet: the event plus the dropped count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventPacket {
    pub event: Event,
    pub dropped_count: u8,
}

/// Decodes 3 raw data bytes into an `EventPacket`.
pub fn decode_event_data(data: &[u8; 3]) -> Result<EventPacket, DecodeError> {
    let event = match data[0] {
        0x01 => Event::SetWeapon(match data[1] {
            0x01 => Weapon::Sabre,
            0x02 => Weapon::Epee,
            0x03 => Weapon::Foil,
            _ => return Err(DecodeError::InvalidEventData),
        }),
        0x02 => Event::EnterMenu,
        0x03 => Event::MenuKey(match data[1] {
            0x00 => MenuKey::Other,
            0x01 => MenuKey::Up,
            0x02 => MenuKey::Down,
            0x03 => MenuKey::Left,
            0x04 => MenuKey::Right,
            0x05 => MenuKey::Select,
            0x06 => MenuKey::Exit,
            0x07 => MenuKey::Func,
            _ => return Err(DecodeError::InvalidEventData),
        }),
        0x04 => Event::SleepNow,
        0x05 => Event::SetRemoteAddr(data[1]),
        0x06 => Event::RemoteBatteryLevel(data[1]),
        0x07 => Event::NextWeapon,
        0x08 => {
            Event::RemoteKey(RemoteKey::from_code(data[1]).ok_or(DecodeError::InvalidEventData)?)
        }

        0x10 => Event::ClearScores,
        0x11 => Event::ScoreUp(decode_side(data[1])?),
        0x12 => Event::ScoreDown(decode_side(data[1])?),
        0x13 => Event::CycleCard(decode_side(data[1])?),
        0x14 => Event::CyclePCard(decode_side(data[1])?),
        0x15 => Event::CyclePriority,

        0x20 => Event::ClockReset,
        0x21 => Event::ClockEnterTime,
        0x22 => Event::ClockStartStop,
        0x23 => Event::ClockStartBreak,
        0x24 => Event::ClockAdjSec(data[1] as i8),
        0x25 => Event::ClockAdjPeriod(data[1] as i8),

        0x30 => Event::ReviewTimelineBack,
        0x31 => Event::Undo,
        0x32 => Event::ReviewTimelineFwd,
        0x33 => Event::Func,
        0x34 => Event::ToucheOccurred,

        _ => return Err(DecodeError::InvalidEventId),
    };

    Ok(EventPacket {
        event,
        dropped_count: data[2],
    })
}

/// Encodes an event and dropped count into 3 raw data bytes.
pub fn encode_event_data(event: &Event, dropped_count: u8) -> [u8; 3] {
    let mut data = [0u8; 3];
    data[0] = encode_event_id(event);
    data[1] = encode_event_extra(event);
    data[2] = dropped_count;
    data
}

fn encode_event_id(event: &Event) -> u8 {
    match event {
        Event::SetWeapon(_) => 0x01,
        Event::EnterMenu => 0x02,
        Event::MenuKey(_) => 0x03,
        Event::SleepNow => 0x04,
        Event::SetRemoteAddr(_) => 0x05,
        Event::RemoteBatteryLevel(_) => 0x06,
        Event::NextWeapon => 0x07,
        Event::RemoteKey(_) => 0x08,

        Event::ClearScores => 0x10,
        Event::ScoreUp(_) => 0x11,
        Event::ScoreDown(_) => 0x12,
        Event::CycleCard(_) => 0x13,
        Event::CyclePCard(_) => 0x14,
        Event::CyclePriority => 0x15,

        Event::ClockReset => 0x20,
        Event::ClockEnterTime => 0x21,
        Event::ClockStartStop => 0x22,
        Event::ClockStartBreak => 0x23,
        Event::ClockAdjSec(_) => 0x24,
        Event::ClockAdjPeriod(_) => 0x25,

        Event::ReviewTimelineBack => 0x30,
        Event::Undo => 0x31,
        Event::ReviewTimelineFwd => 0x32,
        Event::Func => 0x33,
        Event::ToucheOccurred => 0x34,
    }
}

fn encode_event_extra(event: &Event) -> u8 {
    match event {
        Event::SetWeapon(w) => match w {
            Weapon::Sabre => 0x01,
            Weapon::Epee => 0x02,
            Weapon::Foil => 0x03,
        },
        Event::MenuKey(key) => match key {
            MenuKey::Other => 0x00,
            MenuKey::Up => 0x01,
            MenuKey::Down => 0x02,
            MenuKey::Left => 0x03,
            MenuKey::Right => 0x04,
            MenuKey::Select => 0x05,
            MenuKey::Exit => 0x06,
            MenuKey::Func => 0x07,
        },
        Event::SetRemoteAddr(id) => *id,
        Event::RemoteBatteryLevel(v) => *v,
        Event::RemoteKey(key) => key.code(),
        Event::ScoreUp(side) => encode_side(side),
        Event::ScoreDown(side) => encode_side(side),
        Event::CycleCard(side) => encode_side(side),
        Event::CyclePCard(side) => encode_side(side),
        Event::ClockAdjSec(v) => *v as u8,
        Event::ClockAdjPeriod(v) => *v as u8,
        _ => 0,
    }
}

fn encode_side(side: &Side) -> u8 {
    match side {
        Side::Left => 0x01,
        Side::Right => 0x02,
        Side::Both => 0x03,
    }
}

fn decode_side(byte: u8) -> Result<Side, DecodeError> {
    match byte {
        0x01 => Ok(Side::Left),
        0x02 => Ok(Side::Right),
        0x03 => Ok(Side::Both),
        _ => Err(DecodeError::InvalidEventData),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_start_stop_roundtrip() {
        let data = encode_event_data(&Event::ClockStartStop, 13);
        assert_eq!(data, [0x22, 0, 13]);
        let decoded = decode_event_data(&data).unwrap();
        assert_eq!(decoded.event, Event::ClockStartStop);
        assert_eq!(decoded.dropped_count, 13);
    }

    #[test]
    fn clock_adj_sec_positive() {
        let data = encode_event_data(&Event::ClockAdjSec(1), 0);
        assert_eq!(data, [0x24, 1, 0]);
        let decoded = decode_event_data(&data).unwrap();
        assert_eq!(decoded.event, Event::ClockAdjSec(1));
    }

    #[test]
    fn clock_adj_sec_negative() {
        let data = encode_event_data(&Event::ClockAdjSec(-1), 0);
        assert_eq!(data, [0x24, 255, 0]);
        let decoded = decode_event_data(&data).unwrap();
        assert_eq!(decoded.event, Event::ClockAdjSec(-1));
    }

    #[test]
    fn all_events_roundtrip() {
        let events = [
            Event::SetWeapon(Weapon::Sabre),
            Event::SetWeapon(Weapon::Epee),
            Event::SetWeapon(Weapon::Foil),
            Event::EnterMenu,
            Event::MenuKey(MenuKey::Up),
            Event::MenuKey(MenuKey::Down),
            Event::MenuKey(MenuKey::Left),
            Event::MenuKey(MenuKey::Right),
            Event::MenuKey(MenuKey::Select),
            Event::MenuKey(MenuKey::Exit),
            Event::MenuKey(MenuKey::Func),
            Event::MenuKey(MenuKey::Other),
            Event::SleepNow,
            Event::SetRemoteAddr(42),
            Event::RemoteBatteryLevel(85),
            Event::NextWeapon,
            Event::ClearScores,
            Event::ScoreUp(Side::Left),
            Event::ScoreUp(Side::Right),
            Event::ScoreUp(Side::Both),
            Event::ScoreDown(Side::Left),
            Event::CycleCard(Side::Right),
            Event::CyclePCard(Side::Both),
            Event::CyclePriority,
            Event::ClockReset,
            Event::ClockEnterTime,
            Event::ClockStartStop,
            Event::ClockStartBreak,
            Event::ClockAdjSec(5),
            Event::ClockAdjSec(-3),
            Event::ClockAdjPeriod(1),
            Event::ClockAdjPeriod(-1),
            Event::ReviewTimelineBack,
            Event::Undo,
            Event::ReviewTimelineFwd,
            Event::Func,
            Event::ToucheOccurred,
        ];
        for event in events {
            let data = encode_event_data(&event, 0);
            let decoded = decode_event_data(&data).unwrap();
            assert_eq!(decoded.event, event, "roundtrip failed for {:?}", event);
        }
    }

    #[test]
    fn remote_key_roundtrip() {
        for key in RemoteKey::ALL {
            let event = Event::RemoteKey(key);
            let data = encode_event_data(&event, 0);
            assert_eq!(data, [0x08, key.code(), 0]);
            let decoded = decode_event_data(&data).unwrap();
            assert_eq!(decoded.event, event, "roundtrip failed for {:?}", key);
        }
    }

    #[test]
    fn remote_key_rejects_non_button_codes() {
        for code in [0u8, 1, 130, 255] {
            let data = [0x08, code, 0x00];
            assert_eq!(
                decode_event_data(&data),
                Err(DecodeError::InvalidEventData),
                "code {code} should not decode"
            );
        }
    }

    #[test]
    fn invalid_event_id() {
        let data = [0xFF, 0x00, 0x00];
        assert_eq!(decode_event_data(&data), Err(DecodeError::InvalidEventId));
    }

    #[test]
    fn invalid_weapon_in_event() {
        let data = [0x01, 0x00, 0x00]; // SetWeapon with invalid weapon byte
        assert_eq!(decode_event_data(&data), Err(DecodeError::InvalidEventData));
    }

    #[test]
    fn invalid_side_in_event() {
        let data = [0x11, 0x00, 0x00]; // ScoreUp with invalid side byte
        assert_eq!(decode_event_data(&data), Err(DecodeError::InvalidEventData));
    }

    #[test]
    fn dropped_count_capping() {
        // Verify we can encode/decode max dropped count
        let data = encode_event_data(&Event::ClockStartStop, 250);
        let decoded = decode_event_data(&data).unwrap();
        assert_eq!(decoded.dropped_count, 250);
    }
}
