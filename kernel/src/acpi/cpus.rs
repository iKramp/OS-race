use crate::{ap_startup::ap_started_wait_loop, interrupts::idt::TablePointer, memory::{paging::PageTree, PAGE_TREE_ALLOCATOR}, println};
use std::{
    eh::int3,
    mem_utils::{get_at_virtual_addr, VirtAddr},
    PageAllocator,
};

//custom data starts at 0x4 from ap_startup

use super::{platform_info::PlatformInfo, LapicRegisters, LAPIC_REGISTERS};

pub fn wake_cpus(platform_info: &PlatformInfo) {
    println!("Copying trampoline code");
    copy_trampoline();
    println!(
        "wait function ptr is {:#x?}",
        crate::ap_startup::ap_started_wait_loop as *const () as u64
    );

    let start_page = unsafe { crate::memory::TRAMPOLINE_RESERVED.0 } >> 12;
    unsafe {
        let stack_addr = crate::memory::PAGE_TREE_ALLOCATOR.allocate_contigious(2); //2 pages
        let destination = crate::memory::TRAMPOLINE_RESERVED.0 as *mut u8;
        for i in 0..8 {
            destination.add(28 + i).write_volatile((stack_addr.0 >> (i * 8)) as u8);
        }
        for cpu in &platform_info.application_processors {
            let lapic_registers = get_at_virtual_addr::<LapicRegisters>(LAPIC_REGISTERS);
            println!("Waking up CPU {}", cpu.apic_id);

            println!("Sending init ipi {}", cpu.apic_id);
            lapic_registers.send_init_ipi(cpu.apic_id);
            std::thread::sleep(std::time::Duration::from_millis(10));

            println!("Sending sipi {}", cpu.apic_id);
            lapic_registers.send_startup_ipi(cpu.apic_id, start_page as u8);
            std::thread::sleep(std::time::Duration::from_millis(100));

            println!("Sending sipi {}", cpu.apic_id);
            lapic_registers.send_startup_ipi(cpu.apic_id, start_page as u8);

            send_bytes(0x12345678, destination.add(44));
        }
    }
}

fn copy_trampoline() {
    let destination = unsafe { crate::memory::TRAMPOLINE_RESERVED };
    let destination_entry = unsafe { PAGE_TREE_ALLOCATOR.get_page_table_entry_mut(VirtAddr(destination.0)) };
    destination_entry.set_write_through_cahcing(true);
    destination_entry.set_disable_cahce(true);
    println!("copying trampoline to {:#x?}", destination);

    assert!(
        destination.0 <= 0xFFFFF,
        "memory addresss should be less than 1MB to initialize APs"
    );
    //let source = VirtAddr(crate::ap_startup::ap_startup as *const () as *const u64 as u64);
    let source = crate::ap_startup::ap_startup as *const () as *const u8;
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
        let wait_loop_ptr = crate::ap_startup::ap_started_wait_loop as *const () as u64;

        println!("destination: {:#x?}", destination);
        println!("GDTP: {:#x?}", gdt_ptr);
        println!("cr3: {:#x?}", cr3);

        (destination.add(4) as *mut u32).write_unaligned(destination as u32);
        (destination.add(10) as *mut TablePointer).write_unaligned(gdt_ptr);
        (destination.add(20) as *mut u64).write_unaligned(cr3);
        for i in 0..8 {
            (destination.add(36 + i) as *mut u8).write_volatile((wait_loop_ptr >> (i * 8)) as u8);
        }
    }
    int3();
}

fn send_bytes(data: u32, comm_lock: *mut u8) {
    for i in 0..4 {
        unsafe { send_byte((data >> (i * 8)) as u8, comm_lock.add(i)) };
    }
}

fn send_byte(data_byte: u8, comm_lock: *mut u8) {
    unsafe {
        let mut byte;
        loop {
            byte = 1_u8;
            core::arch::asm!(
                "xchg {byte}, [{comm_lock}]",
                byte = inout(reg_byte) byte,
                comm_lock = in(reg) comm_lock,
            );
            if byte != 0 {
                continue;
            }
            let data_ready: u8;
            core::arch::asm!(
                "mov {byte}, [{comm_lock}]",
                byte = out(reg_byte) data_ready,
                comm_lock = in(reg) comm_lock.add(1),
            );
            if data_ready == 1 {
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
        core::arch::asm!(
            "mov {data}, [{comm_lock}]",
            data = in(reg_byte) data_byte,
            comm_lock = in(reg) comm_lock.add(2),
        );
        core::arch::asm!(//release lock
            "mov [{comm_lock}], {zero}",
            comm_lock = in(reg) comm_lock,
            zero = in(reg_byte) 0_u8,
        );
    }
}
