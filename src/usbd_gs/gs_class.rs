use super::{Channel, ChannelConstraints, ChannelEvent, ChannelFeatures};
use scroll::{Pread, Pwrite, LE};
use usb_device::class_prelude::*;

const USB_CLASS_GS: u8 = 0xFF;
const GS_SUBCLASS: u8 = 0xFF;
const GS_PROTOCOL: u8 = 0xFF;

pub struct GsUsbClass<'a, B: UsbBus, const C: usize> {
    comm_if: InterfaceNumber,
    read_ep: EndpointOut<'a, B>,
    write_ep: EndpointIn<'a, B>,
    channels: [Channel; C],
    config: DeviceConfig,
    control_event: Option<ChannelEvent>,
}

#[repr(u8)]
#[derive(Eq, PartialEq)]
#[allow(dead_code)]
enum GsUsbRequest {
    HostFormat = 0,
    BitTiming = 1,
    Mode = 2,
    Berr = 3,
    BtConst = 4,
    DeviceConfig = 5,
    Timestamp = 6,
    Identify = 7,
    GetUserId = 8,
    SetUserId = 9,
    DataBitTiming = 10,
    BtConstExt = 11,
}

impl<B: UsbBus, const C: usize> GsUsbClass<'_, B, C> {
    /// Creates a new GsUsbClass with the provided UsbBus and max_packet_size in bytes. For
    /// full-speed devices, max_packet_size has to be one of 8, 16, 32 or 64.
    pub fn new(
        alloc: &UsbBusAllocator<B>,
        max_packet_size: u16,
        channels: [Channel; C],
        sw_version: u32,
        hw_version: u32,
    ) -> GsUsbClass<'_, B, C> {
        assert!(C < u8::MAX as usize);
        GsUsbClass {
            comm_if: alloc.interface(),
            read_ep: alloc.bulk(max_packet_size),
            write_ep: alloc.bulk(max_packet_size),
            channels,
            config: DeviceConfig {
                reserved: [0; 3],
                icount: (C - 1) as u8,
                sw_version,
                hw_version,
            },
            control_event: None,
        }
    }

    pub fn max_packet_size(&self) -> usize {
        self.write_ep.max_packet_size() as usize
    }

    pub fn stall(&mut self) {
        self.write_ep.stall();
    }

    pub fn unstall(&mut self) {
        self.write_ep.unstall();
    }

    pub fn write_packet(&mut self, data: &[u8]) -> usb_device::Result<usize> {
        self.write_ep.write(data)
    }

    pub fn read_packet(&mut self, data: &mut [u8]) -> usb_device::Result<usize> {
        self.read_ep.read(data)
    }

    pub fn read_control_event(&mut self) -> Option<ChannelEvent> {
        let mut ret_value: Option<ChannelEvent> = None;
        core::mem::swap(&mut ret_value, &mut self.control_event);
        ret_value
    }
}

impl<B: UsbBus, const C: usize> UsbClass<B> for GsUsbClass<'_, B, C> {
    fn get_configuration_descriptors(
        &self,
        writer: &mut DescriptorWriter,
    ) -> usb_device::Result<()> {
        writer.iad(self.comm_if, 1, USB_CLASS_GS, GS_SUBCLASS, GS_PROTOCOL)?;
        writer.interface(self.comm_if, USB_CLASS_GS, GS_SUBCLASS, GS_PROTOCOL)?;

        writer.endpoint(&self.write_ep)?;
        writer.endpoint(&self.read_ep)?;

        Ok(())
    }

    fn control_out(&mut self, xfer: ControlOut<B>) {
        let req = xfer.request();

        if req.request_type != control::RequestType::Vendor
            || req.recipient != control::Recipient::Interface
            || req.index != u8::from(self.comm_if) as u16
        {
            return;
        }

        let gs_request = GsUsbRequest::from_raw(req.request);
        let channel = req.value as usize;

        if let Some(GsUsbRequest::HostFormat) = gs_request {
            xfer.accept().ok();
            return;
        }

        let control_event = match gs_request {
            Some(GsUsbRequest::BitTiming) if channel < C => xfer
                .data()
                .pread_with(0, LE)
                .map(|timing| ChannelEvent::BitTiming(timing, channel)),

            Some(GsUsbRequest::DataBitTiming) if channel < C => xfer
                .data()
                .pread_with(0, LE)
                .map(|timing| ChannelEvent::DataBitTiming(timing, channel)),

            Some(GsUsbRequest::Mode) if channel < C => xfer
                .data()
                .pread_with(0, LE)
                .map(|mode| ChannelEvent::ChannelMode(mode, channel)),

            Some(GsUsbRequest::Identify) if channel < C => xfer
                .data()
                .pread_with(0, LE)
                .map(|identify| ChannelEvent::Identify(identify, channel)),

            _ => Err(scroll::Error::BadInput {
                size: xfer.data().len(),
                msg: "invalid gs_usb request",
            }),
        };

        self.control_event = control_event.ok();

        match &self.control_event {
            Some(_) => {
                xfer.accept().ok();
            }
            None => {
                xfer.reject().ok();
            }
        }
    }

    fn control_in(&mut self, xfer: ControlIn<B>) {
        let req = xfer.request();

        if req.request_type != control::RequestType::Vendor
            || req.recipient != control::Recipient::Interface
            || req.index != u8::from(self.comm_if) as u16
        {
            return;
        }

        let channel = req.value as usize;
        let gs_request = GsUsbRequest::from_raw(req.request);

        fn reply<const N: usize, B: UsbBus>(
            value: Result<[u8; N], scroll::Error>,
            xfer: ControlIn<B>,
        ) -> Result<(), usb_device::UsbError> {
            match &value {
                Ok(packed) => xfer.accept_with(packed),
                Err(_) => xfer.reject(),
            }
        }

        let response = match gs_request {
            Some(GsUsbRequest::DeviceConfig) => reply(self.config.packed(), xfer),
            Some(GsUsbRequest::BtConst) if channel < C => {
                reply(BtConst::new(&self.channels[channel]).packed(), xfer)
            }
            Some(GsUsbRequest::BtConstExt) if channel < C => {
                reply(BtConstExt::new(&self.channels[channel]).packed(), xfer)
            }
            _ => xfer.reject(),
        };

        response.ok();
    }
}

