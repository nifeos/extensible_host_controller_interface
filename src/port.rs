//! **The port status and control register** (`PORTSC`, xHCI 1.2 section 5.4.8), and the trap in it.
//!
//! Half its bits are write-one-to-clear, and one of those is Port Enabled: writing back what was
//! read, with a single bit added, **disables the port** and acknowledges every pending change
//! nobody has looked at yet. So a write is always [`write_value`]: the bits that are safe to write
//! back, plus the one the caller means.
//!
//! Reach for it whenever reading or writing a port status register: on a Port Status Change
//! Event, to reset a port, and to debounce a connection.

/// Current Connect Status: a device is attached.
#[doc(alias = "PORTSC.CCS")]
pub const CONNECTED: u32 = 1 << 0;
/// Port Enabled. **Write one to clear, which disables the port.**
#[doc(alias = "PORTSC.PED")]
pub const ENABLED: u32 = 1 << 1;
/// Port Reset: write one to reset the port, reads one until the reset finishes.
#[doc(alias = "PORTSC.PR")]
pub const RESET: u32 = 1 << 4;
/// Port Power.
#[doc(alias = "PORTSC.PP")]
pub const POWER: u32 = 1 << 9;
/// Connect Status Change, write one to clear.
#[doc(alias = "PORTSC.CSC")]
pub const CONNECT_CHANGE: u32 = 1 << 17;
/// Port Enabled/Disabled Change, write one to clear.
#[doc(alias = "PORTSC.PEC")]
pub const ENABLE_CHANGE: u32 = 1 << 18;
/// Warm Port Reset Change, write one to clear.
#[doc(alias = "PORTSC.WRC")]
pub const WARM_RESET_CHANGE: u32 = 1 << 19;
/// Over-current Change, write one to clear.
#[doc(alias = "PORTSC.OCC")]
pub const OVER_CURRENT_CHANGE: u32 = 1 << 20;
/// Port Reset Change, write one to clear.
#[doc(alias = "PORTSC.PRC")]
pub const RESET_CHANGE: u32 = 1 << 21;
/// Port Link State Change, write one to clear.
#[doc(alias = "PORTSC.PLC")]
pub const LINK_CHANGE: u32 = 1 << 22;
/// Port Config Error Change, write one to clear.
#[doc(alias = "PORTSC.CEC")]
pub const CONFIGURATION_ERROR_CHANGE: u32 = 1 << 23;

/// Every change bit: what a driver writes back to acknowledge all of them.
pub const CHANGES: u32 = CONNECT_CHANGE
    | ENABLE_CHANGE
    | WARM_RESET_CHANGE
    | OVER_CURRENT_CHANGE
    | RESET_CHANGE
    | LINK_CHANGE
    | CONFIGURATION_ERROR_CHANGE;

/// The seven change bits share no bit (each is alone at 17 to 23), which the compiler checks
/// here; the mutation exclusions for this file's `|` constants name this constant. The
/// constant-assertion lint is off on purpose: the assertion is constant by construction, and the
/// compiler checking it is its job.
#[allow(clippy::assertions_on_constants, clippy::eq_op, dead_code)]
pub(crate) const PORT_CHANGE_BITS_DO_NOT_OVERLAP: () = assert!(
    CONNECT_CHANGE
        + ENABLE_CHANGE
        + WARM_RESET_CHANGE
        + OVER_CURRENT_CHANGE
        + RESET_CHANGE
        + LINK_CHANGE
        + CONFIGURATION_ERROR_CHANGE
        == CHANGES
);

/// The bits a write may carry back unchanged: the read-only ones (ignored on write) and the
/// read-write ones that must keep their value (power, indicator, link state without its strobe,
/// wake enables). The same set Linux's `xhci_port_state_to_neutral` keeps.
pub(crate) const PRESERVE: u32 =
    1 | 1 << 3 | 0xf << 10 | 1 << 30 | 0xf << 5 | 1 << 9 | 0x3 << 14 | 0x7 << 25;

/// The eight preserved fields share no bit (0, 3, 5:8, 9, 10:13, 14:15, 25:27 and 30), which the
/// compiler checks here; the mutation exclusions for this file's `|` constants name this
/// constant. The constant-assertion lint is off on purpose: the assertion is constant by
/// construction, and the compiler checking it is its job.
#[allow(clippy::assertions_on_constants, clippy::eq_op, dead_code)]
pub(crate) const PRESERVED_FIELDS_DO_NOT_OVERLAP: () = assert!(
    1 + (1 << 3) + (0xf << 10) + (1 << 30) + (0xf << 5) + (1 << 9) + (0x3 << 14) + (0x7 << 25)
        == PRESERVE
);

