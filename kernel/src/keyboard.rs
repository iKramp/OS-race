pub static mut KEY_STATES: [bool; 128] = [false; 128];

pub fn handle_key(key: u8) {
    unsafe {
        KEY_STATES[key as usize & 0x7F] = key & 0x80 != 0;
    }
}
