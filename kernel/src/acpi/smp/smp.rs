use crate::{
    interrupts::{
        idt::{TablePointer, IDT_POINTER},
        GDT_POINTER,
    },
    memory::PAGE_TREE_ALLOCATOR,
    msr::{get_msr, get_mtrr_cap, get_mtrr_def_type},
    println,
};
use core::sync::atomic::{AtomicBool, AtomicU8};
use std::{
    mem_utils::{get_at_virtual_addr, VirtAddr},
    PageAllocator,
};

const STACK_SIZE_PAGES: usize = 2;
pub static mut CPU_LOCK: AtomicBool = AtomicBool::new(false);
pub static mut CPUS_INITIALIZED: AtomicU8 = AtomicU8::new(0);
pub static mut CPU_LOCALS: Option<std::Vec<VirtAddr>> = None;

//custom data starts at 0x4 from ap_startup

use crate::acpi::{platform_info::PlatformInfo, LapicRegisters, LAPIC_REGISTERS};

pub fn wake_cpus(platform_info: &PlatformInfo) {
    copy_trampoline();

    let start_page = unsafe { crate::memory::TRAMPOLINE_RESERVED.0 } >> 12;
    unsafe {
        CPU_LOCALS = Some(std::Vec::new_with_capacity(platform_info.application_processors.len() + 1));
        let bsp_local = super::cpu_locals::CpuLocals::new(
            0,
            0,
            platform_info.boot_processor.apic_id,
            platform_info.boot_processor.processor_id,
        );
        let bsp_local_ptr = add_cpu_locals(bsp_local);
        crate::msr::set_msr(0xC0000101, bsp_local_ptr.0);


        let destination = crate::memory::TRAMPOLINE_RESERVED.0 as *mut u8;
        let comm_lock = destination.add(56);
        for cpu in platform_info.application_processors.iter().enumerate() {
            let stack_addr = crate::memory::PAGE_TREE_ALLOCATOR.allocate_contigious(STACK_SIZE_PAGES as u64); //2 pages
            (destination.add(32) as *mut u64).write_volatile(stack_addr.0 + (STACK_SIZE_PAGES * 0x1000) as u64);
            let lapic_registers = get_at_virtual_addr::<LapicRegisters>(LAPIC_REGISTERS);
            println!("Waking up CPU {}", cpu.1.apic_id);

            lapic_registers.send_init_ipi(cpu.1.apic_id);
            std::thread::sleep(std::time::Duration::from_millis(10));

            lapic_registers.send_startup_ipi(cpu.1.apic_id, start_page as u8);
            std::thread::sleep(std::time::Duration::from_millis(100));

            lapic_registers.send_startup_ipi(cpu.1.apic_id, start_page as u8);

            send_mtrrs(comm_lock);
            send_cr_registers(comm_lock);
            
            let ap_local = super::cpu_locals::CpuLocals::new(
                stack_addr.0,
                STACK_SIZE_PAGES as u64 * 0x1000,
                cpu.1.apic_id,
                cpu.1.processor_id,
            );
            let ap_local_ptr = add_cpu_locals(ap_local);

            send_cpu_locals(ap_local_ptr.0, comm_lock);
            wait_for_cpus(cpu.0 as u8 + 1);
        }
    }
}

fn add_cpu_locals(locals: super::cpu_locals::CpuLocals) -> VirtAddr {
    unsafe {
        if let Some(cpu_locals) = &mut CPU_LOCALS {
            let ptr = std::Box::leak(std::Box::new(locals)) as *const _ as u64;
            cpu_locals.push(VirtAddr(ptr));
            VirtAddr(cpu_locals.last().unwrap() as *const _ as u64)
        } else {
            panic!("CPU_LOCALS not initialized");
        }
    }
}

fn copy_trampoline() {
    let destination = unsafe { crate::memory::TRAMPOLINE_RESERVED };
    let destination_entry = unsafe { PAGE_TREE_ALLOCATOR.get_page_table_entry_mut(VirtAddr(destination.0)) };
    destination_entry.set_write_through_cahcing(true);
    destination_entry.set_disable_cahce(true);
    println!("copying trampoline to {:x?}", destination);

    assert!(
        destination.0 <= 0xFFFFF,
        "memory addresss should be less than 1MB to initialize APs"
    );
    let source = super::ap_startup::ap_startup as *const () as *const u8;
    let destination = destination.0 as *mut u8;
    for i in 0..0x1000 {
        unsafe {
            destination.add(i).write_volatile(source.add(i).read_volatile());
        }
    }

    let cr3: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, cr3",
            out(reg) cr3,
        );
        let gdt_ptr = crate::interrupts::GDT_POINTER;
        let gdt_ptr = TablePointer {
            limit: gdt_ptr.limit,
            base: std::mem_utils::translate_virt_phys_addr(VirtAddr(gdt_ptr.base)).unwrap().0,
        };
        let wait_loop_ptr = super::ap_startup::ap_started_wait_loop as *const () as u64;

        (destination.add(4) as *mut u32).write_volatile(destination as u32);
        (destination.add(14) as *mut TablePointer).write_volatile(gdt_ptr);

        (destination.add(24) as *mut u64).write_volatile(cr3);
        (destination.add(40) as *mut u64).write_volatile(wait_loop_ptr);
        (destination.add(48) as *mut u64).write_volatile(get_mtrr_def_type());
    }
}