impl GsUsbRequest {
    fn from_raw(raw: u8) -> Option<GsUsbRequest> {
        if raw > GsUsbRequest::BtConstExt as u8 {
            return None;
        }

        unsafe { Some(core::mem::transmute(raw)) }
    }
}

#[derive(Pwrite)]
struct DeviceConfig {
    reserved: [u8; 3],
    icount: u8,
    sw_version: u32,
    hw_version: u32,
}

struct BtConst<'a> {
    features: &'a ChannelFeatures,
    fclk_can: &'a u32,
    constraints: &'a ChannelConstraints,
}

struct BtConstExt<'a> {
    features: &'a ChannelFeatures,
    fclk_can: &'a u32,
    constraints: &'a ChannelConstraints,
    data_constraints: &'a ChannelConstraints,
}

impl DeviceConfig {
    const fn size() -> usize {
        core::mem::size_of::<Self>()
    }

    fn packed(&self) -> Result<[u8; DeviceConfig::size()], scroll::Error> {
        let mut ret_value: [u8; DeviceConfig::size()] = [0; DeviceConfig::size()];
        ret_value.pwrite_with(self, 0, LE)?;
        Ok(ret_value)
    }
}

impl BtConst<'_> {
    fn new(channel: &Channel) -> BtConst {
        BtConst {
            features: &channel.features,
            fclk_can: &channel.fclk_can,
            constraints: &channel.constraints,
        }
    }

    fn packed(&self) -> Result<[u8; 40], scroll::Error> {
        let mut ret_value: [u8; 40] = [0; 40];
        let mut bytes_written: usize = 0;

        bytes_written = ret_value.pwrite_with(self.features, bytes_written, LE)?;
        bytes_written = ret_value.pwrite_with(self.fclk_can, bytes_written, LE)?;
        _ = ret_value.pwrite_with(self.constraints, bytes_written, LE)?;

        Ok(ret_value)
    }
}

impl BtConstExt<'_> {
    fn new(channel: &Channel) -> BtConstExt {
        match &channel.data_constraints {
            Some(data_constraints) => BtConstExt {
                features: &channel.features,
                fclk_can: &channel.fclk_can,
                constraints: &channel.constraints,
                data_constraints,
            },
            None => BtConstExt {
                features: &channel.features,
                fclk_can: &channel.fclk_can,
                constraints: &channel.constraints,
                data_constraints: &channel.constraints,
            },
        }
    }

    fn packed(&self) -> Result<[u8; 72], scroll::Error> {
        let mut ret_value: [u8; 72] = [0; 72];
        let mut bytes_written: usize = 0;

        bytes_written = ret_value.pwrite_with(self.features, bytes_written, LE)?;
        bytes_written = ret_value.pwrite_with(self.fclk_can, bytes_written, LE)?;
        bytes_written = ret_value.pwrite_with(self.constraints, bytes_written, LE)?;
        _ = ret_value.pwrite_with(self.data_constraints, bytes_written, LE)?;

        Ok(ret_value)
    }
}
