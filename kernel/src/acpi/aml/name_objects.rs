use core::fmt::Debug;
use std::{
    boxed::Box,
    string::{String, ToString},
    Vec,
};

use super::{
    arg_local_obj::{ArgObj, LocalObj},
    data_object::EXT_OP_PREFIX,
    expression_opcodes::{DefDerefOf, DefIndex, DefRefOf},
};

use macros::*;
use traits::*;

const ROOT_CHAR: u8 = 0x5C;
const PARENT_PREFIX_CHAR: u8 = 0x5E;
const UNDERSCORE: u8 = 0x5F;
const NULL_NAME: u8 = 0x0;
const MULTI_NAME_PREFIX: u8 = 0x2F;
const DUAL_NAME_PREFIX: u8 = 0x2E;
const DEBUG_OP: [u8; 2] = [EXT_OP_PREFIX, 0x31];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct NameSeg {
    name: [u8; 4],
}

impl<'a> core::convert::From<&'a NameSeg> for &'a [NameSeg] {
    fn from(name_seg: &'a NameSeg) -> Self {
        core::slice::from_ref(name_seg)
    }
}

impl AmlNew for NameSeg {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data.len() < 4 {
            return None;
        }
        if !is_lead_name_char(data[0]) || !is_name_char(data[1]) || !is_name_char(data[2]) || !is_name_char(data[3]) {
            return None;
        }

        Some((
            NameSeg {
                name: [data[0], data[1], data[2], data[3]],
            },
            4,
        ))
    }
}

impl core::fmt::Debug for NameSeg {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let name = unsafe { core::str::from_utf8_unchecked(&self.name) };
        write!(f, "\"{}\"", name)
    }
}

impl std::convert::From<&NameSeg> for String {
    fn from(name_seg: &NameSeg) -> Self {
        unsafe { core::str::from_utf8_unchecked(&name_seg.name) }.to_string()
    }
}

impl std::convert::From<&str> for NameSeg {
    fn from(name: &str) -> Self {
        debug_assert!(name.len() == 4);
        let bytes = name.as_bytes();
        NameSeg {
            name: [bytes[0], bytes[1], bytes[2], bytes[3]],
        }
    }
}

fn is_lead_name_char(c: u8) -> bool {
    (0x41..=0x5a).contains(&c) || c == 0x5f
}

fn is_name_char(c: u8) -> bool {
    is_lead_name_char(c) || (0x30..=0x39).contains(&c)
}

#[derive(Clone)]
pub enum NameString {
    Rootchar(Box<NamePath>),
    PrefixPath(Box<(u8, NamePath)>),
    BlankPath(Box<NamePath>),
}

impl NameString {
    pub fn is_null(&self) -> bool {
        match self {
            NameString::BlankPath(name_path) => matches!(**name_path, NamePath::NullName(_)),
            NameString::PrefixPath(name_path) => matches!(name_path.1, NamePath::NullName(_)),
            NameString::Rootchar(name_path) => matches!(**name_path, NamePath::NullName(_)),
        }
    }
}

impl AmlNew for NameString {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] == ROOT_CHAR {
            let name_path = NamePath::aml_new(&data[1..])?;
            Some((Self::Rootchar(Box::new(name_path.0)), name_path.1 + 1))
        } else {
            let mut count = 0;
            while data[count] == PARENT_PREFIX_CHAR {
                count += 1;
            }
            if count != 0 {
                panic!("we hit it");
            }
            let name_path = NamePath::aml_new(&data[count..])?;
            if count == 0 {
                return Some((Self::BlankPath(Box::new(name_path.0)), name_path.1));
            }
            Some((Self::PrefixPath(Box::new((count as u8, name_path.0))), name_path.1 + count))
        }
    }
}

impl std::convert::From<NameString> for String {
    fn from(name_string: NameString) -> Self {
        let mut res = "".to_string();
        let name_seq: &[NameSeg] = match &name_string {
            NameString::Rootchar(name_path) | NameString::BlankPath(name_path) => (name_path).into(),
            NameString::PrefixPath(prefix_path) => {
                for _ in 0..prefix_path.0 {
                    res.push('\\');
                }
                (&prefix_path.1).into()
            }
        };
        let name_seq = name_seq.iter().map(|seg| seg.into()).collect::<Vec<String>>().join("");
        res.push_str(&name_seq);
        res
    }
}

