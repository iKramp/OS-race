pub mod elf_parser;
pub mod panic;
mod dbg_info_entry;


pub fn int3() {
    unsafe {
        core::arch::asm!("int3");
    }
    crate::thread::sleep(crate::time::Duration::from_secs(1));
}
