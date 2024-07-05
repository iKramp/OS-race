use core::arch::asm;

pub fn byte_to_port(port: u16, byte: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") byte);
    }
}
