use std::mem_utils::PhysAddr;

#[derive(Debug)]
pub enum Rsdp {
    V1(&'static RsdpV1),
    V2(&'static RsdpV2),
}

impl Rsdp {
    fn validate(&self) -> bool {
        match self {
            Self::V1(data) => {
                let mut sum = 0_u16;
                for i in 0..8 {
                    sum += data.signature[i] as u16;
                }
                for i in 0..6 {
                    sum += data.oemid[i] as u16;
                }
                sum += data.checksum as u16;
                sum += data.revision as u16;
                for i in 0..4 {
                    sum += ((data.rsdt_address >> (i * 8)) & 0xFF) as u16
                }

                (sum & 0xFF) == 0
            }
            Self::V2(data) => {
                let mut sum = 0_u16;
                for i in 0..8 {
                    sum += data.signature[i] as u16;
                }
                for i in 0..6 {
                    sum += data.oemid[i] as u16;
                }
                sum += data.checksum as u16;
                sum += data.revision as u16;

                for i in 0..4 {
                    sum += ((data.rsdt_address >> (i * 8)) & 0xFF) as u16
                }

                for i in 0..4 {
                    sum += ((data.length >> (i * 8)) & 0xFF) as u16
                }

                for i in 0..8 {
                    sum += ((data.xsdt_address >> (i * 8)) & 0xFF) as u16
                }

                sum += data.extended_checksum as u16;
                sum += data.reserved[0] as u16;
                sum += data.reserved[1] as u16;
                sum += data.reserved[2] as u16;

                (sum & 0xFF) == 0
            }
        }
    }

    pub fn from_ptr(address: PhysAddr) -> Self {
        let revision = unsafe { *std::mem_utils::get_at_physical_addr::<u8>(address + PhysAddr(15)) };
        if revision == 0 {
            Self::V1(unsafe { std::mem_utils::get_at_physical_addr::<RsdpV1>(address) })
        } else {
            Self::V2(unsafe { std::mem_utils::get_at_physical_addr::<RsdpV2>(address) })
        }
    }

    pub fn address(&self) -> PhysAddr {
        match self {
            Self::V1(data) => PhysAddr(data.rsdt_address as u64),
            Self::V2(data) => PhysAddr(data.xsdt_address),
        }
    }

    pub fn signature(&self) -> [char; 8] {
        match self {
            Self::V1(data) => data.signature.map(|a| a as char),
            Self::V2(data) => data.signature.map(|a| a as char),
        }
    }

    pub fn oem_id(&self) -> [char; 6] {
        match self {
            Self::V1(data) => data.oemid.map(|a| a as char),
            Self::V2(data) => data.oemid.map(|a| a as char),
        }
    }
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct RsdpV1 {
    signature: [u8; 8],
    checksum: u8,
    oemid: [u8; 6],
    revision: u8,
    rsdt_address: u32,
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct RsdpV2 {
    signature: [u8; 8],
    checksum: u8,
    oemid: [u8; 6],
    revision: u8,
    rsdt_address: u32,

    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

//first do memory allocation and mapping, then i can map rsdp memory and do this
pub fn get_rsdp_table(rsdp_addr: Option<u64>) -> Option<Rsdp> {
    let rsdp_table = Rsdp::from_ptr(PhysAddr(rsdp_addr?));
    if !rsdp_table.validate() {
        return None;
    }
    Some(rsdp_table)
}
