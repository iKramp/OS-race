use std::boxed::Box;
use std::Vec;

use super::data_object::{WordData, EXT_OP_PREFIX};
use super::name_objects::{NameString, SimpleName, SuperName, Target};
use super::package::PkgLength;
use super::term_objects::{MethodInvocation, TermArg};

use macros::*;
use traits::*;

const ACQUIRE_OP: [u8; 2] = [EXT_OP_PREFIX, 0x23];
const ADD_OP: u8 = 0x72;
const AND_OP: u8 = 0x7B;
pub const BUFFER_OP: u8 = 0x11;
const CONCAT_OP: u8 = 0x73;
const CONCAT_RES_OP: u8 = 0x84;
const COND_REF_OF_OP: [u8; 2] = [EXT_OP_PREFIX, 0x12];
const COPY_OBJECT_OP: u8 = 0x9D;
const DECREMENT_OP: u8 = 0x76;
const DEREF_OF_OP: u8 = 0x83;
const DIVIDE_OP: u8 = 0x78;
const FIND_SET_LEFT_BIT_OP: u8 = 0x81;
const FIND_SET_RIGHT_BIT_OP: u8 = 0x82;
const FROM_BCD_OP: [u8; 2] = [EXT_OP_PREFIX, 0x28];
const INCREMENT_OP: u8 = 0x75;
const INDEX_OP: u8 = 0x88;
const L_AND_OP: u8 = 0x90;
const L_EQUAL_OP: u8 = 0x93;
const L_GREATER_OP: u8 = 0x94;
const L_LESS_OP: u8 = 0x95;
const L_NOT_OP: u8 = 0x92;
const LOAD_OP: [u8; 2] = [EXT_OP_PREFIX, 0x20];
const LOAD_TABLE_OP: [u8; 2] = [EXT_OP_PREFIX, 0x1F];
const L_OR_OP: u8 = 0x91;
const MATCH_OP: u8 = 0x89;
const MID_OP: u8 = 0x9E;
const MOD_OP: u8 = 0x85;
const MULTIPLY_OP: u8 = 0x77;
const NAND_OP: u8 = 0x7C;
const NOR_OP: u8 = 0x7E;
const NOT_OP: u8 = 0x80;
const OBJECT_TYPE_OP: u8 = 0x8E;
const OR_OP: u8 = 0x7D;
const PACKAGE_OP: u8 = 0x12;
const VAR_PACKAGE_OP: u8 = 0x13;
const REF_OF_OP: u8 = 0x71;
const SHIFT_LEFT_OP: u8 = 0x79;
const SHIFT_RIGHT_OP: u8 = 0x7A;
const SIZE_OF_OP: u8 = 0x87;
const STORE_OP: u8 = 0x70;
const SUBTRACT_OP: u8 = 0x74;
const TIMER_OP: [u8; 2] = [0x5B, 0x33];
const TO_BCD_OP: [u8; 2] = [EXT_OP_PREFIX, 0x29];
const TO_BUFFER_OP: u8 = 0x96;
const TO_DEC_STRING_OP: u8 = 0x97;
const TO_HEX_STRING_OP: u8 = 0x98;
const TO_INTEGER_OP: u8 = 0x99;
const TO_STRING_OP: u8 = 0x9C;
const WAIT_OP: [u8; 2] = [0x5B, 0x25];
const XOR_OP: u8 = 0x7F;
const DEBUG_OP: [u8; 2] = [EXT_OP_PREFIX, 0x31];

#[derive(Debug)]
enum MatchOpcode {
    Mtr = 0x00,
    MatchEqual = 0x01,
    MatchLessEqual = 0x02,
    MatchLess = 0x03,
    MatchGreaterEqual = 0x04,
    MatchGreater = 0x05,
}

impl MatchOpcode {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        match data[0] {
            x if x == Self::Mtr as u8 => Some((Self::Mtr, 1)),
            x if x == Self::MatchEqual as u8 => Some((Self::MatchEqual, 1)),
            x if x == Self::MatchLessEqual as u8 => Some((Self::MatchLessEqual, 1)),
            x if x == Self::MatchLess as u8 => Some((Self::MatchLess, 1)),
            x if x == Self::MatchGreaterEqual as u8 => Some((Self::MatchGreaterEqual, 1)),
            x if x == Self::MatchGreater as u8 => Some((Self::MatchGreater, 1)),
            _ => None,
        }
    }
}

