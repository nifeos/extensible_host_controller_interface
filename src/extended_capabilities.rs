//! **Extended capability ids** (xHCI 1.2 section 7): the ids in the list that starts at
//! [`Capabilities::extended`](crate::Capabilities::extended), for
//! [`find_extended`](crate::registers::find_extended) to look for, and [`SupportedProtocol`], the
//! decoder a driver reaches for to learn which root hub ports are USB 2 and which USB 3.
//!
//! Reach for it during bring-up, to find the firmware handoff and to sort the root hub ports by
//! protocol before resetting any of them.

use crate::PortNumber;

/// Extended capability id: USB Legacy Support, the firmware handoff (xHCI 1.2 section 7.1).
pub const LEGACY_SUPPORT_EXTENDED_CAPABILITY_ID: u8 = 1;

/// Extended capability id: Supported Protocol, which root hub ports speak which USB revision
/// (xHCI 1.2 section 7.2). A controller has at least one.
pub const SUPPORTED_PROTOCOL_EXTENDED_CAPABILITY_ID: u8 = 2;

/// **One Supported Protocol capability, decoded** (xHCI 1.2 section 7.2, Tables 7-6 to 7-8): the
/// USB revision a run of root hub ports speaks. A controller with USB 2 and USB 3 ports has one
/// for each. `decode` takes the capability's first three dwords, read at the byte offset
/// [`find_extended`](crate::registers::find_extended) returns and the two after it.
#[doc(alias = "xHCI Supported Protocol Capability")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct SupportedProtocol {
    major_revision: u8,
    minor_revision: u8,
    name: [u8; 4],
    first_port_number: Option<PortNumber>,
    port_count: u8,
    speed_id_count: u8,
}

impl SupportedProtocol {
    /// Decode the capability's dwords at offsets 0x00, 0x04 and 0x08.
    pub const fn decode(dwords: [u32; 3]) -> SupportedProtocol {
        SupportedProtocol {
            major_revision: (dwords[0] >> 24) as u8,
            minor_revision: (dwords[0] >> 16) as u8,
            name: dwords[1].to_le_bytes(),
            first_port_number: PortNumber::new(dwords[2] as u8),
            port_count: (dwords[2] >> 8) as u8,
            speed_id_count: (dwords[2] >> 28) as u8,
        }
    }

    /// The major revision in binary-coded decimal: 0x02 for USB 2, 0x03 for USB 3.
    #[doc(alias = "Revision Major")]
    pub const fn major_revision(&self) -> u8 {
        self.major_revision
    }

    /// The minor revision in binary-coded decimal: 0x00 for USB 2.0, 0x20 for USB 3.2.
    #[doc(alias = "Revision Minor")]
    pub const fn minor_revision(&self) -> u8 {
        self.minor_revision
    }

    /// The name string's four ASCII characters in memory order, `b"USB "` for every revision the
    /// specification defines (Table 7-11).
    #[doc(alias = "Name String")]
    pub const fn name(&self) -> [u8; 4] {
        self.name
    }

    /// The first root hub port number of the run, or `None` when the field reads 0, which a
    /// broken controller may report and which names no port.
    #[doc(alias = "Compatible Port Offset")]
    pub const fn first_port_number(&self) -> Option<PortNumber> {
        self.first_port_number
    }

    /// How many consecutive ports the run holds.
    #[doc(alias = "Compatible Port Count")]
    pub const fn port_count(&self) -> u8 {
        self.port_count
    }

    /// How many Protocol Speed ID dwords follow. When it is not 0, those dwords define every speed
    /// id the ports report, and the default ids, the associated constants of
    /// [`PortSpeed`](crate::PortSpeed), do not apply.
    #[doc(alias = "PSIC")]
    pub const fn speed_id_count(&self) -> u8 {
        self.speed_id_count
    }

    /// Whether root hub port `port_number` belongs to this run. False for every port of a
    /// capability that names no ports.
    pub const fn contains_port(&self, port_number: PortNumber) -> bool {
        let first = match self.first_port_number {
            Some(first) => first.get() as u16,
            // A run that starts at the reserved 0 still starts there: port 1 is its second port.
            None => 0,
        };
        let port_number = port_number.get() as u16;
        port_number >= first && port_number < first + self.port_count as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn port(n: u8) -> PortNumber {
        PortNumber::new(n).unwrap()
    }

    /// Two runs laid out as QEMU's qemu-xhci lays them out with its defaults: USB 2 for ports 1 to 4
    /// and USB 3 for ports 5 to 8. The second is given revision 3.2 (minor revision 20h) and three
    /// speed id dwords, so that every field is read from a value that is not 0.
    #[test]
    fn a_controllers_two_protocols_decode() {
        let usb2 = SupportedProtocol::decode([0x0200_0802, 0x2042_5355, 0x0000_0401]);
        assert_eq!(usb2.major_revision(), 0x02);
        assert_eq!(usb2.minor_revision(), 0x00);
        assert_eq!(&usb2.name(), b"USB ");
        assert_eq!(
            (usb2.first_port_number(), usb2.port_count()),
            (Some(port(1)), 4)
        );
        assert_eq!(usb2.speed_id_count(), 0);
        assert!(usb2.contains_port(port(1)) && usb2.contains_port(port(4)));
        assert!(!usb2.contains_port(port(5)));

        let usb3 = SupportedProtocol::decode([0x0320_0002, 0x2042_5355, 0x3000_0405]);
        assert_eq!((usb3.major_revision(), usb3.minor_revision()), (0x03, 0x20));
        assert_eq!(
            (usb3.first_port_number(), usb3.port_count()),
            (Some(port(5)), 4)
        );
        assert_eq!(usb3.speed_id_count(), 3);
        assert!(!usb3.contains_port(port(4)));
        assert!(usb3.contains_port(port(5)) && usb3.contains_port(port(8)));
        assert!(!usb3.contains_port(port(9)));
    }

    /// A run that ends at port 255 does not wrap, and an empty run holds nothing.
    #[test]
    fn a_run_at_the_last_port_number_does_not_wrap() {
        let last = SupportedProtocol::decode([0x0300_0002, 0x2042_5355, 0x0000_ffff]);
        assert!(last.contains_port(port(255)));
        assert!(!last.contains_port(port(1)));
        let empty = SupportedProtocol::decode([0x0200_0002, 0x2042_5355, 0x0000_0001]);
        assert!(!empty.contains_port(port(1)));
        // A broken controller's run that starts at 0 still never holds port 0.
        let broken = SupportedProtocol::decode([0x0200_0002, 0x2042_5355, 0x0000_0200]);
        assert!(
            broken.first_port_number().is_none()
                && broken.contains_port(port(1))
                && !broken.contains_port(port(2))
        );
    }
}
