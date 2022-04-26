use super::Channel;
use super::ChannelEvent;
use super::GsUsbClass;
use super::HostFrame;

use usb_device::class_prelude::*;
use usb_device::Result;

pub struct GsUsbPort<'a, B: UsbBus, const C: usize> {
    underlying: GsUsbClass<'a, B, C>,
    read_buffer: [u8; frame_size()],
    read_state: ReadState,
    write_buffer: [u8; frame_size()],
    write_state: WriteState,
}

impl<B: UsbBus, const C: usize> GsUsbPort<'_, B, C> {
    /// Creates a new GsUsbPort with the provided UsbBus and max_packet_size in bytes. For
    /// full-speed devices, max_packet_size has to be one of 8, 16, 32 or 64.
    pub fn new(
        alloc: &UsbBusAllocator<B>,
        max_packet_size: u16,
        channels: [Channel; C],
        sw_version: u32,
        hw_version: u32,
    ) -> GsUsbPort<'_, B, C> {
        GsUsbPort {
            underlying: GsUsbClass::new(alloc, max_packet_size, channels, sw_version, hw_version),
            read_buffer: [0; frame_size()],
            read_state: ReadState::Empty,
            write_buffer: [0; frame_size()],
            write_state: WriteState::Ready,
        }
    }

    pub fn read_control_event(&mut self) -> Option<ChannelEvent> {
        self.underlying.read_control_event()
    }

    pub fn read_frame(&mut self) -> Result<HostFrame> {
        todo!()
        // match &self.read_state {
        //     ReadState::Full => {
        //         self.read_state = ReadState::Empty;
        //         match HostFrame::unpack(&self.read_buffer) {
        //             Ok(frame) => Ok(frame),
        //             Err(_) => Err(UsbError::ParseError),
        //         }
        //     }
        //     _ => Err(UsbError::WouldBlock),
        // }
    }

    pub fn write_frame(&mut self, frame: &HostFrame) -> Result<()> {
        todo!()
        // match &self.write_state {
        //     WriteState::Ready => match frame.pack_to_slice(&mut self.write_buffer) {
        //         Ok(_) => {
        //             self.write_state = WriteState::Writing(frame_size());
        //             Ok(())
        //         }
        //         Err(_) => Err(UsbError::ParseError),
        //     },
        //     WriteState::Writing(_) => Err(UsbError::WouldBlock),
        // }
    }
}

impl<B: UsbBus, const C: usize> UsbClass<B> for GsUsbPort<'_, B, C> {
    fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> Result<()> {
        self.underlying.get_configuration_descriptors(writer)
    }

    fn reset(&mut self) {
        self.underlying.stall();
    }

    fn poll(&mut self) {
        if self.read_state != ReadState::Full {
            let index = self.read_state.index();
            let packet_size = self.underlying.max_packet_size();
            let read_bytes = self.underlying.read_packet(&mut self.read_buffer[index..]);

            match read_bytes {
                Ok(size) if size == packet_size => {
                    self.read_state = ReadState::WaitingForPacket(index + size)
                }
                Ok(_) => self.read_state = ReadState::Full,
                Err(UsbError::WouldBlock) => {}
                Err(_) => self.read_state = ReadState::Empty,
            }
        }

        let was_writing_ready = self.write_state == WriteState::Ready;

        if let WriteState::Writing(remainder) = self.write_state {
            if remainder == 0 {
                self.underlying.write_packet(&[]).ok();
                self.write_state = WriteState::Ready;
            } else {
                let packet_size = self.underlying.max_packet_size();
                let from_index = frame_size() - remainder;
                let to_index = core::cmp::min(frame_size(), from_index + packet_size);
                let written_bytes = self
                    .underlying
                    .write_packet(&self.write_buffer[from_index..to_index]);

                match written_bytes {
                    Ok(bytes) => {
                        if bytes == packet_size {
                            self.write_state = WriteState::Writing(remainder - bytes);
                        } else {
                            self.write_state = WriteState::Ready;
                        }
                    }
                    Err(UsbError::WouldBlock) => {}
                    Err(_) => {
                        self.write_state = WriteState::Ready;
                    }
                }
            }
        }

        let is_writing_ready = self.write_state == WriteState::Ready;

        if was_writing_ready != is_writing_ready {
            if is_writing_ready {
                self.underlying.stall();
            } else {
                self.underlying.unstall();
            }
        }
    }

    fn control_out(&mut self, xfer: ControlOut<B>) {
        self.underlying.control_out(xfer)
    }

    fn control_in(&mut self, xfer: ControlIn<B>) {
        self.underlying.control_in(xfer)
    }
}

const fn frame_size() -> usize {
    core::mem::size_of::<HostFrame>()
}

#[derive(PartialEq, Eq)]
enum ReadState {
    Empty,
    WaitingForPacket(usize),
    Full,
}

impl ReadState {
    fn index(&self) -> usize {
        match self {
            ReadState::Empty => 0,
            ReadState::WaitingForPacket(index) => *index,
            ReadState::Full => frame_size(),
        }
    }
}

#[derive(PartialEq, Eq)]
enum WriteState {
    Ready,
    Writing(usize),
}
