//! **Completion codes** (xHCI 1.2 section 6.4.5, Table 6-90): what a transfer or command
//! completion event says became of the TRB it names.

/// **A completion code**: the byte at the top of an event's dword 2. Every byte is one, since the
/// specification reserves some and leaves two ranges to vendors, so the type is open: the
/// associated constants are the codes the specification defines, and [`name`](Self::name) gives
/// any byte its name for a transcript.
///
/// ```
/// use extensible_host_controller_interface::CompletionCode;
///
/// assert_eq!(CompletionCode::new(6), CompletionCode::STALL);
/// assert_eq!(CompletionCode::STALL.name(), "Stall Error");
/// assert_eq!(CompletionCode::new(200).name(), "Vendor Defined Error");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc(alias = "Completion Code")]
pub struct CompletionCode(u8);

impl CompletionCode {
    /// The code `code`. Every byte is a code, defined, reserved or vendor-defined.
    pub const fn new(code: u8) -> CompletionCode {
        CompletionCode(code)
    }

    /// The code's byte.
    pub const fn get(self) -> u8 {
        self.0
    }

    /// The controller has not written the Completion Code field yet.
    pub const INVALID: CompletionCode = CompletionCode(0);
    /// The command or transfer did what was asked.
    pub const SUCCESS: CompletionCode = CompletionCode(1);
    /// The controller could not keep up with the data: an overrun inbound, an underrun outbound.
    pub const DATA_BUFFER_ERROR: CompletionCode = CompletionCode(2);
    /// The device sent past the end of the packet it was allowed, and the endpoint halted.
    pub const BABBLE_DETECTED_ERROR: CompletionCode = CompletionCode(3);
    /// USB transaction error: the device did not answer, or answered garbage.
    #[doc(alias = "USB Transaction Error")]
    pub const TRANSACTION_ERROR: CompletionCode = CompletionCode(4);
    /// A TRB parameter was out of range or invalid.
    #[doc(alias = "TRB Error")]
    pub const TRANSFER_REQUEST_BLOCK_ERROR: CompletionCode = CompletionCode(5);
    /// The device stalled the endpoint: it refused the request.
    #[doc(alias = "Stall Error")]
    pub const STALL: CompletionCode = CompletionCode(6);
    /// The controller lacks the resources for a Configure Endpoint or Address Device command.
    pub const RESOURCE_ERROR: CompletionCode = CompletionCode(7);
    /// The controller cannot allocate the bandwidth that a Configure Endpoint command's periodic endpoints need.
    pub const BANDWIDTH_ERROR: CompletionCode = CompletionCode(8);
    /// No slot was free for Enable Slot.
    #[doc(alias = "No Slots Available Error")]
    pub const NO_SLOTS: CompletionCode = CompletionCode(9);
    /// A Stream Context Type value was invalid.
    pub const INVALID_STREAM_TYPE_ERROR: CompletionCode = CompletionCode(10);
    /// A command named a device slot in the Disabled state.
    pub const SLOT_NOT_ENABLED_ERROR: CompletionCode = CompletionCode(11);
    /// A doorbell rang for an endpoint in the Disabled state.
    pub const ENDPOINT_NOT_ENABLED_ERROR: CompletionCode = CompletionCode(12);
    /// The transfer finished with fewer bytes than the buffer: normal for a descriptor read.
    pub const SHORT_PACKET: CompletionCode = CompletionCode(13);
    /// An isochronous endpoint had to send with its transfer ring empty; the event's TRB pointer is not valid.
    pub const RING_UNDERRUN: CompletionCode = CompletionCode(14);
    /// An isochronous endpoint had to receive with its transfer ring empty; the event's TRB pointer is not valid.
    pub const RING_OVERRUN: CompletionCode = CompletionCode(15);
    /// A Force Event command found the target virtual function's event ring full.
    #[doc(alias = "VF Event Ring Full Error")]
    pub const VIRTUAL_FUNCTION_EVENT_RING_FULL_ERROR: CompletionCode = CompletionCode(16);
    /// A command found a context parameter invalid.
    pub const PARAMETER_ERROR: CompletionCode = CompletionCode(17);
    /// An isochronous transfer descriptor exceeded the bandwidth allocated to its endpoint.
    pub const BANDWIDTH_OVERRUN_ERROR: CompletionCode = CompletionCode(18);
    /// A command asked for a transition from a context state that does not allow it.
    pub const CONTEXT_STATE_ERROR: CompletionCode = CompletionCode(19);
    /// A periodic transfer missed its service interval because no `PING_RESPONSE` came in time.
    pub const NO_PING_RESPONSE_ERROR: CompletionCode = CompletionCode(20);
    /// The event ring was full, so the controller could not post an event.
    pub const EVENT_RING_FULL_ERROR: CompletionCode = CompletionCode(21);
    /// The device cannot be accessed; software disables its slot to recover.
    pub const INCOMPATIBLE_DEVICE_ERROR: CompletionCode = CompletionCode(22);
    /// The controller could not service an isochronous endpoint within its interval.
    pub const MISSED_SERVICE_ERROR: CompletionCode = CompletionCode(23);
    /// A Command Stop operation stopped the command ring.
    pub const COMMAND_RING_STOPPED: CompletionCode = CompletionCode(24);
    /// A Command Abort operation ended the command.
    pub const COMMAND_ABORTED: CompletionCode = CompletionCode(25);
    /// A Stop Endpoint command ended the transfer.
    pub const STOPPED: CompletionCode = CompletionCode(26);
    /// A Stop Endpoint command ended the transfer, and the event's count of bytes transferred (its
    /// Transfer Length field) is not valid.
    #[doc(alias = "Stopped - Length Invalid")]
    pub const STOPPED_TRANSFERRED_BYTES_INVALID: CompletionCode = CompletionCode(27);
    /// A Stop Endpoint command ended the transfer after a short packet, before the end of the transfer descriptor.
    pub const STOPPED_SHORT_PACKET: CompletionCode = CompletionCode(28);
    /// An Evaluate Context command's Max Exit Latency would leave the slot's periodic endpoints unschedulable.
    #[doc(alias = "Max Exit Latency Too Large Error")]
    pub const MAXIMUM_EXIT_LATENCY_TOO_LARGE_ERROR: CompletionCode = CompletionCode(29);
    /// An isochronous device sent more than the inbound buffer holds.
    #[doc(alias = "Isoch Buffer Overrun")]
    pub const ISOCHRONOUS_BUFFER_OVERRUN: CompletionCode = CompletionCode(31);
    /// The controller lost an event to an internal overrun.
    pub const EVENT_LOST_ERROR: CompletionCode = CompletionCode(32);
    /// No other code applies; software treats it as fatal.
    pub const UNDEFINED_ERROR: CompletionCode = CompletionCode(33);
    /// The controller received an invalid stream id.
    pub const INVALID_STREAM_ID_ERROR: CompletionCode = CompletionCode(34);
    /// A Configure Endpoint command's periodic endpoints need bandwidth that a secondary bandwidth domain cannot give.
    pub const SECONDARY_BANDWIDTH_ERROR: CompletionCode = CompletionCode(35);
    /// A split transaction on a USB2 endpoint failed.
    pub const SPLIT_TRANSACTION_ERROR: CompletionCode = CompletionCode(36);

