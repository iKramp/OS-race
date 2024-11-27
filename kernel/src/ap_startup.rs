#![allow(clippy::erasing_op)]

use core::arch::asm;
use crate::println;

use crate::{
    interrupts::idt::TablePointer,
    memory::paging::PageTree,
    msr::{get_mtrr_cap, set_msr, set_mtrr_def_type},
};

#[link(name = "ap_startup", kind = "static")]
extern "C" {
    pub fn ap_startup() -> !;
}

pub static mut NUM_CPUS: u64 = 1;

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
    set_gdt(comm_lock);
    set_idt(comm_lock);
    PageTree::reload();

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

fn set_gdt(comm_lock: *mut u8) {
    let gdt_ptr_limit = read_2_bytes(comm_lock);
    let gdt_ptr_addr = read_8_bytes(comm_lock);
    let gdt_ptr = TablePointer {
        limit: gdt_ptr_limit,
        base: gdt_ptr_addr,
    };
    unsafe {
        core::arch::asm!("lgdt [{}]", in(reg) core::ptr::addr_of!(gdt_ptr), options(readonly, nostack, preserves_flags));
    }
    crate::interrupts::set_cs();
}

fn set_idt(comm_lock: *mut u8) {
    unsafe {
        let idt_ptr_addr = read_8_bytes(comm_lock);
        asm!("lidt [{}]", "sti", in(reg) idt_ptr_addr);
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
            zero = in(reg_byte) 0_u8,
            comm_lock = in(reg) comm_lock.add(1),
        );
        core::arch::asm!(//release lock
            "mov [{comm_lock}], {zero}",
            comm_lock = in(reg) comm_lock,
            zero = in(reg_byte) 0_u8,
        );
        byte
    }
}
