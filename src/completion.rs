/// The controller has not written the Completion Code field yet.
pub const INVALID: u8 = 0;
/// The command or transfer did what was asked.
pub const SUCCESS: u8 = 1;
/// The controller could not keep up with the data: an overrun inbound, an underrun outbound.
pub const DATA_BUFFER_ERROR: u8 = 2;
/// The device sent past the end of the packet it was allowed, and the endpoint halted.
pub const BABBLE_DETECTED_ERROR: u8 = 3;
/// USB transaction error: the device did not answer, or answered garbage.
#[doc(alias = "USB Transaction Error")]
pub const TRANSACTION_ERROR: u8 = 4;
/// A TRB parameter was out of range or invalid.
#[doc(alias = "TRB Error")]
pub const TRANSFER_REQUEST_BLOCK_ERROR: u8 = 5;
/// The device stalled the endpoint: it refused the request.
#[doc(alias = "Stall Error")]
pub const STALL: u8 = 6;
/// The controller lacks the resources for a Configure Endpoint or Address Device command.
pub const RESOURCE_ERROR: u8 = 7;
/// The controller cannot allocate the bandwidth that a Configure Endpoint command's periodic endpoints need.
pub const BANDWIDTH_ERROR: u8 = 8;
/// No slot was free for Enable Slot.
#[doc(alias = "No Slots Available Error")]
pub const NO_SLOTS: u8 = 9;
/// A Stream Context Type value was invalid.
pub const INVALID_STREAM_TYPE_ERROR: u8 = 10;
/// A command named a device slot in the Disabled state.
pub const SLOT_NOT_ENABLED_ERROR: u8 = 11;
/// A doorbell rang for an endpoint in the Disabled state.
pub const ENDPOINT_NOT_ENABLED_ERROR: u8 = 12;
/// The transfer finished with fewer bytes than the buffer: normal for a descriptor read.
pub const SHORT_PACKET: u8 = 13;
/// An isochronous endpoint had to send with its transfer ring empty; the event's TRB pointer is not valid.
pub const RING_UNDERRUN: u8 = 14;
/// An isochronous endpoint had to receive with its transfer ring empty; the event's TRB pointer is not valid.
pub const RING_OVERRUN: u8 = 15;
/// A Force Event command found the target virtual function's event ring full.
#[doc(alias = "VF Event Ring Full Error")]
pub const VIRTUAL_FUNCTION_EVENT_RING_FULL_ERROR: u8 = 16;
/// A command found a context parameter invalid.
pub const PARAMETER_ERROR: u8 = 17;
/// An isochronous transfer descriptor exceeded the bandwidth allocated to its endpoint.
pub const BANDWIDTH_OVERRUN_ERROR: u8 = 18;
/// A command asked for a transition from a context state that does not allow it.
pub const CONTEXT_STATE_ERROR: u8 = 19;
/// A periodic transfer missed its service interval because no `PING_RESPONSE` came in time.
pub const NO_PING_RESPONSE_ERROR: u8 = 20;
/// The event ring was full, so the controller could not post an event.
pub const EVENT_RING_FULL_ERROR: u8 = 21;
/// The device cannot be accessed; software disables its slot to recover.
pub const INCOMPATIBLE_DEVICE_ERROR: u8 = 22;
/// The controller could not service an isochronous endpoint within its interval.
pub const MISSED_SERVICE_ERROR: u8 = 23;
/// A Command Stop operation stopped the command ring.
pub const COMMAND_RING_STOPPED: u8 = 24;
/// A Command Abort operation ended the command.
pub const COMMAND_ABORTED: u8 = 25;
/// A Stop Endpoint command ended the transfer.
pub const STOPPED: u8 = 26;
/// A Stop Endpoint command ended the transfer, and the event's count of bytes transferred (its
/// Transfer Length field) is not valid.
#[doc(alias = "Stopped - Length Invalid")]
pub const STOPPED_TRANSFERRED_BYTES_INVALID: u8 = 27;
/// A Stop Endpoint command ended the transfer after a short packet, before the end of the transfer descriptor.
pub const STOPPED_SHORT_PACKET: u8 = 28;
/// An Evaluate Context command's Max Exit Latency would leave the slot's periodic endpoints unschedulable.
#[doc(alias = "Max Exit Latency Too Large Error")]
pub const MAXIMUM_EXIT_LATENCY_TOO_LARGE_ERROR: u8 = 29;
/// An isochronous device sent more than the inbound buffer holds.
#[doc(alias = "Isoch Buffer Overrun")]
pub const ISOCHRONOUS_BUFFER_OVERRUN: u8 = 31;
/// The controller lost an event to an internal overrun.
pub const EVENT_LOST_ERROR: u8 = 32;
/// No other code applies; software treats it as fatal.
pub const UNDEFINED_ERROR: u8 = 33;
/// The controller received an invalid stream id.
pub const INVALID_STREAM_ID_ERROR: u8 = 34;
/// A Configure Endpoint command's periodic endpoints need bandwidth that a secondary bandwidth domain cannot give.
pub const SECONDARY_BANDWIDTH_ERROR: u8 = 35;
/// A split transaction on a USB2 endpoint failed.
pub const SPLIT_TRANSACTION_ERROR: u8 = 36;
/// **The specification's name for completion `code`** (xHCI 1.2 Table 6-90), for a transcript.
/// Codes 192 to 223 are vendor-defined errors, which software treats as Undefined Error, and 224
/// to 255 vendor-defined information, which software treats as Success (section 4.11.6). Every
/// other code without a name is reserved.
pub const fn name(code: u8) -> &'static str {
    match code {
        INVALID => "Invalid",
        SUCCESS => "Success",
        DATA_BUFFER_ERROR => "Data Buffer Error",
        BABBLE_DETECTED_ERROR => "Babble Detected Error",
        TRANSACTION_ERROR => "USB Transaction Error",
        TRANSFER_REQUEST_BLOCK_ERROR => "TRB Error",
        STALL => "Stall Error",
        RESOURCE_ERROR => "Resource Error",
        BANDWIDTH_ERROR => "Bandwidth Error",
        NO_SLOTS => "No Slots Available Error",
        INVALID_STREAM_TYPE_ERROR => "Invalid Stream Type Error",
        SLOT_NOT_ENABLED_ERROR => "Slot Not Enabled Error",
        ENDPOINT_NOT_ENABLED_ERROR => "Endpoint Not Enabled Error",
        SHORT_PACKET => "Short Packet",
        RING_UNDERRUN => "Ring Underrun",
        RING_OVERRUN => "Ring Overrun",
        VIRTUAL_FUNCTION_EVENT_RING_FULL_ERROR => "VF Event Ring Full Error",
        PARAMETER_ERROR => "Parameter Error",
        BANDWIDTH_OVERRUN_ERROR => "Bandwidth Overrun Error",
        CONTEXT_STATE_ERROR => "Context State Error",
        NO_PING_RESPONSE_ERROR => "No Ping Response Error",
        EVENT_RING_FULL_ERROR => "Event Ring Full Error",
        INCOMPATIBLE_DEVICE_ERROR => "Incompatible Device Error",
        MISSED_SERVICE_ERROR => "Missed Service Error",
        COMMAND_RING_STOPPED => "Command Ring Stopped",
        COMMAND_ABORTED => "Command Aborted",
        STOPPED => "Stopped",
        STOPPED_TRANSFERRED_BYTES_INVALID => "Stopped - Length Invalid",
        STOPPED_SHORT_PACKET => "Stopped - Short Packet",
        MAXIMUM_EXIT_LATENCY_TOO_LARGE_ERROR => "Max Exit Latency Too Large Error",
        ISOCHRONOUS_BUFFER_OVERRUN => "Isoch Buffer Overrun",
        EVENT_LOST_ERROR => "Event Lost Error",
        UNDEFINED_ERROR => "Undefined Error",
        INVALID_STREAM_ID_ERROR => "Invalid Stream ID Error",
        SECONDARY_BANDWIDTH_ERROR => "Secondary Bandwidth Error",
        SPLIT_TRANSACTION_ERROR => "Split Transaction Error",
        192..=223 => "Vendor Defined Error",
        224..=255 => "Vendor Defined Info",
        _ => "Reserved",
    }
}
