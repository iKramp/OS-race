#![allow(clippy::erasing_op)]

use crate::interrupts::idt::IDT_POINTER;
use crate::interrupts::GDT_POINTER;
use crate::println;
use core::arch::asm;

use crate::{
    memory::paging::PageTree,
    msr::{get_mtrr_cap, set_msr, set_mtrr_def_type},
};

#[link(name = "ap_startup", kind = "static")]
extern "C" {
    pub fn ap_startup() -> !;
}

#[no_mangle]
pub extern "C" fn ap_started_wait_loop() -> ! {
    let comm_lock: *mut u8;
    unsafe {
        core::arch::asm!(//pull the argument
            "mov {comm_lock}, rdi",
            comm_lock = out(reg) comm_lock
        );
    }

    set_mtrrs(comm_lock);
    set_cr_registers(comm_lock);
    set_gdt();
    set_idt();
    PageTree::reload();

    set_cpu_local(comm_lock);
    let locals = super::cpu_locals::CpuLocals::get();
    let processor_id = locals.processor_id;
    crate::acpi::init_acpi_ap(processor_id);

    set_initialized();
    println!("AP: cpu woke up and received all data");

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

fn set_initialized() {
    unsafe {
        let mut lock: u8 = 1;
        while lock == 1 {
            core::arch::asm!(
                "xchg {control}, [{lock}]",
                control = inout(reg_byte) lock,
                lock = in(reg) core::ptr::addr_of!(crate::acpi::CPU_LOCK)
            )
        }

        let num = core::ptr::addr_of!(crate::acpi::CPUS_INITIALIZED).read_volatile();

        core::arch::asm!(
            "mov [{initialized}], {num}",
            "mov [{lock}], {zero}",
            "clflush [{lock}]",
            "clflush [{initialized}]",
            zero = in(reg_byte) 0_u8,
            num = in(reg_byte) num + 1,
            lock = in(reg) core::ptr::addr_of!(crate::acpi::CPU_LOCK),
            initialized = in(reg) core::ptr::addr_of!(crate::acpi::CPUS_INITIALIZED)
        )
    }
}

fn set_cpu_local(comm_lock: *mut u8) {
    let cpu_local_ptr = read_8_bytes(comm_lock);
    crate::msr::set_msr(0xC0000101, cpu_local_ptr);
}

fn set_gdt() {
    unsafe {
        core::arch::asm!("lgdt [{}]", in(reg) core::ptr::addr_of!(GDT_POINTER), options(readonly, nostack, preserves_flags));
    }
    crate::interrupts::set_cs();
}

fn set_idt() {
    unsafe {
        asm!("lidt [{}]", "sti", in(reg) core::ptr::addr_of!(IDT_POINTER));
    }
}

fn set_cr_registers(comm_lock: *mut u8) {
    unsafe {
        let cr0 = read_8_bytes(comm_lock);
        let cr3 = read_8_bytes(comm_lock);
        let cr4 = read_8_bytes(comm_lock);

        core::arch::asm!(
            "mov cr0, {cr0}",
            "mov cr4, {cr4}",
            "mov cr3, {cr3}",
            cr0 = in(reg) cr0,
            cr3 = in(reg) cr3,
            cr4 = in(reg) cr4
        );
    }
}

fn set_mtrrs(comm_lock: *mut u8) {
    let mtrr_def = read_8_bytes(comm_lock);
    set_mtrr_def_type(mtrr_def);
    set_msr(0x250, read_8_bytes(comm_lock)); //fixed range
    set_msr(0x258, read_8_bytes(comm_lock));
    set_msr(0x259, read_8_bytes(comm_lock));
    set_msr(0x268, read_8_bytes(comm_lock));
    set_msr(0x269, read_8_bytes(comm_lock));
    set_msr(0x26A, read_8_bytes(comm_lock));
    set_msr(0x26B, read_8_bytes(comm_lock));
    set_msr(0x26C, read_8_bytes(comm_lock));
    set_msr(0x26D, read_8_bytes(comm_lock));
    set_msr(0x26E, read_8_bytes(comm_lock));
    set_msr(0x26F, read_8_bytes(comm_lock));

    let n = get_mtrr_cap() & 0xFF;

    for i in 0..n {
        set_msr(0x200 + (i as u32 * 2), read_8_bytes(comm_lock));
        set_msr(0x201 + (i as u32 * 2), read_8_bytes(comm_lock));
    }

    set_msr(0xC0000080, read_8_bytes(comm_lock));
}

fn read_8_bytes(comm_lock: *mut u8) -> u64 {
    read_4_bytes(comm_lock) as u64 | (read_4_bytes(comm_lock) as u64) << 32
}

fn read_4_bytes(comm_lock: *mut u8) -> u32 {
    read_2_bytes(comm_lock) as u32 | (read_2_bytes(comm_lock) as u32) << 16
}

fn read_2_bytes(comm_lock: *mut u8) -> u16 {
    (get_next_byte(comm_lock) as u16) | (get_next_byte(comm_lock) as u16) << 8
}

#[inline]
fn get_next_byte(comm_lock: *mut u8) -> u8 {
    unsafe {
        let mut byte;
        loop {
            byte = 1;
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
                "mov {data_ready}, [{comm_lock}]",
                data_ready = out(reg_byte) data_ready,
                comm_lock = in(reg) comm_lock.add(1),
            );
            if data_ready == 0 {
                //bsp didn't write yet
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
        core::arch::asm!(//read data
            "mov {data}, [{comm_lock}]",
            data = out(reg_byte) byte,
            comm_lock = in(reg) comm_lock.add(2),
        );
        core::arch::asm!(//unset pending data
            "mov [{comm_lock}], {zero}",
            "clflush [{comm_lock}]",
            zero = in(reg_byte) 0_u8,
            comm_lock = in(reg) comm_lock.add(1),
        );
        core::arch::asm!(//release lock
            "mov [{comm_lock}], {zero}",
            "clflush [{comm_lock}]",
            comm_lock = in(reg) comm_lock,
            zero = in(reg_byte) 0_u8,
        );
        byte
    }
}
