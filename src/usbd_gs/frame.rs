use scroll::Pread;

#[derive(Pread)]
pub struct HostFrame {
    pub echo_id: u32,
    pub can_id: HostCanId,
    pub can_dlc: u8,
    pub channel: u8,
    pub flags: HostFrameFlags,
    _reserved: u8,
    pub bytes: [u8; 64],
}

impl HostFrame {
    pub fn new(
        echo_id: Option<u32>,
        can_id: HostCanId,
        can_dlc: u8,
        channel: u8,
        flags: HostFrameFlags,
        bytes: [u8; 64],
    ) -> Self {
        Self {
            echo_id: echo_id.unwrap_or(u32::MAX),
            can_id,
            can_dlc,
            channel,
            flags,
            _reserved: 0,
            bytes,
        }
    }
}

#[derive(Pread)]
pub struct HostCanId(u32);

impl HostCanId {
    pub fn new(raw_id: u32, bits: &[HostCanIdBits]) -> Self {
        HostCanId(
            bits.iter()
                .fold(raw_id & 0x1fffffff, |l, r| l | (*r as u32)),
        )
    }

    pub fn id(&self) -> u32 {
        self.0 & 0x1fffffff
    }

    pub fn is_set(&self, bit: HostCanIdBits) -> bool {
        self.0 & (bit as u32) != 0
    }
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum HostCanIdBits {
    #[allow(dead_code)]
    ErrorFrame = 1 << 29,
    RemoteFrame = 1 << 30,
    ExtendedId = 1 << 31,
}

#[derive(Pread)]
pub struct HostFrameFlags(u8);

impl HostFrameFlags {
    pub fn new(bits: &[HostFrameFlagsBits]) -> Self {
        HostFrameFlags(bits.iter().fold(0, |l, r| l | (*r as u8)))
    }

    #[allow(dead_code)]
    pub fn is_set(&self, bit: HostFrameFlagsBits) -> bool {
        self.0 & (bit as u8) != 0
    }

    pub fn set(&mut self, bit: HostFrameFlagsBits) {
        self.0 |= bit as u8
    }
}

#[repr(u8)]
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum HostFrameFlagsBits {
    Overflow = 1 << 0,
    Fd = 1 << 1,
    Brs = 1 << 2,
    Esi = 1 << 3,
}
