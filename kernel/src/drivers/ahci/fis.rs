
#[derive(Debug)]
pub enum FisType {
    RegisterH2D = 0x27,
    RegisterD2H = 0x34,
    DMAActivate = 0x39,
    DMASetup = 0x41,
    Data = 0x46,
    Bist = 0x58,
    PIOSetup = 0x5F,
    SetDeviceBits = 0xA1,
}

#[repr(C)]
struct DataFis {
    fis_type: FisType,
    port_multiplier: u8,
    reserved: [u8; 2],
    data: [u8],
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct H2DRegisterFis {
    pub fis_type: u8,
    ///set to 1 for command, 0 for control
    pub pmport: u8,
    pub command: u8,
    pub featurel: u8,

    pub lba0: u8,
    pub lba1: u8,
    pub lba2: u8,
    pub device: u8,

    pub lba3: u8,
    pub lba4: u8,
    pub lba5: u8,
    pub featureh: u8,

    pub countl: u8,
    pub counth: u8,
    pub icc: u8,
    pub control: u8,

    pub reserved: [u8; 4],
}

#[derive(Debug)]
#[repr(C)]
pub struct D2HRegisterFis {
    pub fis_type: u8,
    pub pmport: u8,
    pub status: u8,
    pub error: u8,

    pub lba0: u8,
    pub lba1: u8,
    pub lba2: u8,
    pub device: u8,

    pub lba3: u8,
    pub lba4: u8,
    pub lba5: u8,
    reserved: u8,

    pub countl: u8,
    pub counth: u8,
    reserved2: [u8; 2],

    reserved3: [u8; 4],
}
