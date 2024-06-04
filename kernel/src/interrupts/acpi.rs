use crate::println;
use core::arch::asm;
use std::mem_utils::PhysAddr;

#[derive(Debug)]
enum Rsdp {
    V1(&'static Rsdp_v1),
    V2(&'static Rsdp_v2),
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

    fn from_ptr(address: PhysAddr) -> Self {
        let revision = unsafe { *std::mem_utils::get_at_physical_addr::<u8>(address + PhysAddr(15)) };
        if revision == 0 {
            Self::V1(unsafe { std::mem_utils::get_at_physical_addr::<Rsdp_v1>(address) })
        } else {
            Self::V2(unsafe { std::mem_utils::get_at_physical_addr::<Rsdp_v2>(address) })
        }
    }

    fn address(&self) -> u64 {
        match self {
            Self::V1(data) => data.rsdt_address as u64,
            Self::V2(data) => data.xsdt_address,
        }
    }

    fn signature(&self) -> [char; 8] {
        match self {
            Self::V1(data) => data.signature.map(|a| a as char),
            Self::V2(data) => data.signature.map(|a| a as char),
        }
    }

    fn oem_id(&self) -> [char; 6] {
        match self {
            Self::V1(data) => data.oemid.map(|a| a as char),
            Self::V2(data) => data.oemid.map(|a| a as char),
        }
    }
}

#[repr(C, packed)]
#[derive(Debug)]
struct Rsdp_v1 {
    signature: [u8; 8],
    checksum: u8,
    oemid: [u8; 6],
    revision: u8,
    rsdt_address: u32,
}

#[repr(C, packed)]
#[derive(Debug)]
struct Rsdp_v2 {
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
pub fn enable_apic(rsdp_addr: Option<u64>) {
    let Some(rsdp_addr) = rsdp_addr else {
        return;
    };
    let rsdp_table = Rsdp::from_ptr(PhysAddr(rsdp_addr));
    println!("{:#?}", rsdp_table);
    if !rsdp_table.validate() {
        return;
    }
    todo!("mask interrupts with 0xff and init apic");
}

#[allow(non_snake_case)]
pub fn init_PIC() {
    const PIC1: u16 = 0x20;
    const PIC2: u16 = 0xA0; /* IO base address for slave PIC */
    const PIC1_COMMAND: u16 = PIC1;
    const PIC1_DATA: u16 = PIC1 + 1;
    const PIC2_COMMAND: u16 = PIC2;
    const PIC2_DATA: u16 = PIC2 + 1;

    byte_to_port(PIC1_COMMAND, 0x11);
    byte_to_port(PIC2_COMMAND, 0x11);

    byte_to_port(PIC1_DATA, 0x20);
    byte_to_port(PIC2_DATA, 0x28);

    byte_to_port(PIC1_DATA, 0x04);
    byte_to_port(PIC2_DATA, 0x02);

    byte_to_port(PIC1_DATA, 0x01);
    byte_to_port(PIC2_DATA, 0x01);

    byte_to_port(PIC1_DATA, 0x03); //change to 0x00 to handle keyboard and timer
    byte_to_port(PIC2_DATA, 0x03);
}

fn byte_to_port(port: u16, byte: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") byte);
    }
}
