#![allow(non_snake_case)]

use core::{arch::x86_64::_mm_setzero_ps, fmt::Debug, time::Duration, u32};
use std::{
    mem_utils::{get_at_physical_addr, get_at_virtual_addr, memset_virtual_addr, set_at_virtual_addr, PhysAddr, VirtAddr},
    println,
    vec::Vec,
    PageAllocator,
};

use bitfield::bitfield;

use crate::{
    disk::Disk, drivers::ahci::fis::D2HRegisterFis, memory::{paging::LiminePat, physical_allocator::BUDDY_ALLOCATOR, PAGE_TREE_ALLOCATOR}, pci::device_config::{self, Bar}
};

use super::fis::{FisType, H2DRegisterFis};

#[derive(Debug, Clone)]
pub struct AhciDriver {}

#[derive(Debug)]
pub struct AhciDisk {
    pub device: device_config::RegularPciDevice,
    pub abar: &'static mut GenericHostControl,
    pub ports: Vec<VirtualPort>,
    is_64_bit: bool,
}

impl Disk for AhciDisk {
    //https://forum.osdev.org/viewtopic.php?t=40969
    fn init(&mut self) {
        self.device.enable_bus_mastering();
        let ghc = unsafe { (&raw const self.abar).read_volatile() };

        //enable AHCI
        ghc.ghc.SetAE(true);
        unsafe { (&raw mut self.abar.ghc).write_volatile(GlobalHBAControl(ghc.ghc.0)) };

        //bios handoff??
        if ghc.cap2.BOH() {
            self.perform_bios_handoff();
        } else {
            println!("No bios handoff");
        }

        self.wait_for_idle_ports();

        //reset HBA
        ghc.ghc.SetHR(true);
        unsafe { (&raw mut self.abar.ghc).write_volatile(GlobalHBAControl(ghc.ghc.0)) };
        while ghc.ghc.HR() {
            unsafe { core::arch::asm!("hlt") };
            ghc.ghc = unsafe { (&raw const self.abar.ghc).read_volatile() };
        }

        self.wait_for_idle_ports();

        //enable AHCI again after reset
        ghc.ghc.SetAE(true);
        unsafe { (&raw mut self.abar.ghc).write_volatile(GlobalHBAControl(ghc.ghc.0)) };

        let staggered_spin_up = ghc.cap.SSS();

        let mut active_ports = Vec::new();
        //loop and init ports
        for port in &mut self.ports {
            if port.init(self.is_64_bit, staggered_spin_up) {
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
        let Bar::Memory(_, addr, _) = abar else {
            panic!("Abar is not memory mapped");
        };

        let ghc = abar.read_from_bar::<GenericHostControl>(0);
        let is_64_bit = ghc.cap.S64A();

        let mut ports = Vec::new();
        let ports_implemented = ghc.pi;

        for i in 0..32 {
            if ports_implemented & (1 << i) != 0 {
                ports.push(VirtualPort {
                    index: i as u8,
                    address: (addr.0 + 0x100 + (i as u64) * 0x80) as *mut u32,
                    command_list: VirtAddr(0),
                    fis: VirtAddr(0),
                    is_64_bit,
                });
            }
        }

        Self {
            device,
            abar: unsafe { &mut *(addr.0 as *mut GenericHostControl) },
            ports,
            is_64_bit,
        }
    }

    fn perform_bios_handoff(&mut self) {
        let mut bohc = Bohc(0);
        bohc.SetOOS(true);
        println!("bohc: {:#x?}", bohc);
        unsafe { (&raw mut self.abar.bohc).write_volatile(bohc); }
        let start = std::time::Instant::now();
        loop {
            let bohc = unsafe { (&raw mut self.abar.bohc).read_volatile() };
            if bohc.BB() {
                loop {
                    let bohc = unsafe { (&raw mut self.abar.bohc).read_volatile() };
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
            let mut port_command = PortCommand(port.get_property(0x18));
            if port_command.ST() {
                port_command.SetST(false);
                port.set_property(0x18, port_command.0);
                unsafe { core::arch::asm!("hlt") }; //i need to find a better system to sleep, 1ms
                                                    //is too long
            }
            while port_command.CR() {
                unsafe { core::arch::asm!("hlt") };
                port_command = PortCommand(port.get_property(0x18));
            }
            if port_command.FR() {
                port_command.SetFRE(false);
                port.set_property(0x18, port_command.0);
                while port_command.FR() {
                    unsafe { core::arch::asm!("hlt") };
                    port_command = PortCommand(port.get_property(0x18));
                }
            }
            let mut sctl = SATAControl(port.get_property(0x2C));
            if sctl.DET() != 0 {
                sctl.SetDet(0);
                port.set_property(0x2C, sctl.0);
            }
        }
    }
}

impl VirtualPort {
    pub fn init_cmd_list_fis(&mut self, is_64_bit: bool) {
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

        self.set_property(0, cmd_list_base.0 as u32);
        self.set_property(4, (cmd_list_base.0 >> 32) as u32);
        self.set_property(8, fis_base.0 as u32);
        self.set_property(12, (fis_base.0 >> 32) as u32);

        let clb_virt = unsafe { PAGE_TREE_ALLOCATOR.allocate(Some(cmd_list_base)) };
        unsafe { memset_virtual_addr(clb_virt, 0, 0x1000) };
        let fis_virt = if !FIS_SWITCHING {
            clb_virt + VirtAddr(0x400)
        } else {
            let temp = unsafe { PAGE_TREE_ALLOCATOR.allocate(Some(fis_base)) };
            unsafe { memset_virtual_addr(temp, 0, 0x1000) };
            temp
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

    fn set_property(&self, offset: u64, value: u32) {
        unsafe { self.address.byte_add(offset as usize).write_volatile(value) };
    }

    fn get_property(&self, offset: u64) -> u32 {
        unsafe { self.address.byte_add(offset as usize).read_volatile() }
    }

    fn get_port(&self) -> Port {
        unsafe { (self.address as *const Port).read_volatile() }
    }

    fn init(&mut self, is_64_bit: bool, staggered_spin_up: bool) -> bool {
        self.init_cmd_list_fis(is_64_bit);
        let mut port_cmd = PortCommand(self.get_property(0x18));
        port_cmd.SetFRE(true);
        self.set_property(0x18, port_cmd.0);
        
        while !port_cmd.FR() {
            unsafe { core::arch::asm!("hlt") };
            port_cmd = PortCommand(self.get_property(0x18));
        }

        port_cmd.SetST(true);
        self.set_property(0x18, port_cmd.0);

        if staggered_spin_up {
            println!("Staggered spin up");
            port_cmd.SetSUD(true);
            self.set_property(0x18, port_cmd.0);
        }

        //wait for port to be ready
        let mut sata_status = SATAStatus(self.get_property(0x28));
        let start = std::time::Instant::now();
        while sata_status.DET() != 3 {
            if start.elapsed().as_millis() > 10 {
                println!("Port {} not working", self.index);
                return false;
            }
            unsafe { core::arch::asm!("hlt") };
            sata_status = SATAStatus(self.get_property(0x28));
        }
        //clear error register
        self.set_property(0x30, 0xFFFFFFFF);

        //wait for device to be ready
        let mut task_file_data = TaskFileData(self.get_property(0x20));
        while task_file_data.STS_BSY() || task_file_data.STS_DRQ() || task_file_data.STS_ERR() {
            unsafe { core::arch::asm!("hlt") };
            task_file_data = TaskFileData(self.get_property(0x20));
        }

        //clear interrupt status
        self.set_property(0x10, 0xFFFFFFFF);
        //enable port interrupts here
        self.set_property(0x14, 0xFF);

        let mut signature = self.get_property(0x24);
        while signature == !0 {
            unsafe { core::arch::asm!("hlt") };
            signature = self.get_property(0x24);
        }
        if signature & 0xFFF != 0x101 {
            println!("Port {} not a SATA device", self.index);
            return false;
        }

        self.send_identify();

        unsafe {
            let register_fis = get_at_virtual_addr::<D2HRegisterFis>(self.fis);
            println!("Register fis: {:#x?}", register_fis);
        }

        println!("Port {} initialized", self.index);
        true
    }

    fn send_identify(&self) {
        let ident_fis = H2DRegisterFis {
            fis_type: FisType::RegisterH2D as u8,
            command: 0xEC,//identify
            pmport: 1,
            device: 0xA0,
            control: 0x08,
            ..Default::default()
        };

        let fis_recv_area = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
        let prdt = Prdt {
            base: fis_recv_area,
            count: 512,
        };



        let ident_fis = unsafe { core::mem::transmute::<H2DRegisterFis, [u8; 20]>(ident_fis) };
        let identify_cmd_index = self.build_command(&ident_fis, &[prdt]).unwrap();

        //let mut ci = self.get_property(0x38);
        //while ci & (1 << identify_cmd_index) != 0 {
        //    unsafe { core::arch::asm!("hlt") };
        //    ci = self.get_property(0x38);
        //}

        self.clean_command(identify_cmd_index);


        let data = unsafe { get_at_physical_addr::<[u8; 512]>(fis_recv_area) };
        //println!("Identify data: {:x?}", data);
    }

    ///PRDT cannot be more than a bit over 900MB. Just use multiple commands
    fn build_command(&self, cfis: &[u8], prdt: &[Prdt]) -> Option<u8> {
        assert!(prdt.len() <= 248); //i don't want to deal with contiguous allocation
        let cmd_issue = self.get_property(0x38);
        if cmd_issue == !0 {
            return None;
        }
        let index = cmd_issue.trailing_ones() as u8;
        
        let cmd_table_page = if self.is_64_bit {
            unsafe { BUDDY_ALLOCATOR.allocate_frame() }
        } else {
            unsafe { BUDDY_ALLOCATOR.allocate_frame_low() }
        };

        


        let cmd_header_0 = ((prdt.len() as u32) << 14) | 
            1 << 10 | //clear busy on complete
            ((cfis.len() >> 2) as u32 & 0b11111); //length in dwords
        let cmd_header_1 = 0; //length of transferred bytes, updated by hardware
        let cmd_header_2 = cmd_table_page.0 as u32;
        let cmd_header_3 = (cmd_table_page.0 >> 32) as u32;

        unsafe {
            let cmd_header = (self.command_list.0 as *mut u32).add(index as usize * 4);
            cmd_header.write_volatile(cmd_header_0);
            cmd_header.add(1).write_volatile(cmd_header_1);
            cmd_header.add(2).write_volatile(cmd_header_2);
            cmd_header.add(3).write_volatile(cmd_header_3);

            let cmd_table_virt = PAGE_TREE_ALLOCATOR.allocate(Some(cmd_table_page));
            PAGE_TREE_ALLOCATOR.get_page_table_entry_mut(cmd_table_virt).set_pat(LiminePat::UC);
            let cmd_table_raw = cmd_table_virt.0 as *mut u8;
            for (i, byte) in cfis.iter().enumerate() {
                cmd_table_raw.add(i).write_volatile(*byte);
            }

            for (i, prdt) in prdt.iter().enumerate() {
                let prdt_entry = cmd_table_raw.add(0x80 + i * 16) as *mut u32;
                prdt_entry.write_volatile(prdt.base.0 as u32);
                prdt_entry.add(1).write_volatile((prdt.base.0 >> 32) as u32);
                
                //convert count to 0 based even number
                let count = (prdt.count - 1) | 1;
                prdt_entry.add(3).write_volatile(count & 0x3FFFFF);
            }
            


            PAGE_TREE_ALLOCATOR.unmap(cmd_table_virt);

        }

        let cmd_issue = 1 << index;
        self.set_property(0x38, cmd_issue);

        Some(index)
    }

    fn clean_command(&self, index: u8) {
        unsafe {
            let cmd_header = (self.command_list.0 as *mut u32).add(index as usize * 4);
            let table_lower = cmd_header.add(2).read_volatile();
            let table_upper = cmd_header.add(3).read_volatile();
            let table = (table_upper as u64) << 32 | table_lower as u64;
            BUDDY_ALLOCATOR.mark_addr(PhysAddr(table * 0x1000), false);
        }
        //potentially anything else
        
    }
}

struct Prdt {
    base: PhysAddr,
    count: u32,
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
pub struct VirtualPort {
    index: u8,
    address: *mut u32,
    command_list: VirtAddr,
    fis: VirtAddr,
    is_64_bit: bool,
}

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
