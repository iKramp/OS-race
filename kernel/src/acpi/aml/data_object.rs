use std::Box;

use macros::*;
use traits::*;

use super::expression_opcodes::{DefBuffer, DefPackage, DefVarPackage};

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

#[derive(EnumNewMacro, Debug)]
pub enum ComputationalData {
    ByteConst(ByteConst),
    WordConst(WordConst),
    DWordConst(DWordConst),
    QWordConst(QWordConst),
    String(Box<StringConst>),
    ConstObj(ConstObj),
    RevisionOp(RevisionOp),
    Buffer(Box<DefBuffer>),
}

#[derive(EnumNewMacro, Debug)]
pub enum DataObject {
    Computational(ComputationalData),
    Package(Box<DefPackage>),
    VarPackage(Box<DefVarPackage>),
}

#[derive(EnumNewMacro, Debug)]
pub enum DataRefObject {
    DataObject(DataObject),
    //DataRefObject(Box::<DataRefObject>),
}

#[derive(Debug)]
pub struct ByteConst(u8);

impl AmlNew for ByteConst {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != BYTE_PREFIX {
            return None;
        }
        Some((Self(data[1]), 2))
    }
}

#[derive(Debug)]
pub struct WordConst(u16);

impl AmlNew for WordConst {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != WORD_PREFIX {
            return None;
        }
        Some((Self(u16::from_le_bytes([data[1], data[2]])), 3))
    }
}

#[derive(Debug)]
pub struct DWordConst(u32);

impl AmlNew for DWordConst {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != DWORD_PREFIX {
            return None;
        }
        Some((Self(u32::from_le_bytes([data[1], data[2], data[3], data[4]])), 5))
    }
}

#[derive(Debug)]
pub struct QWordConst(u64);

impl AmlNew for QWordConst {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != QWORD_PREFIX {
            return None;
        }
        Some((
            Self(u64::from_le_bytes([
                data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8],
            ])),
            9,
        ))
    }
}

#[derive(Debug)]
pub struct StringConst(std::Vec<u8>); //ascii chars, terminated by null

impl AmlNew for StringConst {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != STRING_PREFIX {
            return None;
        }
        let mut vec = std::Vec::new();
        let mut i = 1;
        while data[i] != 0 {
            vec.push(data[i]);
            i += 1;
        }

        Some((Self(vec), i + 1))
    }
}

#[derive(Debug)]
pub enum ConstObj {
    Zero,
    One,
    Ones,
}

impl AmlNew for ConstObj {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        match data[0] {
            ZERO_OP => Some((Self::Zero, 1)),
            ONE_OP => Some((Self::One, 1)),
            ONES_OP => Some((Self::Ones, 1)),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct ByteData(u8);

impl ByteData {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        Some((Self(data[0]), 1))
    }
}

#[derive(Debug)]
pub struct WordData(u16);

impl WordData {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        Some((Self(u16::from_le_bytes([data[0], data[1]])), 2))
    }
}

#[derive(Debug)]
pub struct DWordData(u32);

impl DWordData {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        Some((Self(u32::from_le_bytes([data[0], data[1], data[2], data[3]])), 4))
    }
}

#[derive(StructNewMacro, Debug)]
#[op_prefix(REVISION_OP)]
pub struct RevisionOp;
