use super::expression_opcodes::BUFFER_OP;

const BYTE_PREFIX: u8 = 0x0A;
const WORD_PREFIX: u8 = 0x0B;
const DWORD_PREFIX: u8 = 0x0C;
const QWORD_PREFIX: u8 = 0x0E;
const STRING_PREFIX: u8 = 0x0D;
const NULL_CHAR: u8 = 0x00;
const ZERO_OP: u8 = 0x00;
const ONE_OP: u8 = 0x01;
const ONES_OP: u8 = 0xFF;
const REVISION_OP: u8 = 0x30;
pub const EXT_OP_PREFIX: u8 = 0x5B;

pub enum ComputationalData {
    ByteConst(ByteConst),
    WordConst(WordConst),
    DWordConst(DWordConst),
    QWordConst(QWordConst),
    String(StringConst),
    ConstObj(ConstObj),
    RevisionOp,
    Buffer(super::expression_opcodes::DefBuffer),
}

impl ComputationalData {
    pub fn new(data: &[u8]) -> Option<(Self, usize)> {//returns if valid, also returns the number
                                                      //of bytes read
        match data[0] {
            BYTE_PREFIX => Some((Self::ByteConst(ByteConst::new(data)), 2)),
            WORD_PREFIX => Some((Self::WordConst(WordConst::new(data)), 3)),
            DWORD_PREFIX => Some((Self::DWordConst(DWordConst::new(data)), 5)),
            QWORD_PREFIX => Some((Self::QWordConst(QWordConst::new(data)), 9)),
            STRING_PREFIX => {
                let str_const = StringConst::new(data);
                let len = str_const.0.len();
                Some((Self::String(str_const), len + 2))//str len + prefix + null char
            },
            ZERO_OP | ONE_OP | ONES_OP => Some((Self::ConstObj(ConstObj::new(data)), 1)),
            EXT_OP_PREFIX => {
                match data[1] {
                    REVISION_OP => Some((Self::RevisionOp, 2)),
                    _ => None,
                }
            }
            BUFFER_OP => {
                let (buffer, skip) = super::expression_opcodes::DefBuffer::new(data);
                Some((Self::Buffer(buffer), skip))
            }
            _ => None,
        }
    }
}

//TODO: 
//Data object
//Data ref object

struct ByteConst(u8);

impl ByteConst {
    pub fn new(data: &[u8]) -> Self {
        //sanity check prefix
        debug_assert_eq!(data[0], BYTE_PREFIX);
        Self(data[1])
    }
}

struct WordConst(u16);

impl WordConst {
    pub fn new(data: &[u8]) -> Self {
        //sanity check prefix
        debug_assert_eq!(data[0], WORD_PREFIX);
        Self(u16::from_le_bytes([data[1], data[2]]))
    }
}

struct DWordConst(u32);

impl DWordConst {
    pub fn new(data: &[u8]) -> Self {
        //sanity check prefix
        debug_assert_eq!(data[0], DWORD_PREFIX);
        Self(u32::from_le_bytes([data[1], data[2], data[3], data[4]]))
    }
}

struct QWordConst(u64);

impl QWordConst {
    pub fn new(data: &[u8]) -> Self {
        //sanity check prefix
        debug_assert_eq!(data[0], QWORD_PREFIX);
        Self(u64::from_le_bytes([data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8]]))
    }
}

struct StringConst(std::Vec<u8>);//ascii chars, terminated by null

impl StringConst {
    pub fn new(data: &[u8]) -> Self {
        //sanity check prefix
        debug_assert_eq!(data[0], STRING_PREFIX);
        let mut vec = std::Vec::new();
        let mut i = 1;
        while data[i] != 0 {
            vec.push(data[i]);
            i += 1;
        }
        Self(vec)
    }
}

enum ConstObj {
    ZeroOp,
    OneOp,
    OnesOp,
}

impl ConstObj {
    pub fn new(data: &[u8]) -> Self {
        match data[0] {
            ZERO_OP => Self::ZeroOp,
            ONE_OP => Self::OneOp,
            ONES_OP => Self::OnesOp,
            _ => panic!("Invalid ConstObj prefix"),
        }
    }
}
