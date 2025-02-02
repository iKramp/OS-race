use std::{boxed::Box, Vec};

use macros::*;
use traits::*;

use crate::acpi::aml::name_objects::NameString;

use super::{
    data_object::{ByteData, DWordData, EXT_OP_PREFIX},
    namespace::{self, get_namespace, Namespace},
    term_objects::{TermArg, TermList},
};

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
const RESERVED_FIELD_OP: u8 = 0x00;
const ACCESS_FIELD_OP: u8 = 0x01;
const EXTENDED_ACCESS_FIELD_OP: u8 = 0x03;
const CONNECT_FIELD_OP: u8 = 0x02;
const PROCESSOR_OP: [u8; 2] = [EXT_OP_PREFIX, 0x83];

#[derive(Debug)]
struct FieldFlags {
    flags: u8,
}

#[derive(Debug)]
struct AccessType {
    flags: u8,
}

#[derive(Debug)]
enum FieldFlagsAccessType {
    Any,
    Byte,
    Word,
    DWord,
    QWord,
    Buffer,
}

#[derive(Debug)]
enum FieldFlagsUpdateRule {
    Preserve,
    WriteAsOnes,
    WriteAsZeros,
}

#[derive(Debug)]
enum AccessAttribInner {
    Quick,
    SendReceive,
    Byte,
    Word,
    Block,
    ProcessCall,
    BlockProcessCall,
}

#[derive(Debug)]
enum AccessAttrib {
    Normal,
    AttribBytes,
    AttribRawBytes,
    AttribRawProcessBytes,
}

impl AccessAttrib {
    //only use as extebded access attrib
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        match data[0] {
            0x0B => Some((Self::AttribBytes, 1)),
            0x0E => Some((Self::AttribRawBytes, 1)),
            0x0F => Some((Self::AttribRawProcessBytes, 1)),
            _ => None,
        }
    }
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
        self.flags & 0b10000 != 0
    }

    pub fn get_update_rule(&self) -> FieldFlagsUpdateRule {
        match self.flags & 0b110000 {
            0 => FieldFlagsUpdateRule::Preserve,
            1 => FieldFlagsUpdateRule::WriteAsOnes,
            2 => FieldFlagsUpdateRule::WriteAsZeros,
            _ => panic!("invalid update rule"),
        }
    }
}

impl AccessType {
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

    pub fn get_access_attrib(&self) -> AccessAttrib {
        match (self.flags >> 6) & 0b11 {
            0 => AccessAttrib::Normal,
            1 => AccessAttrib::AttribBytes,
            2 => AccessAttrib::AttribRawBytes,
            3 => AccessAttrib::AttribRawProcessBytes,
            _ => panic!("invalid access attrib"),
        }
    }
}

impl AccessAttribInner {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        match data[0] {
            0x02 => Some((Self::Quick, 1)),
            0x04 => Some((Self::SendReceive, 1)),
            0x06 => Some((Self::Byte, 1)),
            0x08 => Some((Self::Word, 1)),
            0x0A => Some((Self::Block, 1)),
            0x0C => Some((Self::ProcessCall, 1)),
            0x0D => Some((Self::BlockProcessCall, 1)),
            _ => None,
        }
    }
}

#[derive(Debug)]
enum RegionSpace {
    SystemMemory,
    SystemIO,
    PCIConfig,
    EmbeddedController,
    SMBus,
    SystemCMOS,
    PciBarTarget,
    Ipmi,
    GeneralPurposeIO,
    GenericSerialBus,
    Pcc,
    OemDefined,
}

impl RegionSpace {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        match data[0] {
            0 => Some((Self::SystemMemory, 1)),
            1 => Some((Self::SystemIO, 1)),
            2 => Some((Self::PCIConfig, 1)),
            3 => Some((Self::EmbeddedController, 1)),
            4 => Some((Self::SMBus, 1)),
            5 => Some((Self::SystemCMOS, 1)),
            6 => Some((Self::PciBarTarget, 1)),
            7 => Some((Self::Ipmi, 1)),
            8 => Some((Self::GeneralPurposeIO, 1)),
            9 => Some((Self::GenericSerialBus, 1)),
            10 => Some((Self::Pcc, 1)),
            x if x >= 0x80 => Some((Self::OemDefined, 1)),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct MethodFlags {
    flags: u8,
}

impl MethodFlags {
    pub fn get_arg_count(&self) -> u8 {
        self.flags & 0b111
    }

