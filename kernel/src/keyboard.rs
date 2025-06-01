pub static mut KEY_STATES: [bool; 128] = [false; 128];

pub fn handle_key(key: u8) {
    let pressed = key & 0x80 == 0;
    unsafe {
        KEY_STATES[key as usize & 0x7F] = pressed;
    }
    //println!("key action: {}, {}", key & 0x7F, pressed);
}
