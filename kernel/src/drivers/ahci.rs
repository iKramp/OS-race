use core::fmt::Debug;
use std::{println, vec::Vec};

use crate::{
    disk::Disk,
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
    }
}

impl AhciDisk {
    pub fn new(device: device_config::RegularPciDevice) -> Self {
        let abar = device.bars.iter().find(|bar| bar.get_index() == 5).unwrap().clone();

        
        let ghc = abar.read_from_bar::<GlobalHostControl>(0);
        println!("GHC: {:#x?}", ghc);
        let mut ports = Vec::new();
        let ports_implemented = ghc.pi;

        for i in 0..32 {
            if ports_implemented & (1 << i) != 0 {
                ports.push(i as u8);
                let port = abar.read_from_bar::<Port>(0x100 + (i as u64) * 0x80);
                println!("Port {}: {:#x?}", i, port);
            }
        }

        let mut disk = Self { device, abar, ports };
    
        disk.init();

        disk
    }

    fn get_port(&self, port: u8) -> Port {
        let port_offset = 0x100 + (port as u64) * 0x80;
        let port = self.abar.read_from_bar::<Port>(port_offset);
        port
    }
}

#[derive(Debug)]
#[repr(C, packed)]
struct GlobalHostControl {
    cap: u32,
    ghc: u32,
    is: u32,
    pi: u32,
    vs: u32,
    ccc_ctl: u32,
    ccc_ports: u32,
    em_loc: u32,
    em_ctl: u32,
    cap2: u32,
    bohc: u32,
}

#[derive(Debug)]
#[repr(C, packed)]
struct Port {
    command_list_base: u64,
    fis_base: u64,
    interrupt_status: u32,
    interrupt_enable: u32,
    command: u32,
    reserved: u32,
    task_file_data: u32,
    signature: u32,
    sata_status: u32,
    sata_control: u32,
    sata_error: u32,
    sata_active: u32,
    command_issue: u32,
    sata_notification: u32,
    fis_switch_control: u32,
}
