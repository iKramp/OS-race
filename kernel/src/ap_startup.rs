use crate::{interrupts::idt::IDT_POINTER, msr};

#[link(name = "ap_startup", kind = "static")]
extern "C" {
    pub fn ap_startup() -> !;
}

pub static mut NUM_CPUS: u64 = 1;

#[no_mangle]
pub extern "C" fn ap_started_wait_loop() -> ! {
    //let comm_lock;
    unsafe {
        //let a = NUM_CPUS;
        //core::arch::asm!(//pull the argument
        //    "mov {comm_lock}, rdi",
        //    comm_lock = out(reg) comm_lock
        //);
        //let mtrr = msr::get_mtrr();
        //let _num = read_4_bytes(comm_lock);
        //core::arch::asm!("lidt [{}]", "sti", in(reg) core::ptr::addr_of!(IDT_POINTER));
    }
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

pub fn read_4_bytes(comm_lock: *mut bool) -> u32 {
    (get_next_byte(comm_lock) as u32) << (0 * 8);
    0
}

#[inline]
pub fn get_next_byte(comm_lock: *mut bool) -> u8 {
    unsafe {
        let mut byte;
        loop {
            byte = 1;
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
                "mov {data_ready}, [{comm_lock}]",
                data_ready = out(reg_byte) data_ready,
                comm_lock = in(reg) comm_lock.add(1),
            );
            if data_ready == 0 {
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
            data = out(reg_byte) byte,
            comm_lock = in(reg) comm_lock.add(2),
        );
        core::arch::asm!(
            "mov [{comm_lock}], {zero}",
            comm_lock = in(reg) comm_lock,
            zero = in(reg_byte) 0_u8,
        );
        byte
    }
}