    pub fn has_serialized(&self) -> bool {
        self.flags & 0b1000 != 0
    }

    pub fn sync_level(&self) -> u8 {
        (self.flags >> 4) & 0b1111
    }
}

#[derive(Debug)]
struct SyncFlags {
    flags: u8,
}

impl SyncFlags {
    pub fn get_sync_level(&self) -> u8 {
        self.flags & 0b1111
    }
}

impl AmlNew for SyncFlags {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data.is_empty() {
            return None;
        }
        Some((Self { flags: data[0] }, 1))
    }
}

#[derive(EnumNewMacro, Debug)]
pub enum NamedObj {
    BankField(Box<DefBankField>),
    CreateBitField(Box<DefCreateBitField>),
    CreateByteField(Box<DefCreateByteField>),
    CreateDWordField(Box<DefCreateDWordField>),
    CreateQWordField(Box<DefCreateQWordField>),
    CreateField(Box<DefCreateField>),
    DataRegion(Box<DefDataRegion>),
    External(Box<DefExternal>),
    Field(Box<DefField>),
    IndexField(Box<DefIndexField>),
    Method(Box<DefFakeMethod>),
    OpRegion(Box<DefOpRegion>),
    PowerRes(Box<DefPowerRes>),
    Processor(Box<DefProcessor>),
    ThermalZone(Box<DefThermalZone>),
    Device(Box<DefDevice>),
    Event(Box<DefEvent>),
    Mutex(Box<DefMutex>),
}

#[derive(Debug)]
pub struct DefBankField {
    name_1: NameString,
    name_2: NameString,
    bank_value: TermArg,
    field_flags: FieldFlags,
    field_list: FieldList,
}

impl AmlNew for DefBankField {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
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

        let (field_list, _skip_field_list) = FieldList::aml_new(&data[skip..(pkg_length.get_length() + 2)]).unwrap();
        skip = 2 + pkg_length.get_length();

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

#[derive(Debug, StructNewMacro)]
#[op_prefix(CREATE_BIT_FIELD_OP)]
pub struct DefCreateBitField {
    source_buf: TermArg,
    bit_index: TermArg,
    name: NameString,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(CREATE_BYTE_FIELD_OP)]
pub struct DefCreateByteField {
    source_buf: TermArg,
    byte_index: TermArg,
    name: NameString,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(CREATE_DWORD_FIELD_OP)]
pub struct DefCreateDWordField {
    source_buf: TermArg,
    byte_index: TermArg,
    name: NameString,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(CREATE_QWORD_FIELD_OP)]
pub struct DefCreateQWordField {
    source_buf: TermArg,
    byte_index: TermArg,
    name: NameString,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(CREATE_WORD_FIELD_OP)]
pub struct DefCreateWordField {
    source_buf: TermArg,
    byte_index: TermArg,
    name: NameString,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(CREATE_FIELD)]
pub struct DefCreateField {
    source_buf: TermArg,
    bit_index: TermArg,
    num_bits: TermArg,
    name: NameString,
}

#[derive(StructNewMacro, Debug)]
#[ext_op_prefix(DATA_REGION_OP)]
pub struct DefDataRegion {
    name: NameString,
    offset: TermArg,
    size: TermArg,
    third_arg: TermArg, //TODO: what is this? first 2 are also made up by copilot
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(EXTERNAL_OP)]
pub struct DefExternal {
    name: NameString,
    object_type: ByteData,
    argument_count: ByteData,
}

#[derive(Debug)]
pub struct DefField {
    name: NameString,
    field_flags: FieldFlags,
    field_list: FieldList,
}

impl AmlNew for DefField {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != FIELD_OP[0] || data[1] != FIELD_OP[1] {
            return None;
        }
        let mut skip = 2;
        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[skip..]);
        skip += skip_pkg_len;
        let (name, skip_name) = NameString::aml_new(&data[skip..]).unwrap();
        skip += skip_name;
        let field_flags = FieldFlags { flags: data[skip] };
        skip += 1;
        let (field_list, _skip_field_list) = FieldList::aml_new(&data[skip..(pkg_length.get_length() + 2)]).unwrap();
        skip = 2 + pkg_length.get_length();
        Some((
            Self {
                name,
                field_flags,
                field_list,
            },
            skip,
        ))
    }
}