use crate::PortSpeed;

/// The attached device's speed id, from bits 13:10, or `None` when they read 0: no device.
pub const fn speed_of(port_status_and_control: u32) -> Option<PortSpeed> {
    PortSpeed::new(((port_status_and_control >> 10) & 0xf) as u8)
}

/// **What to write to set `bits`** without disabling the port or acknowledging a change nobody
/// asked to: `port_status_and_control`'s preserved bits, plus `bits`.
pub const fn write_value(port_status_and_control: u32, bits: u32) -> u32 {
    (port_status_and_control & PRESERVE) | bits
}

/// **Endpoint 0's packet size before the device has said**, by speed (xHCI 1.2 section 4.3): 8 for low
/// and full speed (a full-speed device may want up to 64, which the caller learns from the first
/// eight bytes of its descriptor and fixes with Evaluate Context), 64 for high speed, 512 above.
/// `None` for a speed id this table does not know.
pub const fn default_control_packet(speed: PortSpeed) -> Option<u16> {
    match speed {
        PortSpeed::LOW | PortSpeed::FULL => Some(8),
        PortSpeed::HIGH => Some(64),
        PortSpeed::SUPER | PortSpeed::SUPER_PLUS => Some(512),
        _ => None,
    }
}

/// The name a bench transcript prints for a speed id.
pub const fn speed_name(speed: PortSpeed) -> &'static str {
    match speed {
        PortSpeed::LOW => "low speed",
        PortSpeed::FULL => "full speed",
        PortSpeed::HIGH => "high speed",
        PortSpeed::SUPER => "SuperSpeed",
        PortSpeed::SUPER_PLUS => "SuperSpeedPlus",
        _ => "an unknown speed",
    }
}

/// **When the root ports are believed after a controller reset.** The reset disconnects every
/// device, powered or not, and silicon reports each one again only after some milliseconds; QEMU
/// does it at once. A boot-time scan therefore waits until no port's `CCS` has changed for
/// [`DEBOUNCE_STABLE_MILLISECONDS`], read every [`DEBOUNCE_STEP_MILLISECONDS`], giving up at [`DEBOUNCE_TIMEOUT_MILLISECONDS`].
/// The clock starts when the ports are powered, so nothing is believed before
/// [`DEBOUNCE_STABLE_MILLISECONDS`] after power either. The numbers are Linux's (`drivers/usb/core/hub.c`).
///
/// ```
/// use extensible_host_controller_interface::port::{
///     Connections, DebounceVerdict, PortDebounce,
/// };
///
/// // Port 1 is connected when the ports are powered at 0 ms, and stays so.
/// let port_1: Connections = [1 << 1, 0, 0, 0, 0, 0, 0, 0];
/// let mut debounce = PortDebounce::new(0, port_1);
/// assert_eq!(debounce.sample(25, port_1), DebounceVerdict::Pending);
/// assert_eq!(debounce.sample(100, port_1), DebounceVerdict::Settled);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PortDebounce {
    start_milliseconds: u64,
    changed_milliseconds: u64,
    last: Connections,
}

/// Every root port's `CCS`, one bit per port number: 256 bits, since `HCSPARAMS1.MaxPorts` is a byte.
pub type Connections = [u32; 8];

/// USB 2.0's `TATTDB` (section 7.1.7.3), a connection's debounce interval, and Linux's
/// `HUB_DEBOUNCE_STABLE`. It also covers USB 2.0's `TSIGATT` (a device attaches within 100 ms of
/// power), and is the floor Linux puts on an external hub's power-on-to-good time (a root hub has
/// none).
pub const DEBOUNCE_STABLE_MILLISECONDS: u64 = 100;
/// Linux's `HUB_DEBOUNCE_STEP`.
pub const DEBOUNCE_STEP_MILLISECONDS: u64 = 25;
/// Linux's `HUB_DEBOUNCE_TIMEOUT`.
pub const DEBOUNCE_TIMEOUT_MILLISECONDS: u64 = 2000;

/// **How long to wait after setting [`POWER`]** before changing the port's state, on a controller
/// with Port Power Control (xHCI 1.2 section 5.4.8).
pub const POWER_ON_WAIT_MILLISECONDS: u64 = 20;
/// **How long to wait for [`RESET_CHANGE`]** after setting [`RESET`]. This is Linux's
/// `HUB_RESET_TIMEOUT`.
pub const RESET_TIMEOUT_MILLISECONDS: u64 = 800;

