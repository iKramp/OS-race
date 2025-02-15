#![allow(non_snake_case)]

use core::fmt::Debug;
use std::{
    mem_utils::{PhysAddr, VirtAddr},
    println,
    vec::Vec,
    PageAllocator,
};

use bitfield::bitfield;

use crate::{
    disk::Disk,
    memory::{paging::LiminePat, physical_allocator::BUDDY_ALLOCATOR, PAGE_TREE_ALLOCATOR},
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
    pub ports: Vec<VirtualPort>,
    is_64_bit: bool,
}

impl Disk for AhciDisk {
    //https://forum.osdev.org/viewtopic.php?t=40969
    fn init(&mut self) {
        let mut ghc = self.abar.read_from_bar::<GenericHostControl>(0);

        //enable AHCI
        ghc.ghc.SetAE(true);
        self.abar.write_to_bar(&ghc.ghc, 4);

        //bios handoff??
        if ghc.cap2.BOH() {
            self.perform_bios_handoff();
        } else {
            println!("No bios handoff");
        }

        self.wait_for_idle_ports();

        //reset HBA
        ghc.ghc.SetHR(true);
        self.abar.write_to_bar(&ghc.ghc, 4);
        while ghc.ghc.HR() {
            unsafe { core::arch::asm!("hlt") };
            ghc.ghc = self.abar.read_from_bar(4);
        }

        //enable AHCI again after reset
        ghc.ghc.SetAE(true);
        self.abar.write_to_bar(&ghc.ghc, 4);

        let staggered_spin_up = ghc.cap.SSS();

        let mut active_ports = Vec::new();
        //loop and init ports
        for port in &mut self.ports {
            if Self::init_port(port, self.is_64_bit, &self.abar, staggered_spin_up) {
                active_ports.push(port.index);
            }
        }

        self.ports.retain(|port| active_ports.contains(&port.index));
    }
}