#[derive(Debug)]
pub struct DefIndexField {
    name: NameString,
    name_2: NameString,
    field_flags: FieldFlags,
    field_list: FieldList,
}

impl AmlNew for DefIndexField {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != INDEX_FIELD_OP[0] || data[1] != INDEX_FIELD_OP[1] {
            return None;
        }
        let mut skip = 2;
        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[skip..]);
        skip += skip_pkg_len;
        let (name, skip_name) = NameString::aml_new(&data[skip..]).unwrap();
        skip += skip_name;
        let (name_2, skip_name_2) = NameString::aml_new(&data[skip..]).unwrap();
        skip += skip_name_2;
        let field_flags = FieldFlags { flags: data[skip] };
        skip += 1;
        let (field_list, skip_field_list) = FieldList::aml_new(&data[skip..(pkg_length.get_length() + 2)]).unwrap();
        skip += skip_field_list;
        Some((
            Self {
                name,
                name_2,
                field_flags,
                field_list,
            },
            skip,
        ))
    }
}

#[derive(Debug)]
pub struct DefMethod {
    pub method_flags: MethodFlags,
    term_list: TermList,
}

impl DefMethod {
    pub fn aml_new(data: &[u8]) -> Option<(Self, NameString)> {
        if data[0] != METHOD_OP {
            return None;
        }
        let mut skip = 1;
        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[skip..]);
        skip += skip_pkg_len;
        let (name, skip_name) = NameString::aml_new(&data[skip..])?;
        skip += skip_name;
        let method_flags = MethodFlags { flags: data[skip] };
        skip += 1;
        Namespace::push_namespace_string(get_namespace(), name.clone());
        let (term_list, _term_list_skip) = TermList::aml_new(&data[skip..(pkg_length.get_length() + 1)]).unwrap();
        Namespace::pop_namespace(get_namespace());
        let method = DefMethod { method_flags, term_list };
        Some((method, name))
    }

    pub fn get_arg_count(data: &[u8]) -> Option<(NameString, u8, usize)> {
        if data[0] != METHOD_OP {
            return None;
        }
        let mut skip = 1;
        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[skip..]);
        skip += skip_pkg_len;
        let (name, skip_name) = NameString::aml_new(&data[skip..])?;
        if name.is_null() {
            return None;
        }
        skip += skip_name;
        let method_flags = MethodFlags { flags: data[skip] };

        if pkg_length.get_length() + 1 > data.len() {
            return None;
        }
        Some((name, method_flags.get_arg_count(), 1 + pkg_length.get_length()))
    }
}

#[derive(Debug)]
pub struct DefFakeMethod {}

impl AmlNew for DefFakeMethod {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != METHOD_OP {
            return None;
        }
        let mut skip = 1;
        let (pkg_length, _skip_pkg_len) = super::package::PkgLength::new(&data[skip..]);
        let method = DefMethod::aml_new(data).unwrap();
        namespace::get_namespace().add_method(&method.1, method.0);

        skip = 1 + pkg_length.get_length();
        Some((Self {}, skip))
    }
}

#[derive(Debug, StructNewMacro)]
#[ext_op_prefix(OP_REGION_OP)]
pub struct DefOpRegion {
    name: NameString,
    region_space: RegionSpace,
    offset: TermArg,
    length: TermArg,
}

#[derive(Debug)]
pub struct DefPowerRes {
    name: NameString,
    system_level: u8,
    resource_order: u16,
    term_list: TermList,
}

impl DefPowerRes {
    pub fn check_namespace(data: &[u8]) -> Option<(NameString, usize)> {
        if data[0] != POWER_RES_OP[0] || data[1] != POWER_RES_OP[1] {
            return None;
        }
        let mut skip = 2;
        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[skip..]);
        skip += skip_pkg_len;
        let (name, _skip_name) = NameString::aml_new(&data[skip..])?;
        skip = 2 + pkg_length.get_length();
        Some((name, skip))
    }
}

