use std::Box;

use functions::*;
use macros::*;


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

#[derive(EnumNewMacro)]
pub enum ComputationalData {
    ByteConst(ByteConst),
    WordConst(WordConst),
    DWordConst(DWordConst),
    QWordConst(QWordConst),
    String(StringConst),
    ConstObj(ConstObj),
    RevisionOp(RevisionOp),
    Buffer(super::expression_opcodes::DefBuffer),
}

pub enum DataObject {
    Computational(ComputationalData),
    Package(super::expression_opcodes::DefPackage),
    VarPackage(super::expression_opcodes::DefVarPackage),
}

impl EnumNew for DataObject {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        todo!();
    }
}

pub enum DataRefObject {
    DataObject(DataObject),
    DataRefObject(Box<DataRefObject>),
}

impl DataRefObject {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        todo!();
    }
}

struct ByteConst(u8);

impl ByteConst {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != BYTE_PREFIX {
            return None;
        }
        Some((Self(data[1]), 2))
    }
}

struct WordConst(u16);

impl WordConst {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != WORD_PREFIX {
            return None;
        }
        Some((Self(u16::from_le_bytes([data[1], data[2]])), 3))
    }
}

struct DWordConst(u32);

impl DWordConst {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != DWORD_PREFIX {
            return None;
        }
        Some((Self(u32::from_le_bytes([data[1], data[2], data[3], data[4]])), 5))
    }
}

struct QWordConst(u64);

impl QWordConst {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != QWORD_PREFIX {
            return None;
        }
        Some((Self(u64::from_le_bytes([data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8]])), 9))
    }
}

struct StringConst(std::Vec<u8>);//ascii chars, terminated by null

impl StringConst {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != STRING_PREFIX {
            return None;
        }
        let mut vec = std::Vec::new();
        let mut i = 1;
        while data[i] != 0 {
            vec.push(data[i]);
            i += 1;
        }

        return Some((Self(vec), i + 1));
    }
}

enum ConstObj {
    ZeroOp,
    OneOp,
    OnesOp,
}

impl ConstObj {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        match data[0] {
            ZERO_OP => Some((Self::ZeroOp, 1)),
            ONE_OP => Some((Self::OneOp, 1)),
            ONES_OP => Some((Self::OnesOp, 1)),
            _ => panic!("Invalid ConstObj prefix"),
        }
    }
}

#[derive(StructNewMacro)]
#[op_prefix(REVISION_OP)]
struct RevisionOp;
