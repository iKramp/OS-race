#![allow(non_snake_case)]
#![allow(clippy::identity_op)]

use core::fmt::Debug;
use std::{
    mem_utils::{get_at_physical_addr, get_at_virtual_addr, memset_virtual_addr, PhysAddr, VirtAddr},
    println,
    vec::Vec,
    PageAllocator, PAGE_ALLOCATOR,
};

use bitfield::bitfield;

use crate::{
    disk::Disk,
    drivers::{ahci::fis::{D2HRegisterFis, IdentifyStructure, PioSetupFis}, gpt::GPTDriver, DiskDriver, PartitionSchemeDriver},
    memory::{paging::LiminePat, physical_allocator::BUDDY_ALLOCATOR, PAGE_TREE_ALLOCATOR},
    pci::device_config::{self, Bar},
};

use super::fis::{FisType, H2DRegFisPmport, H2DRegisterFis};

//we assume 48 bit lba
const READ_DMA: u8 = 0x25;
const WRITE_DMA: u8 = 0x35;

#[derive(Debug, Clone)]
pub struct AhciDriver {}

#[derive(Debug)]
pub struct AhciController {
    pub device: device_config::RegularPciDevice,
    pub abar: &'static mut GenericHostControl,
    pub ports: Vec<VirtualPort>,
    is_64_bit: bool,
}

impl Disk for AhciController {
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
        ghc.ghc.SetIE(true);
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

        let gpt_driver = GPTDriver {};
        gpt_driver.partitions(self.ports.first_mut().unwrap());
    }
}

impl AhciController {
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
                    sectors: 0,
                    command_depth: 0,
                    device: 0,
                    command_metadata: [CommandMetadata { issued: false, }; 32],
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
        unsafe {
            (&raw mut self.abar.bohc).write_volatile(bohc);
        }
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

#[derive(Debug)]
pub struct VirtualPort {
    index: u8,
    address: *mut u32,
    command_list: VirtAddr,
    fis: VirtAddr,
    is_64_bit: bool,
    sectors: u64,
    command_depth: u16,
    device: u8,
    command_metadata: [CommandMetadata; 32],
}

#[derive(Debug, Clone, Copy)]
struct CommandMetadata {
    issued: bool,
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

