# Protocol Specification

This document describes the serial/bluetooth protocol for the Skewered Fencing
scoring boxes as of build 324. Please note that this is the initial release of
the protocol and changes may occur in future releases. Despite that, efforts
will be made to keep the protocols backwards compatible when possible.

# Overview

The communication protocol is described in two layers:

- The data layer describes the types of packets that may be transmitted and
  their structure.

- The transport layer describes encapsulation of the data packets according to
  the specific transport mechanism (e.g. serial port, bluetooth, wifi, usb).

# Data layer

There are currently two types of data packets: state updates and event
notifications.

## State updates

State update packets are available at intervals of up to every 5ms, although the
actual transmission rate depends on the transport layer.

### Structure

State update packets consist of 13 data bytes:

```text
   ┌─── byte index in data packet
   │
   │     1 byte Type prefix: 0xee
  ━┷   ---------------------------------------------------------
  00     1 byte config flags
  01     1 byte match info
  02     3 byte clock info
  --   ----
  05     1 byte raw strip input state
  06     1 byte basic lights
  07     3 byte special light info (usually all 0s)
  --   ----
  10     2 byte fencer score data
  12     1 byte fencer penalty cards
       ---------------------------------------------------------
```

### Fields

- **config flags** <br>
  This byte provides overall system flags:

  ```
  High bit is always 0
  Subsequent bits are zero until defined flags below as the low bits:
  R = reviewing last touch, may also be used to indicate video replay review
  P = live preview is on for any video replay system.
  L = Lockout has started. This will stay on until touch resets.
  S = sleep mode enabled
  (^- lowest bit)

  E.g. | 0000 RPLS |
  ```

- **match info** <br>
  This byte encodes informatino about the current match configuration:

  ```
  priority: 2 bits (00 = none, 01 = left, 10 = right, 11 = reserved)
  weapon: 2 bits (00 = sabre, 01 = epee, 10 = foil, 11 = reserved)
  period: 4 bits (1-9) (*NOTE: should never be 0, minimum is 1)

  E.g.  | PP WW pppp |
  ```

- **clock info** <br>
  These 3 bytes encode the state of the bout clock:

  ```
  clock flags:
    00 = highest 2 bits are reserved, always 0
    expired: 1 bit
    on_break: 1 bit
    centiseconds: 1 bit, 0 = seconds, 1 = centiseconds
    running: 1 bit
  remaining_time: 10 bits (in seconds or centiseconds)
    (when time remaining is 10s or less, it's sent as centiseconds)
    The highest 2 bits are jammed into the lower 2 bits of the flags,
    and the remaining 8 bits are sent as the following byte.
  passivity clock: 8 bits (in seconds, maxes out at 99s)

  E.g.:  | 00 ffff rr | rrrrrrrr | pppppppp |
         high         |          |        low

     where ffff = flags, rr...rr = remaining time, pp...pp = passivity time
  ```

- **raw strip state** <br>
  This byte indicates the instantaneous state of the strip. That is, even if a
  touch has occured and a colored light may be on, if the blade is not currently
  in contact with the target area then "valid" will be off. Likewise, a valid
  hit contact may be occurring but too short to trigger a touch.

  ```
  0 high bit always 0
  X blade contact = 1 bit
  S short = 2 bits (Left high, Right low)
  F fault = 2 bits (Left high, Right low)
  V valid = 2 bits (Left high, Right low)

  E.g.  | 0 X SS FF VV |
              LR LR LR
  ```

- **latched lights** <br>
  This byte indicates the state of the latched lights that should remain on when
  a touch is detected and the buzzer sounds.

  ```
  bit 7: always 0
  bit 6: hide_extra_hits flag (1 = hide late/whipover from display, 0 = show all)
  left: 3 bits: 0 = off, 1 = valid, 2 = nonvalid, 3 = short/whipover, 4 = late, 5+ = reserved
  right: 3 bits: 0 = off, 1 = valid, 2 = nonvalid, 3 = short/whipover, 4 = late, 5+ = reserved

  E.g.:  | 0H LLL RRR |
  ```

  The `hide_extra_bits` flag is set when the scoring box is configured NOT to
  show additional hit timing and allows repeater displays to respect that
  setting.

- **extra timing values** <br>
  These 3 bytes are the timing values of late hits or short/whipover values on
  the box. The left and right sides are both allocated up a 10 bit value for
  additional timing information. The interpretation of this value depends on the
  type of latched hit:
  - **0 (off)**: No additional timing information, the value is always 0.
  - **1 (valid)**: Milliseconds since the hit occurred.
  - **2 (nonvalid)**: Milliseconds since the hit occurred.
  - **3 (short hit/whipover)**: Duration of the short hit / whipover hit in milliseconds.
  - **4 (late hit)**: Time of the hit in milliseconds since the lockout started.
    <br>(For example, value of 183 in sabre indicates a hit that occurs 183ms
    after the opponents touch, which is 13ms late.)

  In all cases, the range of encoded values is 0 - 999. If the actual value
  exceeds that, the encoded value is capped at 999.

  The values for valid and nonvalid allow clients to synchronize hit timing.

  The structure of these 3 bytes is:

  ```
  high bit always 0 (reserved)
  10 bits left time indicator (0-999), only set on a left hit
  2 bits always zero (reserved)
  10 bits right time indicator (0-999), only set on a right hit
  low bit always 0 (reserved)

  E.g.: | 0LLL_LLLL | LLL_00_RRR | RRRR_RRR0 |
        high        |            |         low
  ```