impl AhciDisk {
    ///Disk::init() must be called after this
    pub fn new(device: device_config::RegularPciDevice) -> Self {
        let abar = device.bars.iter().find(|bar| bar.get_index() == 5).unwrap().clone();

        let ghc = abar.read_from_bar::<GenericHostControl>(0);
        let is_64_bit = ghc.cap.S64A();

        let mut ports = Vec::new();
        let ports_implemented = ghc.pi;

        for i in 0..32 {
            if ports_implemented & (1 << i) != 0 {
                ports.push(VirtualPort {
                    index: i as u8,
                    command_list: VirtAddr(0),
                    fis: VirtAddr(0),
                });
            }
        }

        Self {
            device,
            abar,
            ports,
            is_64_bit,
        }
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
                    if !bohc.BB() || start.elapsed().as_secs() > 2 {
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

    fn wait_for_idle_ports(&self) {
        for port in &self.ports {
            //let mut port_command = self.get_port(port_index).PxCMD;
            let mut port_command = PortCommand(port.get_port_property(0x18, &self.abar));
            if port_command.ST() {
                port_command.SetST(false);
                port.set_port_property(0x18, port_command.0, &self.abar);
                unsafe { core::arch::asm!("hlt") }; //i need to find a better system to sleep, 1ms
                                                    //is too long
            }
            while port_command.CR() {
                unsafe { core::arch::asm!("hlt") };
                port_command = PortCommand(port.get_port_property(0x18, &self.abar));
            }
            if port_command.FR() {
                port_command.SetFRE(false);
                port.set_port_property(0x18, port_command.0, &self.abar);
                while port_command.FR() {
                    unsafe { core::arch::asm!("hlt") };
                    port_command = PortCommand(port.get_port_property(0x18, &self.abar));
                }
            }
            let mut sctl = SATAControl(port.get_port_property(0x2C, &self.abar));
            if sctl.DET() != 0 {
                sctl.SetDet(0);
                port.set_port_property(0x2C, sctl.0, &self.abar);
            }
        }
    }

    fn get_port(&self, port: &VirtualPort) -> Port {
        let port_offset = 0x100 + (port.index as u64) * 0x80;
        let port = self.abar.read_from_bar::<Port>(port_offset);
        port
    }

    fn init_port(port: &mut VirtualPort, is_64_bit: bool, abar: &Bar, staggered_spin_up: bool) -> bool {
        port.init_cmd_list_fis(is_64_bit, abar);
        let mut port_cmd = PortCommand(port.get_port_property(0x18, abar));
        port_cmd.SetFRE(true);
        port.set_port_property(0x18, port_cmd.0, abar);

        while !port_cmd.FR() {
            port_cmd = PortCommand(port.get_port_property(0x18, abar));
        }

        if staggered_spin_up {
            println!("Staggered spin up");
            port_cmd.SetSUD(true);
            port.set_port_property(0x18, port_cmd.0, abar);
        }

        //wait for port to be ready
        let mut sata_status = SATAStatus(port.get_port_property(0x28, abar));
        let start = std::time::Instant::now();
        while sata_status.DET() != 3 {
            if start.elapsed().as_millis() > 10 {
                println!("Port {} not working", port.index);
                return false;
            }
            unsafe { core::arch::asm!("hlt") };
            sata_status = SATAStatus(port.get_port_property(0x28, abar));
        }

        //clear error register
        port.set_port_property(0x30, 0xFFFFFFFF, abar);

        //wait for device to be ready
        let mut task_file_data = TaskFileData(port.get_port_property(0x20, abar));
        while task_file_data.STS_BSY() || task_file_data.STS_DRQ() || task_file_data.STS_ERR() {
            unsafe { core::arch::asm!("hlt") };
            task_file_data = TaskFileData(port.get_port_property(0x20, abar));
        }

        println!("Port {} initialized", port.index);
        return true;
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct GenericHostControl {
    cap: Capabilities,
    ghc: GlobalHBAControl,
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
    struct GlobalHBAControl(u32);
    impl Debug;
    AE, SetAE: 31;
    MRSM, _: 2;
    IE, SetIE: 1;
    /// SetOOC write 1 to set
    HR, SetHR: 0;
}

bitfield! {
    struct Capabilities(u32);
    impl Debug;
    S64A, _: 31;
    SSS, _: 27;
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
struct VirtualPort {
    index: u8,
    command_list: VirtAddr,
    fis: VirtAddr,
}

impl VirtualPort {
    pub fn init_cmd_list_fis(&mut self, is_64_bit: bool, abar: &Bar) {
        const FIS_SWITCHING: bool = false;

        let cmd_list_base = if is_64_bit {
            unsafe { BUDDY_ALLOCATOR.allocate_frame() }
        } else {
            unsafe { BUDDY_ALLOCATOR.allocate_frame_low() }
        };

        let fis_base = if !FIS_SWITCHING {
            cmd_list_base + PhysAddr(0x400)
        } else if is_64_bit {
            unsafe { BUDDY_ALLOCATOR.allocate_frame() }
        } else {
            unsafe { BUDDY_ALLOCATOR.allocate_frame_low() }
        };

        self.set_port_property(0, cmd_list_base.0 as u32, abar);
        self.set_port_property(4, (cmd_list_base.0 >> 32) as u32, abar);
        self.set_port_property(8, fis_base.0 as u32, abar);
        self.set_port_property(12, (fis_base.0 >> 32) as u32, abar);

        let clb_virt = unsafe { PAGE_TREE_ALLOCATOR.allocate(Some(cmd_list_base)) };
        let fis_virt = if !FIS_SWITCHING {
            clb_virt + VirtAddr(0x400)
        } else {
            unsafe { PAGE_TREE_ALLOCATOR.allocate(Some(fis_base)) }
        };

        unsafe {
            PAGE_TREE_ALLOCATOR.get_page_table_entry_mut(clb_virt).set_pat(LiminePat::UC);
            if FIS_SWITCHING {
                PAGE_TREE_ALLOCATOR.get_page_table_entry_mut(fis_virt).set_pat(LiminePat::UC);
            }
        }

        self.command_list = clb_virt;
        self.fis = fis_virt;
    }

    fn set_port_property(&self, offset: u64, value: u32, abar: &Bar) {
        let port_offset = 0x100 + (self.index as u64) * 0x80 + offset;
        abar.write_to_bar(&value, port_offset);
    }

    fn get_port_property(&self, offset: u64, abar: &Bar) -> u32 {
        let port_offset = 0x100 + (self.index as u64) * 0x80 + offset;
        abar.read_from_bar(port_offset)
    }
}

#[derive(Debug)]
#[repr(C)]
struct Port {
    PxCLB: u64,
    PxFB: u64,
    PxIS: u32,
    PxIE: u32,
    ///WARNING! contains RW1 field
    PxCMD: PortCommand,
    reserved: u32,
    PxTFD: TaskFileData,
    PxSIG: u32,
    PxSSTS: SATAStatus,
    PxSCTL: SATAControl,
    PxSERR: u32,
    PxSACT: u32,
    PxCI: u32,
    PxSNTF: u32,
    PxFBS: u32,
    PxDEVSLP: u32,
    reserved2: [u32; 10],
    PxVS: u32,
}

impl Port {}

bitfield! {
    struct PortCommand(u32);
    impl Debug;
    CR, _: 15;
    FR, _: 14;
    FRE, SetFRE: 4;
    /// RW1
    CLO, SetClo: 3;
    ///Before setting, set CLO and wait for it to clear
    SUD, SetSUD: 1;
    ST, SetST: 0;
}

bitfield! {
    struct TaskFileData(u32);
    impl Debug;
    ERR, _: 15, 8;
    STS_BSY, _: 7;
    STS_DRQ, _: 3;
    STS_ERR, _: 0;
}

bitfield! {
    struct SATAStatus(u32);
    impl Debug;
    IPM, _: 11, 8;
    SPD, _: 7, 4;
    DET, _: 3, 0;
}

bitfield! {
    struct SATAControl(u32);
    impl Debug;
    DET, SetDet: 3, 0;
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
