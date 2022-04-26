#![no_std]
#![no_main]

mod frame_ext;
mod usbd_gs;

use cortex_m_rt::entry;
use defmt_rtt as _;
use embedded_time::rate::*;
use frame_ext::*;
use mcp2515::{frame::CanFrame, regs::OpMode, *};
use panic_probe as _;
use ringbuffer::*;
use rp_pico::hal::{
    clocks,
    clocks::Clock,
    gpio::{FunctionSpi, Pins},
    pac,
    spi::Spi,
    usb, Sio, Watchdog,
};
use usb_device::{class_prelude::*, prelude::*};
use usbd_gs::*;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let clocks = clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());
    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let _spi_sclk = pins.gpio2.into_mode::<FunctionSpi>();
    let _spi_mosi = pins.gpio3.into_mode::<FunctionSpi>();
    let _spi_miso = pins.gpio4.into_mode::<FunctionSpi>();

    let mcp2515_cs = pins.gpio9.into_push_pull_output();
    let mcp2515_spi = Spi::<_, _, 8>::new(pac.SPI0).init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        16_000_000u32.Hz(),
        &embedded_hal::spi::MODE_0,
    );

    let usb_bus = UsbBusAllocator::new(usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let channels = [Channel {
        features: ChannelFeatures::new(&[
            ChannelFeaturesBit::ListenOnly,
            ChannelFeaturesBit::Loopback,
        ]),
        fclk_can: 8000000,
        constraints: ChannelConstraints {
            tseg1_min: 3,
            tseg1_max: 8,
            tseg2_min: 2,
            tseg2_max: 8,
            sjw_max: 4,
            brp_min: 1,
            brp_max: 64,
            brp_inc: 1,
        },
        data_constraints: None,
    }];

    let mut gs_port = GsUsbPort::new(&usb_bus, 64, channels, 2, 1);
    let mut mcp2515 = MCP2515::new(mcp2515_spi, mcp2515_cs, delay);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1209, 0x2323))
        .manufacturer("Longan Labs")
        .product("CANBED Dual")
        .serial_number("TBD")
        .device_class(0)
        .build();

    let mut inbox = ConstGenericRingBuffer::<HostFrame, 8>::new();
    let mut outbox = ConstGenericRingBuffer::<HostFrame, 8>::new();

    assert_eq!(mcp2515.init(Settings::default()), Ok(()));

    loop {
        if usb_dev.poll(&mut [&mut gs_port]) {
            if let Some(event) = gs_port.read_control_event() {
                match event {
                    ChannelEvent::BitTiming(timing, ch) => {
                        assert_eq!(
                            mcp2515.set_bitrate(
                                can_speed_from_bit_rate(timing.bit_rate(&channels[ch])),
                                McpSpeed::MHz16,
                                false,
                            ),
                            Ok(())
                        );
                    }
                    ChannelEvent::DataBitTiming(_, _) => {}
                    ChannelEvent::ChannelMode(mode, _) => {
                        let mut mcp_mode: OpMode = OpMode::Normal;

                        if mode.flags.is_set(ChannelFlagsBit::Loopback) {
                            mcp_mode = OpMode::Loopback;
                        }

                        if mode.flags.is_set(ChannelFlagsBit::ListenOnly) {
                            mcp_mode = OpMode::ListenOnly;
                        }

                        if !mode.is_on() {
                            mcp_mode = OpMode::Sleep;
                        }

                        assert_eq!(mcp2515.set_mode(mcp_mode), Ok(()));
                    }
                    ChannelEvent::Identify(_, _) => {}
                };
            }

            if let Ok(host_frame) = gs_port.read_frame() {
                outbox.push(host_frame);
            }

            if let Some(host_frame) = inbox.peek() {
                match gs_port.write_frame(host_frame) {
                    Ok(_) => inbox.skip(),
                    Err(UsbError::WouldBlock) => {}
                    Err(_) => inbox.skip(),
                };
            }
        }

        if let Ok(mcp_frame) = mcp2515.read_message() {
            inbox.push(mcp_frame.to_host_frame(1));
        }

        if let Some(host_frame) = outbox.peek() {
            if let Some(mcp_frame) = CanFrame::from_host_frame(host_frame) {
                match mcp2515.send_message(mcp_frame) {
                    Ok(_) => {
                        inbox.push(outbox.dequeue().unwrap());
                    }
                    Err(mcp2515::error::Error::TxBusy) => {}
                    Err(mcp2515::error::Error::NewModeTimeout) => {}
                    Err(_) => {
                        let mut err_frame = outbox.dequeue().unwrap();
                        err_frame.flags.set(HostFrameFlagsBits::Overflow);
                        inbox.push(err_frame);
                    }
                }
            } else {
                outbox.skip();
            }
        }
    }
}

fn can_speed_from_bit_rate(bit_rate: u32) -> CanSpeed {
    match bit_rate / 1000 {
        0..=5000 => CanSpeed::Kbps5,
        5001..=10000 => CanSpeed::Kbps10,
        10001..=20000 => CanSpeed::Kbps20,
        20001..=31250 => CanSpeed::Kbps31_25,
        31251..=33300 => CanSpeed::Kbps33_3,
        33301..=40000 => CanSpeed::Kbps40,
        40001..=50000 => CanSpeed::Kbps50,
        50001..=80000 => CanSpeed::Kbps80,
        80001..=100000 => CanSpeed::Kbps100,
        100001..=125000 => CanSpeed::Kbps125,
        125001..=200000 => CanSpeed::Kbps200,
        200001..=250000 => CanSpeed::Kbps250,
        250001..=500000 => CanSpeed::Kbps500,
        _ => CanSpeed::Kbps1000,
    }
}
