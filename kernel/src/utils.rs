use core::arch::asm;

pub fn byte_to_port(port: u16, byte: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") byte);
    }
}

pub fn byte_form_port(port: u16) -> u8 {
    unsafe {
        let byte;
        asm!("in al, dx", in("dx") port, out("al") byte);
        byte
    }
}
