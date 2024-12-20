
pub struct PkgLength {
    length: u32,
}

impl PkgLength {
    pub fn new(data: &[u8]) -> (Self, usize) {
        let byte_length = (data[0] >> 6) & 0b11;
        match byte_length {
            0 => (Self { length: data[0] as u32 }, 1),
            1 => (Self { length: u16::from_le_bytes([data[0] & 0xF, data[1]]) as u32 }, 2),
            2 => (Self { length: u32::from_le_bytes([0, data[0] & 0xF, data[1], data[2]]) }, 3),
            3 => (Self { length: u32::from_le_bytes([data[0] & 0xF, data[1], data[2], data[3]]) }, 4),
            _ => panic!("invalid byte length"),
        }
    }

    pub fn get_length(&self) -> usize {
        self.length as usize
    }
}

pub struct Packgage {
    //TODO:
}
