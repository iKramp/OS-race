use crate::println;
use core::arch::asm;

enum Rsdp {
    V1(&'static Rsdp_v1),
    V2(&'static Rsdp_v2),
}

impl Rsdp {
    fn validate(&self) -> bool {
        let self_ptr = self as *const _ as *const u8;
        match self {
            Self::V1(_) => {
                let mut sum: u16 = 0; //to avoid overflow panic. using u8 would be ok in release
                                      //build, but in debug rust checks for overflow at runtime
                for i in 0..20 {
                    sum += unsafe { *self_ptr.offset(i) as u16 };
                }
                sum & 0xFF == 0
            }
            Self::V2(_) => {
                let mut sum: u16 = 0;
                for i in 0..34 {
                    sum += unsafe { *self_ptr.offset(i) as u16 };
                }
                sum & 0xFF == 0
            }
        }
    }

    fn from_ptr(ptr: *const u8) -> Self {
        let revision = unsafe { *ptr.offset(15) };
        if revision == 0 {
            Self::V1(unsafe { &*(ptr as *const _) })
        } else {
            Self::V2(unsafe { &*(ptr as *const _) })
        }
    }

    fn get_address(&self) -> u64 {
        match self {
            Self::V1(data) => data.rsdt_address as u64,
            Self::V2(data) => data.xsdt_address,
        }
    }
}

#[repr(C, packed)]
struct Rsdp_v1 {
    signature: [u8; 8],
    checksum: u8,
    oemid: [u8; 6],
    revision: u8,
    rsdt_address: u32,
}

#[repr(C, packed)]
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
pub fn enable_interrupt_controller(rsdp_addr: Option<u64>) {
    /*let Some(rsdp_addr) = rsdp_addr else {
        init_PIC(false);
        return;
    };
    let rsdp_table = Rsdp::from_ptr(rsdp_addr as *const u8);
    if !rsdp_table.validate() {
        init_PIC(false);
        return;
    } else {
        init_PIC(true);
    }*/

    init_PIC(false);
}

#[allow(non_snake_case)]
fn init_PIC(disable: bool) {
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

    let mask = if disable { 0xff } else { 0x03 };

    byte_to_port(PIC1_DATA, mask);
    byte_to_port(PIC2_DATA, mask);
}

fn byte_to_port(port: u16, byte: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") byte);
    }
}