#[derive(Debug, EnumNewMacro)]
pub enum ExpressionOpcode {
    Acquire(Box<DefAcquire>),
    Add(Box<DefAdd>),
    And(Box<DefAnd>),
    Buffer(Box<DefBuffer>),
    Concat(Box<DefConcat>),
    ConcatRes(Box<DefConcatRes>),
    CondRefOf(Box<DefCondRefOf>),
    CopyObject(Box<DefCopyObject>),
    Decrement(Box<DefDecrement>),
    DerefOf(Box<DefDerefOf>),
    Divide(Box<DefDivide>),
    FindSetLeftBit(Box<DefFindSetLeftBit>),
    FindSetRightBit(Box<DefFindSetRightBit>),
    FromBcd(Box<DefFromBcd>),
    Increment(Box<DefIncrement>),
    Index(Box<DefIndex>),
    LAnd(Box<DefLAnd>),
    LEqual(Box<DefLEqual>),
    LGreater(Box<DefLGreater>),
    LLess(Box<DefLLess>),
    Mid(Box<DefMid>),
    LNot(Box<DefLNot>),
    LoadTable(Box<DefLoadTable>),
    Load(Box<DefLoad>), //not in specification?
    LOr(Box<DefLOr>),
    Match(Box<DefMatch>),
    Mod(Box<DefMod>),
    Multiply(Box<DefMultiply>),
    Nand(Box<DefNand>),
    Nor(Box<DefNor>),
    Not(Box<DefNot>),
    ObjectType(Box<DefObjectType>),
    Or(Box<DefOr>),
    Package(Box<DefPackage>),
    VarPackage(Box<DefVarPackage>),
    RefOf(Box<DefRefOf>),
    ShiftLeft(Box<DefShiftLeft>),
    ShiftRight(Box<DefShiftRight>),
    SizeOf(Box<DefSizeOf>),
    Store(Box<DefStore>),
    Subtract(Box<DefSubtract>),
    Timer(Box<DefTimer>),
    ToBcd(Box<DefToBcd>),
    ToBuffer(Box<DefToBuffer>),
    ToDecString(Box<DefToDecString>),
    ToHexString(Box<DefToHexString>),
    ToInteger(Box<DefToInteger>),
    ToString(Box<DefToString>),
    Wait(Box<DefWait>),
    Xor(Box<DefXor>),
    MethodInvocation(Box<MethodInvocation>),
    NameObjOrField(Box<NameString>),
}

#[derive(Debug, StructNewMacro)]
#[ext_op_prefix(ACQUIRE_OP)]
pub struct DefAcquire {
    mutex_object: SuperName,
    timeout: WordData,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(ADD_OP)]
pub struct DefAdd {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(AND_OP)]
pub struct DefAnd {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

#[derive(Debug)]
pub struct DefBuffer {
    buffer_length: TermArg,
    byte_list: Vec<u8>,
}

impl AmlNew for DefBuffer {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != BUFFER_OP {
            return None;
        }