/// **What [`PortDebounce::sample`] says**, after each read of the ports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DebounceVerdict {
    /// Keep reading: the ports have changed within [`DEBOUNCE_STABLE_MILLISECONDS`], and the wait
    /// has not reached [`DEBOUNCE_TIMEOUT_MILLISECONDS`].
    Pending,
    /// Nothing has changed for [`DEBOUNCE_STABLE_MILLISECONDS`]: the connections read are the
    /// ports' own.
    Settled,
    /// The wait reached [`DEBOUNCE_TIMEOUT_MILLISECONDS`] with the ports still changing, so the
    /// connections read are not a fact.
    TimedOut,
}

impl PortDebounce {
    /// Start at `now_milliseconds`, when the ports were powered, having read `first`.
    pub const fn new(now_milliseconds: u64, first: Connections) -> PortDebounce {
        PortDebounce {
            start_milliseconds: now_milliseconds,
            changed_milliseconds: now_milliseconds,
            last: first,
        }
    }

    /// The ports read `seen` at `now_milliseconds`: what to do next, as a [`DebounceVerdict`].
    pub fn sample(&mut self, now_milliseconds: u64, seen: Connections) -> DebounceVerdict {
        if seen != self.last {
            self.last = seen;
            self.changed_milliseconds = now_milliseconds;
        }
        if now_milliseconds.saturating_sub(self.changed_milliseconds)
            >= DEBOUNCE_STABLE_MILLISECONDS
        {
            DebounceVerdict::Settled
        } else if now_milliseconds.saturating_sub(self.start_milliseconds)
            >= DEBOUNCE_TIMEOUT_MILLISECONDS
        {
            DebounceVerdict::TimedOut
        } else {
            DebounceVerdict::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The trap, closed**: a port that is enabled with a reset change pending, written to start
    /// another reset, keeps its enable bit off in the written value (writing it would disable the
    /// port) and does not acknowledge the change.
    #[test]
    fn a_write_never_carries_the_enable_bit_or_a_change() {
        let read = CONNECTED | ENABLED | POWER | RESET_CHANGE | 3 << 10;
        let w = write_value(read, RESET);
        assert_eq!(w & ENABLED, 0);
        assert_eq!(w & CHANGES, 0);
        assert_ne!(w & POWER, 0);
        assert_ne!(w & RESET, 0);
        assert_eq!(speed_of(read), Some(PortSpeed::HIGH));
        assert_eq!(speed_of(CONNECTED | POWER), None);
    }

    /// Each speed id names itself in the boot line, and an id no table knows says so rather than
    /// claiming one.
    #[test]
    fn every_speed_has_its_own_name() {
        let names = [
            PortSpeed::LOW,
            PortSpeed::FULL,
            PortSpeed::HIGH,
            PortSpeed::SUPER,
            PortSpeed::SUPER_PLUS,
            PortSpeed::new(15).unwrap(),
        ]
        .map(speed_name);
        assert_eq!(names[1], "full speed");
        assert_eq!(names[5], "an unknown speed");
        for (i, a) in names.iter().enumerate() {
            assert!(names[i + 1..].iter().all(|b| a != b), "{a} is named twice");
        }
    }

    #[test]
    fn the_preserved_set_is_linuxs() {
        assert_eq!(PRESERVE, 0x4e00_ffe9);
    }

    /// The bit positions, pinned against xHCI 1.2 Table 5-27, so a shift written the wrong way
    /// round fails here rather than as a port that never enables on the bench.
    #[test]
    fn the_port_status_and_control_bits_are_where_the_specification_puts_them() {
        assert_eq!([CONNECTED, ENABLED, RESET, POWER], [1, 2, 0x10, 0x200]);
        assert_eq!(
            [
                CONNECT_CHANGE,
                ENABLE_CHANGE,
                WARM_RESET_CHANGE,
                OVER_CURRENT_CHANGE
            ],
            [0x2_0000, 0x4_0000, 0x8_0000, 0x10_0000]
        );
        assert_eq!(
            [RESET_CHANGE, LINK_CHANGE, CONFIGURATION_ERROR_CHANGE],
            [0x20_0000, 0x40_0000, 0x80_0000]
        );
        assert_eq!(CHANGES, 0x00fe_0000);
    }

    /// Setting a bit the port already has keeps it set: power stays on when the caller writes
    /// power again.
    #[test]
    fn writing_a_preserved_bit_again_keeps_it() {
        assert_eq!(write_value(CONNECTED | POWER, POWER), CONNECTED | POWER);
    }

    #[test]
    fn endpoint_zero_starts_at_the_speeds_minimum() {
        assert_eq!(default_control_packet(PortSpeed::LOW), Some(8));
        assert_eq!(default_control_packet(PortSpeed::FULL), Some(8));
        assert_eq!(default_control_packet(PortSpeed::HIGH), Some(64));
        assert_eq!(default_control_packet(PortSpeed::SUPER), Some(512));
        assert_eq!(default_control_packet(PortSpeed::new(6).unwrap()), None);
    }

    fn ports(numbers: &[u8]) -> Connections {
        let mut bits = [0; 8];
        for &n in numbers {
            bits[usize::from(n / 32)] |= 1 << (n % 32);
        }
        bits
    }

    /// Run a settle over `timeline` (milliseconds after power, the ports connected from then on),
    /// reading every step as a caller does: when it stops, and what it last read.
    fn settle(timeline: &[(u64, &[u8])]) -> (u64, DebounceVerdict, Connections) {
        let at = |t: u64| {
            ports(
                timeline
                    .iter()
                    .rev()
                    .find(|(from, _)| *from <= t)
                    .unwrap()
                    .1,
            )
        };
        let mut s = PortDebounce::new(0, at(0));
        let mut t = 0;
        // Bounded, so a broken `sample` fails here rather than hanging the test run.
        while t < DEBOUNCE_TIMEOUT_MILLISECONDS {
            t += DEBOUNCE_STEP_MILLISECONDS;
            let verdict = s.sample(t, at(t));
            if verdict != DebounceVerdict::Pending {
                return (t, verdict, at(t));
            }
        }
        (t, DebounceVerdict::Pending, at(t))
    }

    /// **Ports that come back one after another after a controller reset**, as a real controller's
    /// did: nothing at first, ports 4 and 9 about 10 ms later, port 1 a millisecond after them. A
    /// scan at once reads none of them; a settled scan reads all three.
    #[test]
    fn ports_that_return_late_are_all_counted_once_they_settle() {
        let (t, settled, seen) = settle(&[(0, &[]), (10, &[4, 9]), (11, &[1, 4, 9])]);
        assert_eq!(settled, DebounceVerdict::Settled);
        assert_eq!(seen, ports(&[1, 4, 9]));
        assert_eq!(t, 125);
    }

    /// **A plug that bounces as it goes in**: connected, gone 30 ms later, back 30 ms after that.
    /// The port is believed only once the last bounce has held for the stable interval, so the
    /// caller resets the device after the bounce, not during it.
    #[test]
    fn a_bouncing_connect_is_believed_after_its_last_bounce_holds() {
        let (t, settled, seen) = settle(&[(0, &[3]), (30, &[]), (60, &[3])]);
        assert_eq!(settled, DebounceVerdict::Settled);
        assert_eq!(seen, ports(&[3]));
        assert_eq!(t, 175);
    }

    /// QEMU reports a connection the moment the controller runs: nothing changes, and the scan
    /// still waits the power-on floor rather than reading at once.
    #[test]
    fn a_port_connected_from_the_start_is_believed_after_the_floor() {
        assert_eq!(
            settle(&[(0, &[5])]),
            (100, DebounceVerdict::Settled, ports(&[5]))
        );
    }

    /// A port that keeps changing never settles, and the wait ends at the timeout, so
    /// the boot line can say its count is not a fact.
    #[test]
    fn a_port_that_never_holds_still_ends_at_the_timeout() {
        let mut s = PortDebounce::new(0, ports(&[]));
        let mut t = 0;
        let end = loop {
            assert!(t < DEBOUNCE_TIMEOUT_MILLISECONDS, "the wait never ended");
            t += DEBOUNCE_STEP_MILLISECONDS;
            let bouncing = if (t / DEBOUNCE_STEP_MILLISECONDS) % 2 == 0 {
                ports(&[3])
            } else {
                ports(&[])
            };
            let verdict = s.sample(t, bouncing);
            if verdict != DebounceVerdict::Pending {
                break verdict;
            }
        };
        assert_eq!(end, DebounceVerdict::TimedOut);
        assert_eq!(t, DEBOUNCE_TIMEOUT_MILLISECONDS);
    }
}
