use scroll::Pwrite;

#[derive(Clone, Copy)]
pub struct Channel {
    pub features: ChannelFeatures,
    pub fclk_can: u32,
    pub constraints: ChannelConstraints,
    pub data_constraints: Option<ChannelConstraints>,
}

#[derive(Pwrite, Clone, Copy)]
pub struct ChannelFeatures(u32);

impl ChannelFeatures {
    pub fn new(bits: &[ChannelFeaturesBit]) -> Self {
        ChannelFeatures(bits.iter().fold(0, |l, r| l | (*r as u32)))
    }

    pub fn is_set(&self, bit: ChannelFeaturesBit) -> bool {
        self.0 & bit as u32 != 0
    }
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum ChannelFeaturesBit {
    ListenOnly = 1 << 0,
    Loopback = 1 << 1,
    TripleSample = 1 << 2,
    OneShot = 1 << 3,
    HwTimestamp = 1 << 4,
    Identify = 1 << 5,
    UserId = 1 << 6,
    PadPktsToMaxPktSize = 1 << 7,
    Fd = 1 << 8,
    ReqUsbQuirkLpc546xx = 1 << 9,
    BtConstExt = 1 << 10,
}

#[derive(Pwrite, Clone, Copy)]
pub struct ChannelConstraints {
    pub tseg1_min: u32,
    pub tseg1_max: u32,
    pub tseg2_min: u32,
    pub tseg2_max: u32,
    pub sjw_max: u32,
    pub brp_min: u32,
    pub brp_max: u32,
    pub brp_inc: u32,
}
