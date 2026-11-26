//! **The numbers the controller gives a slot, an endpoint, a port and a speed**, each its own type
//! so that the compiler tells them apart and none holds a reserved value.
//!
//! Each one's `new` refuses the values the specification reserves, and `get` gives back the byte.

/// **A device slot** (xHCI 1.2 section 4.5.3): 1 to 255. Slot 0 is reserved; the doorbell at
/// index 0 is the host controller's own, not a slot's.
///
/// ```
/// use extensible_host_controller_interface::SlotId;
///
/// assert_eq!(SlotId::new(1).map(SlotId::get), Some(1));
/// assert_eq!(SlotId::new(0), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc(alias = "Slot ID")]
pub struct SlotId(u8);

impl SlotId {
    /// The slot numbered `id`, or `None` for the reserved 0.
    pub const fn new(id: u8) -> Option<SlotId> {
        if id == 0 { None } else { Some(SlotId(id)) }
    }

    /// The slot's number, 1 to 255.
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// **A device context index** (DCI, xHCI 1.2 section 4.5.1): 1 to 31, the Endpoint ID field of a
/// command or event. Endpoint 0 is 1, endpoint `n` `OUT` is `2n` and `IN` is `2n + 1`;
/// [`context::endpoint_index`](crate::context::endpoint_index) computes it from an endpoint
/// address. Index 0 is the slot context, which no endpoint field names.
///
/// ```
/// use extensible_host_controller_interface::{DeviceContextIndex, context};
///
/// assert_eq!(context::endpoint_index(0x81), DeviceContextIndex::new(3).unwrap());
/// assert_eq!(DeviceContextIndex::new(32), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc(alias = "DCI")]
#[doc(alias = "Endpoint ID")]
pub struct DeviceContextIndex(
    /// Crate-visible so that [`context::endpoint_index`](crate::context::endpoint_index), whose
    /// result is in range by construction, builds one without a refusal it can never take.
    pub(crate) u8,
);

impl DeviceContextIndex {
    /// The index `index`, or `None` outside 1 to 31.
    pub const fn new(index: u8) -> Option<DeviceContextIndex> {
        if index == 0 || index > 31 {
            None
        } else {
            Some(DeviceContextIndex(index))
        }
    }

    /// The index, 1 to 31.
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// **A root hub port** (xHCI 1.2 section 4.19): 1 to 255. Port 0 is reserved. Which numbers exist
/// is the controller's to say, so [`Capabilities::port_status_and_control`](crate::Capabilities::port_status_and_control)
/// still refuses one past its port count.
///
/// ```
/// use extensible_host_controller_interface::PortNumber;
///
/// assert_eq!(PortNumber::new(5).map(PortNumber::get), Some(5));
/// assert_eq!(PortNumber::new(0), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc(alias = "Port Number")]
pub struct PortNumber(u8);

impl PortNumber {
    /// The port numbered `number`, or `None` for the reserved 0.
    pub const fn new(number: u8) -> Option<PortNumber> {
        if number == 0 {
            None
        } else {
            Some(PortNumber(number))
        }
    }

    /// The port's number, 1 to 255.
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// **A port speed id** (`PORTSC` bits 10 to 13, xHCI 1.2 section 7.2.1): 1 to 15. Id 0 means no
/// device. The type is open: the associated constants are the default ids every controller uses
/// (Table 7-13), and a Supported Protocol capability whose speed id count is not 0 defines its own
/// ids in the same range.
///
/// ```
/// use extensible_host_controller_interface::{PortSpeed, port};
///
/// // A full-speed keyboard on a port that reads 0x0000_06e1.
/// assert_eq!(port::speed_of(0x0000_06e1), Some(PortSpeed::FULL));
/// assert_eq!(PortSpeed::new(16), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc(alias = "Port Speed")]
#[doc(alias = "PSIV")]
pub struct PortSpeed(u8);

impl PortSpeed {
    /// Full speed, 12 Mb/s: most keyboards.
    pub const FULL: PortSpeed = PortSpeed(1);
    /// Low speed, 1.5 Mb/s: cheap keyboards.
    pub const LOW: PortSpeed = PortSpeed(2);
    /// High speed, 480 Mb/s.
    pub const HIGH: PortSpeed = PortSpeed(3);
    /// `SuperSpeed`, 5 Gb/s.
    pub const SUPER: PortSpeed = PortSpeed(4);
    /// `SuperSpeedPlus`, 10 Gb/s.
    pub const SUPER_PLUS: PortSpeed = PortSpeed(5);

    /// The speed id `id`, or `None` outside 1 to 15.
    pub const fn new(id: u8) -> Option<PortSpeed> {
        if id == 0 || id > 15 {
            None
        } else {
            Some(PortSpeed(id))
        }
    }

    /// The speed id, 1 to 15.
    pub const fn get(self) -> u8 {
        self.0
    }
}