impl Debug for NameString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let name = String::from(self.clone());
        Debug::fmt(&name, f)
    }
}

#[derive(EnumNewMacro, Debug, Clone)]
pub enum NamePath {
    Single(NameSeg),
    Dual(DualNamePath),
    Multi(MultiNamePath),
    NullName(NullName),
}

impl<'a> std::convert::From<&'a Box<NamePath>> for &'a [NameSeg] {
    fn from(name_path: &'a Box<NamePath>) -> Self {
        match &**name_path {
            NamePath::Single(single_name_path) => core::slice::from_ref(single_name_path),
            NamePath::Dual(dual_name_path) => &dual_name_path.segments,
            NamePath::Multi(multi_name_path) => &multi_name_path.segments,
            NamePath::NullName(_) => &[],
        }
    }
}

impl<'a> std::convert::From<&'a NamePath> for &'a [NameSeg] {
    fn from(name_path: &'a NamePath) -> Self {
        match name_path {
            NamePath::Single(single_name_path) => core::slice::from_ref(single_name_path),
            NamePath::Dual(dual_name_path) => &dual_name_path.segments,
            NamePath::Multi(multi_name_path) => &multi_name_path.segments,
            NamePath::NullName(_) => &[],
        }
    }
}

impl std::convert::From<Box<NamePath>> for Box<[NameSeg]> {
    fn from(name_path: Box<NamePath>) -> Self {
        match *name_path {
            NamePath::Single(single_name_path) => Box::new([single_name_path]),
            NamePath::Dual(dual_name_path) => dual_name_path.segments.into(),
            NamePath::Multi(multi_name_path) => multi_name_path.segments.into(),
            NamePath::NullName(_) => Box::new([]),
        }
    }
}

//identifiable with 0x2E
#[derive(Debug, Clone)]
pub struct DualNamePath {
    segments: [NameSeg; 2],
}

impl AmlNew for DualNamePath {
    //include prefix returns number of bytes to skip
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != DUAL_NAME_PREFIX {
            return None;
        }
        let (seg_1, seg_2) = (
            NameSeg::aml_new(&data[1..]).unwrap().0,
            NameSeg::aml_new(&data[5..]).unwrap().0,
        );
        Some((
            DualNamePath {
                segments: [seg_1, seg_2],
            },
            9,
        ))
    }
}

//identifiable with 0x2F
#[derive(Debug, Clone)]
pub struct MultiNamePath {
    segments: std::Vec<NameSeg>,
}

impl AmlNew for MultiNamePath {
    //include prefix and segcount, returns number of bytes to skip
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != MULTI_NAME_PREFIX {
            return None;
        }
        let len = data[1];
        let mut buf = std::Vec::with_capacity(len as usize);
        for i in 0..len {
            let (seg, _skip) = NameSeg::aml_new(&data[2 + i as usize * 4..])?;
            buf.push(seg);
        }
        Some((MultiNamePath { segments: buf }, len as usize * 4 + 2))
    }
}

#[derive(StructNewMacro, Debug, Clone)]
#[op_prefix(NULL_NAME)]
pub struct NullName;

#[derive(Debug, EnumNewMacro)]
pub enum SimpleName {
    Local(Box<LocalObj>),
    Arg(Box<ArgObj>),
    NameString(Box<NameString>),
}

#[derive(Debug, EnumNewMacro)]
pub enum SuperName {
    SimpleName(SimpleName),
    DebugObj(DebugObj),
    ReferenceTypeOpcode(ReferenceTypeOpcode),
}

#[derive(StructNewMacro, Debug)]
#[ext_op_prefix(DEBUG_OP)]
pub struct DebugObj;

#[derive(EnumNewMacro, Debug)]
pub enum ReferenceTypeOpcode {
    RefOf(Box<DefRefOf>),
    DerefOf(DefDerefOf),
    Index(Box<DefIndex>),
}

#[derive(EnumNewMacro, Debug)]
pub enum Target {
    SuperName(SuperName),
    SimpleName(SimpleName),
}
