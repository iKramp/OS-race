
pub struct PkgLength {
    length: u32,
}

impl PkgLength {
    pub fn new(data: &[u8]) -> Self {
        let byte_length = (data[0] >> 6) & 0b11;
        match byte_length {
            0 => Self { length: data[0] as u32 },
            1 => Self { length: u16::from_le_bytes([data[0] & 0xF, data[1]]) as u32 },
            2 => Self { length: u32::from_le_bytes([0, data[0] & 0xF, data[1], data[2]]) },
            3 => Self { length: u32::from_le_bytes([data[0] & 0xF, data[1], data[2], data[3]]) },
            _ => panic!("invalid byte length"),
        }
    }
}

pub struct Packgage {
    //TODO:
}