impl AmlNew for DefPowerRes {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != POWER_RES_OP[0] || data[1] != POWER_RES_OP[1] {
            return None;
        }
        let mut skip = 2;
        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[skip..]);
        skip += skip_pkg_len;
        let (name, skip_name) = NameString::aml_new(&data[skip..]).unwrap();
        skip += skip_name;
        let system_level = data[skip];
        skip += 1;
        let resource_order = u16::from_le_bytes([data[skip], data[skip + 1]]);
        skip += 2;
        Namespace::push_namespace_string(get_namespace(), name.clone());
        let (term_list, term_list_skip) = TermList::aml_new(&data[skip..(2 + pkg_length.get_length())]).unwrap();
        Namespace::pop_namespace(get_namespace());
        skip += term_list_skip;
        Some((
            Self {
                name,
                system_level,
                resource_order,
                term_list,
            },
            skip,
        ))
    }
}

#[derive(Debug)]
pub struct DefProcessor {
    processor_id: ByteData,
    pblk_address: DWordData,
    pblk_length: ByteData,
    term_list: TermList,
}

impl DefProcessor {
    pub fn check_namespace(data: &[u8]) -> Option<(NameString, usize)> {
        if data[0] != PROCESSOR_OP[0] || data[1] != PROCESSOR_OP[1] {
            return None;
        }
        let mut skip = 2;
        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[skip..]);
        skip += skip_pkg_len;
        let (name, _skip_name) = NameString::aml_new(&data[skip..])?;
        skip = 2 + pkg_length.get_length();
        Some((name, skip))
    }
}

impl AmlNew for DefProcessor {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != PROCESSOR_OP[0] || data[1] != PROCESSOR_OP[1] {
            return None;
        }
        let mut skip = 2;
        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[skip..]);
        skip += skip_pkg_len;
        let (name, skip_name) = NameString::aml_new(&data[skip..]).unwrap();
        skip += skip_name;
        let (processor_id, skip_id) = ByteData::aml_new(&data[skip..]).unwrap();
        skip += skip_id;
        let (pblk_address, skip_pblk_address) = DWordData::aml_new(&data[skip..]).unwrap();
        skip += skip_pblk_address;
        let (pblk_length, skip_pblk_length) = ByteData::aml_new(&data[skip..]).unwrap();
        skip += skip_pblk_length;

        Namespace::push_namespace_string(get_namespace(), name.clone());
        let (term_list, term_list_skip) = TermList::aml_new(&data[skip..(2 + pkg_length.get_length())]).unwrap();
        Namespace::pop_namespace(get_namespace());
        skip += term_list_skip;

        Some((
            Self {
                processor_id,
                pblk_address,
                pblk_length,
                term_list,
            },
            skip,
        ))
    }
}

#[derive(Debug)]
pub struct DefThermalZone {
    name: NameString,
    term_list: TermList,
}

impl DefThermalZone {
    pub fn check_namespace(data: &[u8]) -> Option<(NameString, usize)> {
        if data[0] != THERMAL_ZONE_OP[0] || data[1] != THERMAL_ZONE_OP[1] {
            return None;
        }
        let mut skip = 2;
        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[skip..]);
        skip += skip_pkg_len;
        let (name, _skip_name) = NameString::aml_new(&data[skip..])?;
        skip = 2 + pkg_length.get_length();
        Some((name, skip))
    }
}

impl AmlNew for DefThermalZone {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != THERMAL_ZONE_OP[0] || data[1] != THERMAL_ZONE_OP[1] {
            return None;
        }
        let mut skip = 2;
        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[skip..]);
        skip += skip_pkg_len;
        let (name, skip_name) = NameString::aml_new(&data[skip..]).unwrap();
        skip += skip_name;
        Namespace::push_namespace_string(get_namespace(), name.clone());
        let (term_list, term_list_skip) = TermList::aml_new(&data[skip..(2 + pkg_length.get_length())]).unwrap();
        Namespace::pop_namespace(get_namespace());
        skip += term_list_skip;
        Some((Self { name, term_list }, skip))
    }
}

#[derive(Debug)]
pub struct DefDevice {
    name: NameString,
    term_list: TermList,
}

impl DefDevice {
    pub fn check_namespace(data: &[u8]) -> Option<(NameString, usize)> {
        if data[0] != DEVICE_OP[0] || data[1] != DEVICE_OP[1] {
            return None;
        }
        let mut skip = 2;
        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[skip..]);
        skip += skip_pkg_len;
        let (name, _skip_name) = NameString::aml_new(&data[skip..])?;
        skip = 2 + pkg_length.get_length();
        Some((name, skip))
    }
}