        let mut skip = 1;
        let (pkg_length, pkg_skip) = PkgLength::new(&data[skip..]);
        skip += pkg_skip;
        let (buffer_length, buffer_skip) = TermArg::aml_new(&data[skip..]).unwrap();
        skip += buffer_skip;
        let byte_list = &data[skip..1 + pkg_length.get_length()];
        let mut byte_vec = Vec::new();
        for byte in byte_list {
            byte_vec.push(*byte);
        }
        skip = 1 + pkg_length.get_length();
        Some((
            Self {
                buffer_length,
                byte_list: byte_vec,
            },
            skip,
        ))
    }
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(CONCAT_OP)]
pub struct DefConcat {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(CONCAT_RES_OP)]
pub struct DefConcatRes {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

#[derive(Debug, StructNewMacro)]
#[ext_op_prefix(COND_REF_OF_OP)]
pub struct DefCondRefOf {
    name: SuperName,
    target: Target,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(COPY_OBJECT_OP)]
pub struct DefCopyObject {
    source: TermArg,
    destination: SimpleName,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(DECREMENT_OP)]
pub struct DefDecrement {
    operand: SuperName,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(DEREF_OF_OP)]
pub struct DefDerefOf {
    operator: TermArg,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(DIVIDE_OP)]
pub struct DefDivide {
    dividend: TermArg,
    divisor: TermArg,
    remainder: Target,
    quotient: Target,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(FIND_SET_LEFT_BIT_OP)]
pub struct DefFindSetLeftBit {
    operand: TermArg,
    target: Target,
}

#[derive(StructNewMacro, Debug)]
#[op_prefix(FIND_SET_RIGHT_BIT_OP)]
pub struct DefFindSetRightBit {
    operand: TermArg,
    target: Target,
}

#[derive(Debug, StructNewMacro)]
#[ext_op_prefix(FROM_BCD_OP)]
pub struct DefFromBcd {
    operand: TermArg,
    target: Target,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(INCREMENT_OP)]
pub struct DefIncrement {
    operand: SuperName,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(INDEX_OP)]
pub struct DefIndex {
    operand: TermArg,
    index: TermArg,
    target: Target,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(L_AND_OP)]
pub struct DefLAnd {
    operand1: TermArg,
    operand2: TermArg,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(L_EQUAL_OP)]
pub struct DefLEqual {
    operand1: TermArg,
    operand2: TermArg,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(L_GREATER_OP)]
pub struct DefLGreater {
    operand1: TermArg,
    operand2: TermArg,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(L_LESS_OP)]
pub struct DefLLess {
    operand1: TermArg,
    operand2: TermArg,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(L_NOT_OP)]
pub struct DefLNot {
    operand: TermArg,
}

#[derive(StructNewMacro, Debug)]
#[ext_op_prefix(LOAD_OP)]
pub struct DefLoad {
    name: super::name_objects::NameString,
    target: Target,
}

#[derive(Debug, StructNewMacro)]
#[ext_op_prefix(LOAD_TABLE_OP)]
pub struct DefLoadTable {
    //TODO: check documentation
    arg0: TermArg,
    arg1: TermArg,
    arg2: TermArg,
    arg3: TermArg,
    arg4: TermArg,
    arg5: TermArg,
}

#[derive(StructNewMacro, Debug)]
#[op_prefix(L_OR_OP)]
pub struct DefLOr {
    operand1: TermArg,
    operand2: TermArg,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(MATCH_OP)]
pub struct DefMatch {
    search_pkg: TermArg,
    operand1: TermArg,
    operator: MatchOpcode,
    operand2: TermArg,
    start_index: TermArg,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(MID_OP)]
pub struct DefMid {
    arg1: TermArg,
    arg2: TermArg,
    arg3: TermArg,
    target: Target,
}

#[derive(StructNewMacro, Debug)]
#[op_prefix(MOD_OP)]
pub struct DefMod {
    dividend: TermArg,
    divisor: TermArg,
    remainder: Target,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(MULTIPLY_OP)]
pub struct DefMultiply {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(NAND_OP)]
pub struct DefNand {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(NOR_OP)]
pub struct DefNor {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

#[derive(StructNewMacro, Debug)]
#[op_prefix(NOT_OP)]
pub struct DefNot {
    operand: TermArg,
    target: Target,
}

#[derive(StructNewMacro, Debug)]
#[op_prefix(OBJECT_TYPE_OP)]
pub struct DefObjectType {
    object: DefObjectTypeInner,
}

#[derive(EnumNewMacro, Debug)]
enum DefObjectTypeInner {
    SimpleName(SimpleName),
    DebugObj(DebugObj),
    DefRefOf(DefRefOf),
    DefDerefOf(DefDerefOf),
    DefIndex(DefIndex),
}

#[derive(Debug, StructNewMacro)]
#[ext_op_prefix(DEBUG_OP)]
pub struct DebugObj;

#[derive(StructNewMacro, Debug)]
#[op_prefix(OR_OP)]
pub struct DefOr {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

#[derive(Debug)]
pub struct DefPackage {
    elemtn_list: PackageElementList,
}

impl AmlNew for DefPackage {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != PACKAGE_OP {
            return None;
        }
        let mut skip = 1;
        let (pkg_length, skip_pkg_len) = PkgLength::new(&data[1..]);
        skip += skip_pkg_len;
        let num_elements = data[skip];
        skip += 1;
        let (mut element_list, skip_element_list) = PackageElementList::aml_new(&data[skip..(1 + pkg_length.get_length())])?;
        element_list.elements.reserve_exact(num_elements as usize - element_list.elements.len());
        skip += skip_element_list;
        debug_assert!(skip == pkg_length.get_length() + 1);
        Some((
            Self {
                elemtn_list: element_list,
            },
            skip,
        ))
    }
}

#[derive(Debug)]
pub struct PackageElementList {
    pub elements: Vec<PackageElement>,
}

impl PackageElementList {
    //reads whole buffer
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        let mut elements = Vec::new();
        let mut skip = 0;
        while skip < data.len() {
            let (element, skip_element) = PackageElement::aml_new(&data[skip..])?;
            elements.push(element);
            skip += skip_element;
        }
        //sanity check
        debug_assert_eq!(skip, data.len());
        Some((Self { elements }, skip))
    }
}

#[derive(EnumNewMacro, Debug)]
pub enum PackageElement {
    DataRefObject(Box<super::data_object::DataRefObject>),
    NameString(super::name_objects::NameString),
}

#[derive(Debug)]
pub struct DefVarPackage {
    num_elements: TermArg,
    element_list: PackageElementList,
}

impl AmlNew for DefVarPackage {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != VAR_PACKAGE_OP {
            return None;
        }
        let mut skip = 1;
        let (pkg_length, skip_pkg_len) = PkgLength::new(&data[1..]);
        skip += skip_pkg_len;
        let (num_elements, skip_num_elements) = TermArg::aml_new(&data[skip..]).unwrap();
        skip += skip_num_elements;
        let (element_list, element_list_skip) = PackageElementList::aml_new(&data[skip..1 + pkg_length.get_length()]).unwrap();
        skip += element_list_skip;
        Some((
            Self {
                num_elements,
                element_list,
            },
            skip,
        ))
    }
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(REF_OF_OP)]
pub struct DefRefOf {
    operand: SuperName,
}

