use std::Vec;

use functions::EnumNew;

use crate::acpi::aml::name_objects::NameString;

use super::{data_object::EXT_OP_PREFIX, term_objects::TermArg, Integer};

const BANK_FIELD_OP: [u8; 2] = [EXT_OP_PREFIX, 0x87];
const CREATE_BIT_FIELD_OP: u8 = 0x8D;
const CREATE_BYTE_FIELD_OP: u8 = 0x8C;
const CREATE_DWORD_FIELD_OP: u8 = 0x8A;
const CREATE_QWORD_FIELD_OP: u8 = 0x8F;
const CREATE_FIELD: u8 = 0x13;
const CREATE_WORD_FIELD_OP: u8 = 0x8B;
const DATA_REGION_OP: [u8; 2] = [EXT_OP_PREFIX, 0x88];
const DEVICE_OP: [u8; 2] = [EXT_OP_PREFIX, 0x82];
const EVENT_OP: [u8; 2] = [EXT_OP_PREFIX, 0x02];
const EXTERNAL_OP: u8 = 0x15;
const FIELD_OP: [u8; 2] = [EXT_OP_PREFIX, 0x81];
const INDEX_FIELD_OP: [u8; 2] = [EXT_OP_PREFIX, 0x86];
const METHOD_OP: u8 = 0x14;
const MUTEX_OP: [u8; 2] = [EXT_OP_PREFIX, 0x01];
const OP_REGION_OP: [u8; 2] = [EXT_OP_PREFIX, 0x80];
const POWER_RES_OP: [u8; 2] = [EXT_OP_PREFIX, 0x84];
const THERMAL_ZONE_OP: [u8; 2] = [EXT_OP_PREFIX, 0x85];

struct FieldFlags {
    flags: u8,
}

enum FieldFlagsAccessType {
    Any,
    Byte,
    Word,
    DWord,
    QWord,
    Buffer,
}

enum FieldFlagsUpdateRule {
    Preserve,
    WriteAsOnes,
    WriteAsZeros,
}

impl FieldFlags {
    pub fn get_access_type(&self) -> FieldFlagsAccessType {
        match self.flags & 0b111 {
            0 => FieldFlagsAccessType::Any,
            1 => FieldFlagsAccessType::Byte,
            2 => FieldFlagsAccessType::Word,
            3 => FieldFlagsAccessType::DWord,
            4 => FieldFlagsAccessType::QWord,
            5 => FieldFlagsAccessType::Buffer,
            _ => panic!("invalid access type"),
        }
    }

    pub fn has_lock(&self) -> bool {
        self.flags & 0b1000 != 0
    }

    pub fn has_update_rule(&self) -> bool {
        self.flags & 0b10000 != 0
    }
}

pub enum NamedObj {}

impl NamedObj {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        todo!("named obj a bit sus here");
    }
}

struct DefBankField {
    name_1: NameString,
    name_2: NameString,
    bank_value: TermArg,
    field_flags: FieldFlags,
    field_list: FieldList,
}

impl DefBankField {
    pub fn new(data: &[u8]) -> Option<(Self, usize)> {
        //sanity check
        if data[0] != BANK_FIELD_OP[0] || data[1] != BANK_FIELD_OP[1] {
            return None;
        }
        let mut skip = 2;
        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[skip..]);
        skip += skip_pkg_len;
        let (name_1, skip_name_1) = NameString::aml_new(&data[skip..]).unwrap();
        skip += skip_name_1;
        let (name_2, skip_name_2) = NameString::aml_new(&data[skip..]).unwrap();
        skip += skip_name_2;

        let (bank_value, skip_bank_value) = TermArg::aml_new(&data[skip..]).unwrap();
        skip += skip_bank_value;

        let field_flags = FieldFlags { flags: data[skip] };
        skip += 1;

        let (field_list, skip_field_list) = FieldList::new(&data[skip..(pkg_length.get_length() + 2)]).unwrap();
        skip += skip_field_list;

        Some((
            Self {
                name_1,
                name_2,
                bank_value,
                field_flags,
                field_list,
            },
            skip,
        ))
    }
}

struct FieldList {
    fields: Vec<FieldElement>,
}

impl FieldList {
    pub fn new(data: &[u8]) -> Option<(Self, usize)> {
        todo!()
    }
}

enum FieldElement {}
