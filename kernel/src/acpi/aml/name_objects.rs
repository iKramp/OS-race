use super::{
    arg_local_obj::{ArgObj, LocalObj, ARG6_OP, LOCAL0_OP, LOCAL7_OP},
    data_object::EXT_OP_PREFIX,
};

use macros::EnumNewMacro;
use functions::EnumNew;

const ROOT_CHAR: u8 = 0x5C;
const PARENT_PREFIX_CHAR: u8 = 0x5E;
const UNDERSCORE: u8 = 0x5F;
const NULL_NAME: u8 = 0x0;
const MULTI_NAME_PREFIX: u8 = 0x2F;
const DUAL_NAME_PREFIX: u8 = 0x2E;

type NameSeg = [u8; 4];

fn is_lead_name_char(c: u8) -> bool {
    (c >= 0x41 && c <= 0x5a) || c == 0x5f
}

fn is_name_char(c: u8) -> bool {
    is_lead_name_char(c) || (c >= 0x30 && c <= 0x39)
}

pub enum NameString {
    Rootchar(NamePath),
    PrefixPath((u8, NamePath)),
}

impl NameString {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] == ROOT_CHAR {
            let name_path = NamePath::new(&data[1..])?;
            return Some((Self::Rootchar(name_path.0), name_path.1.into()));
        } else {
            let mut count = 0;
            while data[count] == PARENT_PREFIX_CHAR {
                count += 1;
            }
            if count == 0 {
                panic!("maybe valid but catch so i can check");
            }
            let name_path = NamePath::new(&data[count..])?;
            return Some((Self::PrefixPath((count as u8, name_path.0)), name_path.1.into()));
        }
    }
}

enum NamePath {
    NameSeg(NameSeg),
    DualNamePath(DualNamePath),
    MultiNamePath(MultiNamePath),
    NullName,
}

impl NamePath {
    pub fn new(data: &[u8]) -> Option<(Self, u8)> {
        //returns skip
        match data[0] {
            NULL_NAME => return Some((Self::NullName, 1)),
            DUAL_NAME_PREFIX => return Some((Self::DualNamePath(DualNamePath::aml_new(data).unwrap().0), 9)),
            MULTI_NAME_PREFIX => {
                let ret_data = MultiNamePath::aml_new(data);
                return Some((Self::MultiNamePath(ret_data.1), ret_data.0));
            }
            _ => {
                let typed_slie = unsafe { core::slice::from_raw_parts(&data[2] as *const _ as *const NameSeg, 1 as usize) };
                if !is_lead_name_char(typed_slie[0][0])
                    || !is_name_char(typed_slie[0][1])
                    || !is_name_char(typed_slie[1][0])
                    || !is_name_char(typed_slie[1][1])
                {
                    return None;
                }
                return Some((Self::NameSeg(typed_slie[0]), 4));
            }
        }
    }
}

//identifiable with 0x2E
struct DualNamePath {
    segments: [NameSeg; 2],
}

impl DualNamePath {
    //include prefix returns number of bytes to skip
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        debug_assert!(data[0] == DUAL_NAME_PREFIX);
        let typed_slie = unsafe { core::slice::from_raw_parts(&data[2] as *const _ as *const NameSeg, 2 as usize) };
        return Some((DualNamePath {
            segments: [typed_slie[0], typed_slie[1]],
        }, 9));
    }
}

//identifiable with 0x2F
struct MultiNamePath {
    segments: std::Vec<NameSeg>,
}

impl MultiNamePath {
    //include prefix and segcount, returns number of bytes to skip
    pub fn aml_new(data: &[u8]) -> (u8, Self) {
        debug_assert!(data[0] == MULTI_NAME_PREFIX);
        let len = data[1];
        let mut buf = std::Vec::new_with_capacity(len as usize);
        let typed_slie = unsafe { core::slice::from_raw_parts(&data[2] as *const _ as *const NameSeg, len as usize) };
        buf.copy_from_slice(typed_slie);
        return (len * 4 + 2, MultiNamePath { segments: buf }); //segments * 4chars + prefix + len
    }
}

//TODO:
//simple name
//super name
//target

#[derive(EnumNewMacro)]
pub enum SimpleName {
    Local(LocalObj),
    Arg(ArgObj),
    NameString(NameString),
}

pub enum SuperName {
    SimpleName(SimpleName),
    DebugObj,
    ReferenceTypeOpcode(),
}

impl SuperName {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] == EXT_OP_PREFIX && data[1] == 0x31 {
            return Some((Self::DebugObj, 2));
        }
        if let Some((name, skip)) = SimpleName::aml_new(data) {
            return Some((Self::SimpleName(name), skip));
        }
        todo!("ReferenceTypeOpcode");
    }
}

#[derive(EnumNewMacro)]
pub enum Target {
    SuperName(SuperName),
    SimpleName(SimpleName),
}

