#![allow(non_snake_case)]

use core::fmt::Debug;
use std::{mem_utils::VirtAddr, println, vec::Vec, PageAllocator, PAGE_ALLOCATOR};

use bitfield::bitfield;

use crate::{
    disk::Disk,
    memory::{physical_allocator::BUDDY_ALLOCATOR, PAGE_TREE_ALLOCATOR},
    pci::device_config::{self, Bar},
};

enum FisType {
    RegisterH2D = 0x27,
    RegisterD2H = 0x34,
    DMAActivate = 0x39,
    DMASetup = 0x41,
    Data = 0x46,
    Bist = 0x58,
    PIOSetup = 0x5F,
    SetDeviceBits = 0xA1,
}

#[repr(C, packed)]
struct DataFis {
    fis_type: FisType,
    port_multiplier: u8,
    reserved: [u8; 2],
    data: [u8],
}

#[derive(Debug, Clone)]
pub struct AhciDriver {}

#[derive(Debug)]
pub struct AhciDisk {
    pub device: device_config::RegularPciDevice,
    pub abar: Bar,
    pub ports: Vec<u8>,
}

impl Disk for AhciDisk {
    fn init(&mut self) {
        let mut ghc = self.abar.read_from_bar::<GenericHostControl>(0);
        let is_64_bit = (ghc.cap & (1 << 31)) != 0;

        //enable AHCI
        ghc.ghc.SetAE(true);
        ghc.ghc.SetHR(false); //just in case, this is enabled with writing 1
        self.abar.write_to_bar(&ghc.ghc, 4);

        //bios handoff??
        if ghc.cap2.BOH() {
            self.perform_bios_handoff();
        } else {
            println!("No bios handoff");
        }

        //https://forum.osdev.org/viewtopic.php?t=40969

        //enable AHCI again, just in case
        ghc.ghc.SetAE(true);
        ghc.ghc.SetHR(false);
        self.abar.write_to_bar(&ghc.ghc, 4);
        

        //loop and init ports
        
        for port_index in &self.ports {
            let mut port = self.abar.read_from_bar::<Port>(0x100 + (*port_index as u64) * 0x80);
            port.init(is_64_bit);
            self.abar.write_to_bar(&port, 0x100 + (*port_index as u64) * 0x80);
        }
    }
}

impl AhciDisk {
    ///Disk::init() must be called after this
    pub fn new(device: device_config::RegularPciDevice) -> Self {
        let abar = device.bars.iter().find(|bar| bar.get_index() == 5).unwrap().clone();

        let ghc = abar.read_from_bar::<GenericHostControl>(0);

        let mut ports = Vec::new();
        let ports_implemented = ghc.pi;

        for i in 0..32 {
            if ports_implemented & (1 << i) != 0 {
                ports.push(i as u8);
            }
        }

        Self { device, abar, ports }
    }

    fn perform_bios_handoff(&self) {
        let mut bohc = Bohc(0);
        bohc.SetOOS(true);
        self.abar.write_to_bar(&bohc, 0x28);
        println!("bohc: {:#x?}", bohc);
        let start = std::time::Instant::now();
        loop {
            let bohc = self.abar.read_from_bar::<Bohc>(0x28);
            if bohc.BB() {
                loop {
                    let bohc = self.abar.read_from_bar::<Bohc>(0x28);
                    if !bohc.BB()  || start.elapsed().as_secs() > 2 {
                        break;
                    }
                    unsafe { core::arch::asm!("hlt") };
                }
                println!("Bios handoff complete");
                break;
            }
            if start.elapsed().as_millis() > 25 {
                println!("Bios handoff timeout");
                break;
            }
            unsafe { core::arch::asm!("hlt") };
        }
    }

    fn get_port(&self, port: u8) -> Port {
        let port_offset = 0x100 + (port as u64) * 0x80;
        let port = self.abar.read_from_bar::<Port>(port_offset);
        port
    }
}

#[derive(Debug)]
#[repr(C)]
struct GenericHostControl {
    cap: u32,
    ghc: GlobalHostControl,
    is: u32,
    pi: u32,
    vs: u32,
    ccc_ctl: u32,
    ccc_ports: u32,
    em_loc: u32,
    em_ctl: u32,
    cap2: Capabilities2,
    ///WARNING! containes RWC field
    bohc: Bohc,
}

bitfield! {
    struct GlobalHostControl(u32);
    impl Debug;
    AE, SetAE: 31;
    MRSM, _: 2;
    IE, SetIE: 1;
    /// SetOOC write 1 to set
    HR, SetHR: 0;
}

bitfield! {
    struct Capabilities2(u32);
    impl Debug;
    DESO, _: 5;
    SADM, _: 4;
    SDS, _: 3;
    APST, _: 2;
    NVMP, _: 1;
    BOH, _: 0;
}

bitfield! {
    struct Bohc(u32);
    impl Debug;
    BB, SetBB: 4;
    /// SetOOC write 1 to clear
    OOC, SetOOC: 3;
    SOOE, SetSOOE: 2;
    OOS, SetOOS: 1;
    BOS, SetBOS: 0;
}

#[derive(Debug)]
#[repr(C, packed)]
struct Port {
    PxCLB: u64,
    PxFB: u64,
    PxIS: u32,
    PxIE: u32,
    PxCMD: u32,
    reserved: u32,
    PxTFD: u32,
    PxSIG: u32,
    PxSSTS: u32,
    PxSCTL: u32,
    PxSERR: u32,
    PxSACT: u32,
    PxCI: u32,
    PxSNTF: u32,
    PxFBS: u32,
    PxDEVSLP: u32,
    reserved2: [u32; 10],
    PxVS: u32,
}

impl Port {
    pub fn init(&mut self, is_64_bit: bool) -> VirtAddr {
        let allocated_frame_addr = if is_64_bit {
            unsafe { BUDDY_ALLOCATOR.allocate_frame() }
        } else {
            unsafe { BUDDY_ALLOCATOR.allocate_frame_low() }
        };

        self.PxCLB = allocated_frame_addr.0;
        self.PxFB = allocated_frame_addr.0 + 0x400;
        let clb_fis_virt = std::mem_utils::translate_phys_virt_addr(allocated_frame_addr);

        clb_fis_virt
    }

    fn init_command_list(&self, addr: VirtAddr) {
        //let command_list = addr.0 as *mut CommandHeader;
        //for i in 0..32 {
        //    unsafe {
        //        let header = command_list.add(i);
        //        (*header).prdtl = 8;
        //        (*header).ctba = addr.0 + 0x80 + i * 0x1000;
        //    }
        //}
    }
}

#[derive(Debug)]
#[repr(C, packed)]
struct CommandHeader {
    dw0: u32,
    dw1: u32,
    dw2: u32,
    dw3: u32,
}

impl CommandHeader {
    pub fn new() -> Self {
        Self {
            dw0: 0,
            dw1: 0,
            dw2: 0,
            dw3: 0,
        }
    }
}