pub fn wait_for_cpus(num_cpus: u8) {
    loop {
        let mut cpus_initialized = unsafe { CPUS_INITIALIZED.load(core::sync::atomic::Ordering::Relaxed) };
        if cpus_initialized == num_cpus {
            break;
        }
    }
}

fn send_cpu_locals(ptr: u64, comm_lock: *mut u8) {
    //TODO:
    send_u64(ptr, comm_lock);
}

fn send_mtrrs(comm_lock: *mut u8) {
    send_u64(get_mtrr_def_type(), comm_lock);
    send_u64(get_msr(0x250), comm_lock);
    send_u64(get_msr(0x258), comm_lock);
    send_u64(get_msr(0x259), comm_lock);
    send_u64(get_msr(0x268), comm_lock);
    send_u64(get_msr(0x269), comm_lock);
    send_u64(get_msr(0x26A), comm_lock);
    send_u64(get_msr(0x26B), comm_lock);
    send_u64(get_msr(0x26C), comm_lock);
    send_u64(get_msr(0x26D), comm_lock);
    send_u64(get_msr(0x26E), comm_lock);
    send_u64(get_msr(0x26F), comm_lock);

    let n = get_mtrr_cap() & 0xFF;
    for i in 0..n {
        send_u64(get_msr(0x200 + (i as u32 * 2)), comm_lock);
        send_u64(get_msr(0x201 + (i as u32 * 2)), comm_lock);
    }

    send_u64(get_msr(0xC0000080), comm_lock);
}

fn send_cr_registers(comm_lock: *mut u8) {
    unsafe {
        let cr0: u64;
        let cr3: u64;
        let cr4: u64;

        core::arch::asm!(
            "mov {cr0}, cr0",
            "mov {cr3}, cr3",
            "mov {cr4}, cr4",

            cr0 = out(reg) cr0,
            cr3 = out(reg) cr3,
            cr4 = out(reg) cr4
        );
        send_u64(cr0, comm_lock);
        send_u64(cr3, comm_lock);
        send_u64(cr4, comm_lock);
    }
}

fn send_dts(comm_lock: *mut u8) {
    unsafe {
        send_u16(GDT_POINTER.limit, comm_lock);
        send_u64(GDT_POINTER.base, comm_lock);
    }

    let idt_ptr_addr = core::ptr::addr_of!(IDT_POINTER);
    send_u64(idt_ptr_addr as u64, comm_lock);
}

fn send_u64(data: u64, comm_lock: *mut u8) {
    send_bytes(&data.to_ne_bytes(), comm_lock);
}

fn send_u16(data: u16, comm_lock: *mut u8) {
    send_bytes(&data.to_ne_bytes(), comm_lock);
}

fn send_bytes(data: &[u8], comm_lock: *mut u8) {
    for byte in data {
        send_byte(*byte, comm_lock);
    }
}

fn send_byte(data_byte: u8, comm_lock: *mut u8) {
    unsafe {
        let mut byte;
        loop {
            byte = 1_u8;
            core::arch::asm!(//obtain comm lock
                "xchg {byte}, [{comm_lock}]",
                byte = inout(reg_byte) byte,
                comm_lock = in(reg) comm_lock,
            );
            if byte != 0 {
                continue;
            }
            let data_ready: u8;
            core::arch::asm!(//check if there's pending data
                "mov {byte}, [{comm_lock}]",
                byte = out(reg_byte) data_ready,
                comm_lock = in(reg) comm_lock.add(1),
            );
            if data_ready == 1 {
                //ap didn't read yet
                core::arch::asm!(//release lock
                    "mov [{comm_lock}], {zero}",
                    "clflush [{comm_lock}]",
                    comm_lock = in(reg) comm_lock,
                    zero = in(reg_byte) 0_u8,
                );
                continue;
            } else {
                break;
            }
        }
        core::arch::asm!(//write data
            "mov [{comm_lock}], {data}",
            "clflush [{comm_lock}]",
            data = in(reg_byte) data_byte,
            comm_lock = in(reg) comm_lock.add(2),
        );
        core::arch::asm!(//set pending data
            "mov [{comm_lock}], {one}",
            "clflush [{comm_lock}]",
            one = in(reg_byte) 1_u8,
            comm_lock = in(reg) comm_lock.add(1),
        );
        core::arch::asm!(//release lock
            "mov [{comm_lock}], {zero}",
            "clflush [{comm_lock}]",
            comm_lock = in(reg) comm_lock,
            zero = in(reg_byte) 0_u8,
        );
    }
}