    /// **The specification's name for this code** (xHCI 1.2 Table 6-90), for a transcript.
    /// Codes 192 to 223 are vendor-defined errors, which software treats as Undefined Error, and 224
    /// to 255 vendor-defined information, which software treats as Success (section 4.11.6). Every
    /// other code without a name is reserved.
    pub const fn name(self) -> &'static str {
        match self {
            CompletionCode::INVALID => "Invalid",
            CompletionCode::SUCCESS => "Success",
            CompletionCode::DATA_BUFFER_ERROR => "Data Buffer Error",
            CompletionCode::BABBLE_DETECTED_ERROR => "Babble Detected Error",
            CompletionCode::TRANSACTION_ERROR => "USB Transaction Error",
            CompletionCode::TRANSFER_REQUEST_BLOCK_ERROR => "TRB Error",
            CompletionCode::STALL => "Stall Error",
            CompletionCode::RESOURCE_ERROR => "Resource Error",
            CompletionCode::BANDWIDTH_ERROR => "Bandwidth Error",
            CompletionCode::NO_SLOTS => "No Slots Available Error",
            CompletionCode::INVALID_STREAM_TYPE_ERROR => "Invalid Stream Type Error",
            CompletionCode::SLOT_NOT_ENABLED_ERROR => "Slot Not Enabled Error",
            CompletionCode::ENDPOINT_NOT_ENABLED_ERROR => "Endpoint Not Enabled Error",
            CompletionCode::SHORT_PACKET => "Short Packet",
            CompletionCode::RING_UNDERRUN => "Ring Underrun",
            CompletionCode::RING_OVERRUN => "Ring Overrun",
            CompletionCode::VIRTUAL_FUNCTION_EVENT_RING_FULL_ERROR => "VF Event Ring Full Error",
            CompletionCode::PARAMETER_ERROR => "Parameter Error",
            CompletionCode::BANDWIDTH_OVERRUN_ERROR => "Bandwidth Overrun Error",
            CompletionCode::CONTEXT_STATE_ERROR => "Context State Error",
            CompletionCode::NO_PING_RESPONSE_ERROR => "No Ping Response Error",
            CompletionCode::EVENT_RING_FULL_ERROR => "Event Ring Full Error",
            CompletionCode::INCOMPATIBLE_DEVICE_ERROR => "Incompatible Device Error",
            CompletionCode::MISSED_SERVICE_ERROR => "Missed Service Error",
            CompletionCode::COMMAND_RING_STOPPED => "Command Ring Stopped",
            CompletionCode::COMMAND_ABORTED => "Command Aborted",
            CompletionCode::STOPPED => "Stopped",
            CompletionCode::STOPPED_TRANSFERRED_BYTES_INVALID => "Stopped - Length Invalid",
            CompletionCode::STOPPED_SHORT_PACKET => "Stopped - Short Packet",
            CompletionCode::MAXIMUM_EXIT_LATENCY_TOO_LARGE_ERROR => {
                "Max Exit Latency Too Large Error"
            }
            CompletionCode::ISOCHRONOUS_BUFFER_OVERRUN => "Isoch Buffer Overrun",
            CompletionCode::EVENT_LOST_ERROR => "Event Lost Error",
            CompletionCode::UNDEFINED_ERROR => "Undefined Error",
            CompletionCode::INVALID_STREAM_ID_ERROR => "Invalid Stream ID Error",
            CompletionCode::SECONDARY_BANDWIDTH_ERROR => "Secondary Bandwidth Error",
            CompletionCode::SPLIT_TRANSACTION_ERROR => "Split Transaction Error",
            CompletionCode(192..=223) => "Vendor Defined Error",
            CompletionCode(224..=255) => "Vendor Defined Info",
            _ => "Reserved",
        }
    }
}
