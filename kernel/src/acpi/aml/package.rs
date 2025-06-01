pub struct PkgLength {
    length: u32,
}

impl PkgLength {
    pub fn new(data: &[u8]) -> (Self, usize) {
        let byte_length = (data[0] >> 6) & 0b11;
        match byte_length {
            0 => (
                Self {
                    length: (data[0] & 0b111111) as u32,
                },
                1,
            ),
            1 => {
                let mut num = data[0] as u32 & 0b1111;
                num |= (data[1] as u32) << 4;
                (Self { length: num }, 2)
            }
            2 => {
                let mut num = data[0] as u32 & 0b1111;
                num |= (data[1] as u32) << 4;
                num |= (data[2] as u32) << 12;
                (Self { length: num }, 3)
            }
            3 => {
                let mut num = data[0] as u32 & 0b1111;
                num |= (data[1] as u32) << 4;
                num |= (data[2] as u32) << 12;
                num |= (data[3] as u32) << 20;
                (Self { length: num }, 4)
            }
            _ => panic!("invalid byte length"),
        }
    }

    pub fn get_length(&self) -> usize {
        self.length as usize
    }
}