#[derive(StructNewMacro, Debug)]
#[op_prefix(SHIFT_LEFT_OP)]
pub struct DefShiftLeft {
    operand: TermArg,
    count: TermArg,
    target: Target,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(SHIFT_RIGHT_OP)]
pub struct DefShiftRight {
    operand: TermArg,
    count: TermArg,
    target: Target,
}

#[derive(StructNewMacro, Debug)]
#[op_prefix(SIZE_OF_OP)]
pub struct DefSizeOf {
    operand: SuperName,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(STORE_OP)]
pub struct DefStore {
    value: TermArg,
    target: SuperName,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(SUBTRACT_OP)]
pub struct DefSubtract {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

#[derive(StructNewMacro, Debug)]
#[ext_op_prefix(TIMER_OP)]
pub struct DefTimer; //no fields

#[derive(StructNewMacro, Debug)]
#[ext_op_prefix(TO_BCD_OP)]
pub struct DefToBcd {
    operand: TermArg,
    target: Target,
}

#[derive(StructNewMacro, Debug)]
#[op_prefix(TO_BUFFER_OP)]
pub struct DefToBuffer {
    operand: TermArg,
    target: Target,
}

#[derive(StructNewMacro, Debug)]
#[op_prefix(TO_DEC_STRING_OP)]
pub struct DefToDecString {
    operand: TermArg,
    target: Target,
}

#[derive(StructNewMacro, Debug)]
#[op_prefix(TO_HEX_STRING_OP)]
pub struct DefToHexString {
    operand: TermArg,
    target: Target,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(TO_INTEGER_OP)]
pub struct DefToInteger {
    operand: TermArg,
    target: Target,
}

#[derive(StructNewMacro, Debug)]
#[op_prefix(TO_STRING_OP)]
pub struct DefToString {
    operand: TermArg,
    length_arg: TermArg,
    target: Target,
}

#[derive(StructNewMacro, Debug)]
#[ext_op_prefix(WAIT_OP)]
pub struct DefWait {
    event_object: SuperName,
    operand: TermArg,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(XOR_OP)]
pub struct DefXor {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}
