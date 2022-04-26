use crate::Channel;
use scroll::Pread;

pub enum ChannelEvent {
    BitTiming(BitTiming, usize),
    DataBitTiming(BitTiming, usize),
    ChannelMode(ChannelMode, usize),
    Identify(ChannelIdentify, usize),
}

#[derive(Pread)]
pub struct BitTiming {
    pub prop_seg: u32,
    pub phase_seg1: u32,
    pub phase_seg2: u32,
    pub sjw: u32,
    pub brp: u32,
}

impl BitTiming {
    pub fn bit_rate(&self, channel: &Channel) -> u32 {
        let fclk = channel.fclk_can;
        let fbrp = fclk / self.brp;

        fbrp / (1 + self.prop_seg + self.phase_seg1 + self.phase_seg2)
    }
}

#[derive(Pread)]
pub struct ChannelMode {
    mode: u32,
    pub flags: ChannelFlags,
}

impl ChannelMode {
    pub fn is_on(&self) -> bool {
        self.mode != 0
    }
}

#[derive(Pread)]
pub struct ChannelFlags(u32);

impl ChannelFlags {
    pub fn is_set(&self, bit: ChannelFlagsBit) -> bool {
        self.0 & bit as u32 != 0
    }
}

#[repr(u32)]
#[allow(dead_code)]
pub enum ChannelFlagsBit {
    ListenOnly = 1 << 0,
    Loopback = 1 << 1,
    TripleSample = 1 << 2,
    OneShot = 1 << 3,
    HwTimestamp = 1 << 4,
    PadPktsToMaxPktSize = 1 << 7,
    Fd = 1 << 8,
}

#[derive(Pread)]
pub struct ChannelIdentify(u32);

impl ChannelIdentify {
    #[allow(dead_code)]
    pub fn is_on(&self) -> bool {
        self.0 != 0
    }
}