    fn display_port(&self) {
        println!("{:#x?}", self.get_port());
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    fn init(&mut self, is_64_bit: bool, staggered_spin_up: bool) -> bool {
        self.init_cmd_list_fis(is_64_bit);
        let mut port_cmd = PortCommand(self.get_property(0x18));
        port_cmd.SetFRE(true);
        self.set_property(0x18, port_cmd.0);
        //here a register FIS is sent immediately

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
        self.set_property(0x14, 0xFFFFFFFF);

        self.send_identify();

        unsafe {
            let register_fis = &raw const *get_at_virtual_addr::<D2HRegisterFis>(self.fis + VirtAddr(0x40));
            let _pio_setup_fis = &raw const *get_at_virtual_addr::<PioSetupFis>(self.fis + VirtAddr(0x20));
            self.set_property(0x10, 3);
            self.device = register_fis.read_volatile().device;
            //use them?
        }

        println!("Port {} initialized", self.index);

        true
    }

    fn send_identify(&mut self) {
        let mut pmport = H2DRegFisPmport(0);
        pmport.set_command(true);
        let ident_fis = H2DRegisterFis {
            fis_type: FisType::RegisterH2D as u8,
            command: 0xEC, //identify
            pmport,
            device: 0xA0, // change depending on SATA/ATAPI
            control: 0x08,
            ..Default::default()
        };

        let fis_recv_area = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
        let prdt = PrdtDescriptor {
            base: fis_recv_area,
            count: 512,
        };

        let ident_fis = unsafe { core::mem::transmute::<H2DRegisterFis, [u8; 20]>(ident_fis) };
        let identify_cmd_index = self.build_command(false, &ident_fis, &[prdt]).unwrap();

        let mut ci = self.get_property(0x38);
        while ci & (1 << identify_cmd_index) != 0 {
            unsafe { core::arch::asm!("hlt") };
            ci = self.get_property(0x38);
        }

        std::thread::sleep(std::time::Duration::from_secs(1));

        self.clean_command(identify_cmd_index);

        unsafe {
            let data = &raw const *get_at_physical_addr::<IdentifyStructure>(fis_recv_area);
            let data = data.read_volatile();

            self.sectors = data.total_usr_sectors();
            self.command_depth = data.queue_depth;
            assert!(data.sector_bytes == 512);
        }
    }

    ///PRDT cannot be more than a bit over 900MB. Just use multiple commands
    fn build_command(&mut self, write: bool, cfis: &[u8], prdt: &[PrdtDescriptor]) -> Option<u8> {
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

        let mut cmd_header = CmdHeader(0);
        cmd_header.SetWrite(write);
        cmd_header.SetCFL(cfis.len() as u128 / 4);
        cmd_header.SetClearBusy(true);
        cmd_header.SetPRDTL(prdt.len() as u128);
        debug_assert!(cmd_table_page.0 & 0b1111111 == 0); //128 byte alignment
        cmd_header.SetCTBA(cmd_table_page.0 as u128);

        unsafe {
            let cmd_header_ptr = (self.command_list.0 as *mut CmdHeader).add(index as usize * 4);
            cmd_header_ptr.write_volatile(cmd_header);

            let cmd_table_virt = PAGE_TREE_ALLOCATOR.allocate(Some(cmd_table_page));
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(cmd_table_virt)
                .set_pat(LiminePat::UC);
            let cmd_table_raw = cmd_table_virt.0 as *mut u8;
            for (i, byte) in cfis.iter().enumerate() {
                cmd_table_raw.add(i).write_volatile(*byte);
            }

            for (i, prdt) in prdt.iter().enumerate() {
                let prdt_entry_ptr = cmd_table_raw.add(0x80 + i * 16) as *mut PrdtEntry;
                let mut prdt_entry = PrdtEntry(0);
                prdt_entry.SetInt(true);
                prdt_entry.SetDBA(prdt.base.0.into());
                prdt_entry.SetDBC(prdt.count as u128 - 1);
                prdt_entry_ptr.write_volatile(PrdtEntry(prdt_entry.0));
                println!("PRDT: {:#x?}", prdt_entry);
            }

            PAGE_TREE_ALLOCATOR.unmap(cmd_table_virt);
        }

        let cmd_issue = 1 << index;
        println!("Command issued: {:#x?}", index);

        //spin on busy
        let mut port_cmd = TaskFileData(self.get_property(0x20));
        while port_cmd.STS_BSY() {
            unsafe { core::arch::asm!("hlt") };
            port_cmd = TaskFileData(self.get_property(0x20));
        }

        self.set_property(0x38, cmd_issue);
        self.command_metadata[index as usize].issued = true;

        Some(index)
    }

    ///frees command header memory. Does not free regions pointed to by PRDT
    fn clean_command(&self, index: u8) {
        unsafe {
            let cmd_header = (self.command_list.0 as *mut u32).add(index as usize * 4);
            let table_lower = cmd_header.add(2).read_volatile();
            let table_upper = cmd_header.add(3).read_volatile();
            let table = (table_upper as u64) << 32 | table_lower as u64;
            BUDDY_ALLOCATOR.mark_addr(PhysAddr(table), false);
        }
        //potentially anything else
    }
}

impl DiskDriver for VirtualPort {
    ///Returns the virtual address of the read data and the command index used
    fn read(&mut self, start_sec_index: usize, sec_count: usize, addr: VirtAddr) -> u64 {
        assert!(sec_count <= self.sectors as usize);
        let prdt_entries = (sec_count + 7) / 8; //8 sectors in one physical frame
        let mut phys_addresses = Vec::new();
        for i in 0..prdt_entries {
            let frame = std::mem_utils::translate_virt_phys_addr(addr + VirtAddr(i as u64 * 4096)).unwrap();
            phys_addresses.push(frame);
        }

        let prdt = phys_addresses
            .iter()
            .enumerate()
            .map(|(i, addr)| {
                PrdtDescriptor {
                    base: *addr,
                    count: if i == prdt_entries - 1 {
                        (((sec_count - 1) as u32 % 8) + 1) * 512
                    } else {
                        //4K byte regions
                        8 * 512
                    },
                }
            })
            .collect::<Vec<_>>();

        let mut pmport = H2DRegFisPmport(0);
        pmport.set_command(true);

        let cfis = H2DRegisterFis {
            pmport,
            command: READ_DMA,
            device: self.device | (1 << 6),
            countl: sec_count as u8,
            counth: (sec_count >> 8) as u8,
            lba0: (start_sec_index >> 0) as u8,
            lba1: (start_sec_index >> 8) as u8,
            lba2: (start_sec_index >> 16) as u8,
            lba3: (start_sec_index >> 24) as u8,
            lba4: (start_sec_index >> 32) as u8,
            lba5: (start_sec_index >> 40) as u8,
            ..H2DRegisterFis::default()
        };

        let read_cmd_index = self.build_command(false, (&cfis).into(), &prdt).unwrap();

        //for now synchronous
        let mut ci = self.get_property(0x38);
        while ci & (1 << read_cmd_index) != 0 {
            unsafe { core::arch::asm!("hlt") };
            ci = self.get_property(0x38);
        }

        read_cmd_index as u64
    }