impl AmlNew for DefDevice {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != DEVICE_OP[0] || data[1] != DEVICE_OP[1] {
            return None;
        }
        let mut skip = 2;
        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[skip..]);
        skip += skip_pkg_len;
        let (name, skip_name) = NameString::aml_new(&data[skip..]).unwrap();
        skip += skip_name;
        Namespace::push_namespace_string(get_namespace(), name.clone());
        let (term_list, term_list_skip) = TermList::aml_new(&data[skip..(2 + pkg_length.get_length())]).unwrap();
        Namespace::pop_namespace(get_namespace());
        skip += term_list_skip;
        Some((Self { name, term_list }, skip))
    }
}

#[derive(Debug, StructNewMacro)]
#[ext_op_prefix(EVENT_OP)]
pub struct DefEvent {
    name: NameString,
}

#[derive(Debug, StructNewMacro)]
#[ext_op_prefix(MUTEX_OP)]
pub struct DefMutex {
    name: NameString,
    sync_flafs: SyncFlags,
}

#[derive(Debug)]
struct FieldList {
    fields: Vec<FieldElement>,
}

impl FieldList {
    //reads the whole data stream
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        let mut skip = 0;
        let mut fields = Vec::new();
        while skip < data.len() {
            if let Some((field, field_skip)) = FieldElement::aml_new(&data[skip..]) {
                fields.push(field);
                skip += field_skip;
            } else {
                panic!("FieldList::new: could not read field");
            }
        }
        //sanity check
        if skip != data.len() {
            panic!(
                "FieldList::new: did not read all data: {} != {}\n{:x?}",
                skip,
                data.len(),
                data
            );
        }
        Some((Self { fields }, skip))
    }
}

#[allow(clippy::enum_variant_names)] //they are all fields
#[derive(Debug, EnumNewMacro)]
enum FieldElement {
    NamedField(NamedField),
    ReservedField(ReservedField),
    AccessField(AccessField),
    ExtendedAccessField(ExtendedAccessField),
    ConnectField(ConnectField),
}

#[derive(Debug)]
struct NamedField {
    name_seg: super::name_objects::NameSeg,
    len: usize,
}

impl AmlNew for NamedField {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        let (name_seg, mut skip) = super::name_objects::NameSeg::aml_new(data)?;
        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[skip..]);
        skip += skip_pkg_len;
        Some((
            Self {
                name_seg,
                len: pkg_length.get_length(),
            },
            skip,
        ))
    }
}

#[derive(Debug)]
struct ReservedField {
    len: usize,
}

impl AmlNew for ReservedField {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != RESERVED_FIELD_OP {
            return None;
        }
        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[1..]);
        let skip = 1 + skip_pkg_len;
        Some((
            Self {
                len: pkg_length.get_length(),
            },
            skip,
        ))
    }
}

#[derive(Debug)]
struct AccessField {
    access_type: AccessType,
    access_attrib: AccessAttribInner,
}

impl AmlNew for AccessField {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != ACCESS_FIELD_OP {
            return None;
        }
        let access_type = AccessType { flags: data[1] };
        let access_attrib = AccessAttribInner::aml_new(&data[2..]).unwrap().0;
        Some((
            Self {
                access_type,
                access_attrib,
            },
            3,
        ))
    }
}

#[derive(Debug)]
struct ExtendedAccessField {
    access_type: AccessType,
    extended_access_attrib: AccessAttrib,
    access_length: ByteData,
}

impl AmlNew for ExtendedAccessField {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != EXTENDED_ACCESS_FIELD_OP {
            return None;
        }
        let access_type = AccessType { flags: data[1] };
        let mut skip = 2;
        let (extended_access_attrib, skip_add) = AccessAttrib::aml_new(&data[2..]).unwrap();
        skip += skip_add;
        let access_length = ByteData::aml_new(&data[skip..]).unwrap().0;
        skip += 1;
        Some((
            Self {
                access_type,
                extended_access_attrib,
                access_length,
            },
            skip,
        ))
    }
}

#[derive(Debug)]
enum ConnectField {
    NameString(NameString),
    //TODO: buffer data?? it's specified in ASL
}

impl AmlNew for ConnectField {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != CONNECT_FIELD_OP {
            todo!("buffer data???");
            //return None;
        }
        let name_string = NameString::aml_new(data)?;
        Some((Self::NameString(name_string.0), name_string.1))
    }
}
