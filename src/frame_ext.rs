use super::usbd_gs::{HostCanId, HostCanIdBits, HostFrame, HostFrameFlags};
use embedded_hal::can::{ExtendedId, Frame, Id, StandardId};
use mcp2515::frame::CanFrame;

pub trait ToHostFrame {
    fn to_host_frame(&self, channel: u8) -> HostFrame;
}

pub trait FromHostFrame: Sized {
    fn from_host_frame(frame: &HostFrame) -> Option<Self>;
}

impl ToHostFrame for CanFrame {
    fn to_host_frame(&self, channel: u8) -> HostFrame {
        let flags = HostFrameFlags::new(&[]);
        let can_id = match (self.id(), self.is_remote_frame()) {
            (Id::Standard(id), true) => {
                HostCanId::new(id.as_raw() as u32, &[HostCanIdBits::RemoteFrame])
            }
            (Id::Standard(id), false) => HostCanId::new(id.as_raw() as u32, &[]),
            (Id::Extended(id), true) => HostCanId::new(
                id.as_raw(),
                &[HostCanIdBits::RemoteFrame, HostCanIdBits::ExtendedId],
            ),
            (Id::Extended(id), false) => HostCanId::new(id.as_raw(), &[HostCanIdBits::ExtendedId]),
        };

        let mut bytes: [u8; 64] = [0; 64];

        bytes.copy_from_slice(self.data());

        HostFrame::new(None, can_id, self.dlc() as u8, channel, flags, bytes)
    }
}

impl FromHostFrame for CanFrame {
    fn from_host_frame(frame: &HostFrame) -> Option<Self> {
        let id = unsafe {
            match frame.can_id.is_set(HostCanIdBits::ExtendedId) {
                true => Id::Extended(ExtendedId::new_unchecked(frame.can_id.id())),
                false => Id::Standard(StandardId::new_unchecked(frame.can_id.id() as u16)),
            }
        };

        if frame.can_id.is_set(HostCanIdBits::RemoteFrame) {
            CanFrame::new_remote(id, frame.can_dlc as usize)
        } else {
            CanFrame::new(id, &frame.bytes[..(frame.can_dlc as usize)])
        }
    }
}