    ///Returns the virtual address of the read data and the command index used
    fn write(&mut self, start_sec_index: usize, sec_count: usize, buffer: VirtAddr) -> u64 {
        assert!(sec_count <= self.sectors as usize);
        let prdt_entries = (sec_count + 7) / 8; //8 sectors in one physical frame
        let mut phys_addresses = Vec::new();

        //here we pray that either AHCI supports 64 bit addressing or we are in low memory
        for _ in 0..prdt_entries {
            let virt_addr = buffer + VirtAddr(phys_addresses.len() as u64 * 4096);
            let physical_addr = std::mem_utils::translate_virt_phys_addr(virt_addr).unwrap();
            if physical_addr.0 > u32::MAX as u64 && !self.is_64_bit {
                panic!("AHCI controller does not support 64 bit addressing");
            }
            phys_addresses.push(physical_addr);
        }

        let prdt = phys_addresses
            .iter()
            .enumerate()
            .map(|(i, addr)| {
                PrdtDescriptor {
                    base: *addr,
                    count: if i == prdt_entries - 1 {
                        (((sec_count - 1) as u32 % 8) + 1) * 512
                    } else {
                        //4K byte regions
                        8 * 512
                    },
                }
            })
            .collect::<Vec<_>>();

        let mut pmport = H2DRegFisPmport(0);
        pmport.set_command(true);

        let cfis = H2DRegisterFis {
            pmport,
            command: WRITE_DMA,
            device: self.device | (1 << 6),
            countl: sec_count as u8,
            counth: (sec_count >> 8) as u8,
            lba0: (start_sec_index >> 0) as u8,
            lba1: (start_sec_index >> 8) as u8,
            lba2: (start_sec_index >> 16) as u8,
            lba3: (start_sec_index >> 24) as u8,
            lba4: (start_sec_index >> 32) as u8,
            lba5: (start_sec_index >> 40) as u8,
            ..H2DRegisterFis::default()
        };

        let write_cmd_index = self.build_command(true, (&cfis).into(), &prdt).unwrap();

        //for now synchronous
        let mut ci = self.get_property(0x38);
        while ci & (1 << write_cmd_index) != 0 {
            unsafe { core::arch::asm!("hlt") };
            ci = self.get_property(0x38);
        }

        write_cmd_index as u64
    }

    fn clean_after_read(&mut self, metadata: u64) {
        self.clean_command(metadata as u8);
    }

    fn clean_after_write(&mut self, metadata: u64) {
        self.clean_command(metadata as u8);
    }
}

bitfield! {
    struct CmdHeader(u128);
    impl Debug;
    CFL, SetCFL: 4, 0;
    Atapi, SetAtapi: 5;
    Write, SetWrite: 6;
    Prefetchable, SetPrefetchable: 7;
    Reset, SetReset: 8;
    Bist, SetBist: 9;
    ClearBusy, SetClearBusy: 10;
    PMP, SetPMP: 15, 12;
    PRDTL, SetPRDTL: 31, 16;
    PRDBC, SetPRDBC: 63, 32;
    CTBA, SetCTBA: 127, 64;

}

struct PrdtDescriptor {
    base: PhysAddr,
    count: u32,
}

bitfield! {
    struct PrdtEntry(u128);
    impl Debug;
    DBA, SetDBA: 63, 0;
    DBC, SetDBC: 117, 96;
    Int, SetInt: 127;
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