- **score info** <br>
  These two bytes provide scores for each player as well as the "last scored"
  flag:
  ```
  left score: 7 bits (0-99) + high bit set if last changed
  right score: 7 bits (0-99) + high bit set if last changed
  (note that both 'last changed' bits may be set,
   e.g. double score in epee)
  ```
- **penalty card info** <br>
  This byte indicates the status of cards and p-cards for each fencer:

  ```
  left: 2 bits for normal card, 2 bits for p card (Red/Yellow)
  right: 2 bits for normal card, 2 bits for p card (Red/Yellow)
  For example:
          76 54   32 10 (lowest bit)
          RY RY   RY RY   <-- color: R=red, Y=yellow)
         |-rght-|-left-|  <-- side
          PP|NN   PP|NN   <-- type of card (P- or Normal)
  bit 0 is left normal yellow card on/off.
  bit 7 is right red p-card on/off.

  Note that currently it's not allowed to have both cards active for a fencer.
  ```

## Event notifications

Event notifications emit almost all of the input data that is processed by the
scoring box. These are informational and completely independent of the state of
the box or the processing of these events. These can also be sent to the box to
control/configure the machine.

### Structure

Event data packets consist of 3 data bytes:

```
   ┌─── byte index in data packet
  ━┷   ---------------------------------------------------------
  00     1 byte event ID (see below)
  01     1 byte extra data (depending on the event ID)
  02     1 byte dropped event count up to 250
  --   ---------------------------------------------------------
```

The dropped notification count should always be 0. If it is non-zero, it
means that notifications could not be sent to the client fast enough and some
notifications have been lost.

### Event IDs

- `0x00`: Ignored / Invalid. These should not be sent but might be. <p>
- **System configuration**
  - `0x01`: SetWeapon(weapon)
    <br> The next byte is the selected weapon mode:
    - `0x01`: Sabre
    - `0x02`: Epee
    - `0x03`: Foil
  - `0x02`: EnterMenu
  - `0x03`: MenuKey(key)
    <br>The next byte is the key:
    - `0x00`: Other/Ignored
    - `0x01`: Up
    - `0x02`: Down
    - `0x03`: Left
    - `0x04`: Right
    - `0x05`: Select
    - `0x06`: Exit
    - `0x07`: Func
  - `0x04`: SleepNow
  - `0x05`: SetStripID/RemoteAddress(id)
    <br> The next byte is the ID: 0 - 99
  - `0x06`: RemoteBatteryLevel(charge)
    <br>The next byte is the charge level percetage: 0 - 100
- **Bout operations**
  - `0x10`: ClearScores
  - `0x11`: ScoreUpByOne(side)
    <br> The next byte is the side affected (`0x01` = left, `0x02` = right, `0x03` = both)
    <br> (in epee both sides can score)
  - `0x12`: ScoreDownByOne(side)
    <br> The next byte is the side affected (`0x01` = left, `0x02` = right, `0x03` = both)
  - `0x13`: CycleCard(side)
    <br> The next byte is the side affected (`0x01` = left, `0x02` = right, `0x03` = both)
  - `0x14`: CyclePCard(side)
    <br> The next byte is the side affected (`0x01` = left, `0x02` = right, `0x03` = both)
  - `0x15`: CyclePriority
- **Clock operations**
  - `0x20`: Reset (to 3 min)
  - `0x21`: EnterTime (enter time-editing mode)
  - `0x22`: StartStop
  - `0x23`: StartBreak
  - `0x24`: AdjustSec(amount)
    <br> The next byte is the number of seconds adjusted as a signed int8. Values
    are generally just +1 (`0x01`) or -1 (`0xff`).
  - `0x25`: AdjustPeriod(amount)
    <br> The next byte is the amount of adjustment as a signed int8. Amount is
    usually just +1 (`0x01`) or -1 (`0xff`).
- **Timeline / Misc**
  - `0x30`: ReviewTimelineBack / ToggleVideoReplay
  - `0x31`: Undo
  - `0x32`: ReviewTimelineFwd
  - `0x33`: Func
  - `0x34`: ToucheOccurred

# Transport Layers

## Serial Port

The serial protocol is full-duplex RS-485, operating at 115200 baud, 8 data
bits, 1 stop bit, no parity bits.

TODO: wiring diagram

The data emitted from the box is a continuous stream of packets that correspond
to the data packets wrapped in an envelope consisting of 1 prefix byte and 2
suffix bytes:

- Packet type (1 byte):
  - `0xEE`: State update packet
  - `0xED`: Event notification packet
- ...data packet...
- Checksum (1 byte): wrapping sum of the packet type byte and all data bytes, truncated to one byte.
- Terminator (1 byte): `0xFF`

For example, an event packet would look like:

```
   ┌───── byte index with envelope
   │   ┌─── byte index in data packet only
  ━┷   │
  00   │     1 byte Type prefix: 0xed
  --  ━┷   ---------------------------------------------------------
  01  00     1 byte event ID
  02  01     1 byte extra data
  03  02     1 byte dropped event count up to 250
  --  --   ---------------------------------------------------------
  04         1 byte checksum
  05         1 byte terminator: 0xff
  --       ----------------------------------------------------------
```

## Bluetooth

The scoring box will advertise itself by selecting a descriptive name and
including manufacturer-specific data under the manufacturer ID of `0x0E88`.

The short name will be `SkF:#` where `#` is the configured strip id of the box
(one or two digits, 0-99). This is intended to quickly and trivially allow
identifying which strip a given bluetooth source is associated with.

Each advertisement packet will include (in the manufacturer's payload) an
up-to-date [State data packet](#state-updates).
