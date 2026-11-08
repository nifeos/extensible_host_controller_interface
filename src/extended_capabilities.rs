//! **Extended capability ids** (xHCI 1.2 section 7): the ids in the list that starts at
//! [`Capabilities::extended`](crate::Capabilities::extended), for
//! [`find_extended`](crate::registers::find_extended) to look for.

/// Extended capability id: USB Legacy Support, the firmware handoff (xHCI 1.2 section 7.1).
pub const LEGACY_SUPPORT_EXTENDED_CAPABILITY_ID: u8 = 1;
