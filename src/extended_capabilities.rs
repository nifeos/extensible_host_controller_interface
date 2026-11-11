//! **Extended capability ids** (xHCI 1.2 section 7): the ids in the list that starts at
//! [`Capabilities::extended`](crate::Capabilities::extended), for
//! [`find_extended`](crate::registers::find_extended) to look for, and [`SupportedProtocol`], the
//! decoder a driver reaches for to learn which root hub ports are USB 2 and which USB 3.

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupportedProtocol {
    /// The major revision in binary-coded decimal: 0x02 for USB 2, 0x03 for USB 3.
    #[doc(alias = "Revision Major")]
    pub major_revision: u8,
    /// The minor revision in binary-coded decimal: 0x00 for USB 2.0, 0x20 for USB 3.2.
    #[doc(alias = "Revision Minor")]
    pub minor_revision: u8,
    /// The name string's four ASCII characters in memory order, `b"USB "` for every revision the
    /// specification defines (Table 7-11).
    #[doc(alias = "Name String")]
    pub name: [u8; 4],
    /// The first root hub port number of the run, counting from 1.
    #[doc(alias = "Compatible Port Offset")]
    pub first_port_number: u8,
    /// How many consecutive ports the run holds.
    #[doc(alias = "Compatible Port Count")]
    pub port_count: u8,
    /// How many Protocol Speed ID dwords follow. When it is not 0, those dwords define every speed
    /// id the ports report, and the default ids in [`port::speed`](crate::port::speed) do not
    /// apply.
    #[doc(alias = "PSIC")]
    pub speed_id_count: u8,
}

impl SupportedProtocol {
    /// Decode the capability's dwords at offsets 0x00, 0x04 and 0x08.
    pub const fn decode(dwords: [u32; 3]) -> SupportedProtocol {
        SupportedProtocol {
            major_revision: (dwords[0] >> 24) as u8,
            minor_revision: (dwords[0] >> 16) as u8,
            name: dwords[1].to_le_bytes(),
            first_port_number: dwords[2] as u8,
            port_count: (dwords[2] >> 8) as u8,
            speed_id_count: (dwords[2] >> 28) as u8,
        }
    }

    /// Whether root hub port `port_number` belongs to this run. False for port 0, and for every
    /// port of a capability that names no ports.
    pub const fn contains_port(&self, port_number: u8) -> bool {
        port_number >= self.first_port_number
            && (port_number as u16) < self.first_port_number as u16 + self.port_count as u16
            && port_number != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// QEMU's qemu-xhci with its defaults: a USB 2.0 capability for ports 1 to 4 and a USB 3.0
    /// capability for ports 5 to 8, each with no speed id dwords.
    #[test]
    fn a_controllers_two_protocols_decode() {
        let usb2 = SupportedProtocol::decode([0x0200_0802, 0x2042_5355, 0x0000_0401]);
        assert_eq!(usb2.major_revision, 0x02);
        assert_eq!(usb2.minor_revision, 0x00);
        assert_eq!(&usb2.name, b"USB ");
        assert_eq!((usb2.first_port_number, usb2.port_count), (1, 4));
        assert_eq!(usb2.speed_id_count, 0);
        assert!(!usb2.contains_port(0));
        assert!(usb2.contains_port(1) && usb2.contains_port(4));
        assert!(!usb2.contains_port(5));

        let usb3 = SupportedProtocol::decode([0x0300_0002, 0x2042_5355, 0x3000_0405]);
        assert_eq!((usb3.major_revision, usb3.minor_revision), (0x03, 0x00));
        assert_eq!((usb3.first_port_number, usb3.port_count), (5, 4));
        assert_eq!(usb3.speed_id_count, 3);
        assert!(!usb3.contains_port(4));
        assert!(usb3.contains_port(5) && usb3.contains_port(8));
        assert!(!usb3.contains_port(9));
    }

    /// A run that ends at port 255 does not wrap, and an empty run holds nothing.
    #[test]
    fn a_run_at_the_last_port_number_does_not_wrap() {
        let last = SupportedProtocol::decode([0x0300_0002, 0x2042_5355, 0x0000_ffff]);
        assert!(last.contains_port(255));
        assert!(!last.contains_port(0) && !last.contains_port(1));
        let empty = SupportedProtocol::decode([0x0200_0002, 0x2042_5355, 0x0000_0001]);
        assert!(!empty.contains_port(1));
        // A broken controller's run that starts at 0 still never holds port 0.
        let broken = SupportedProtocol::decode([0x0200_0002, 0x2042_5355, 0x0000_0200]);
        assert!(!broken.contains_port(0) && broken.contains_port(1) && !broken.contains_port(2));
    }
}
