use std::Vec;

use super::data_object::EXT_OP_PREFIX;
use super::name_objects::{SimpleName, SuperName, Target};
use super::package::PkgLength;
use super::term_objects::{MethodInvocation, TermArg};

use functions::*;
use macros::*;

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

enum MatchOpcode {
    MTR = 0x00,
    MatchEqual = 0x01,
    MatchLessEqual = 0x02,
    MatchLess = 0x03,
    MatchGreaterEqual = 0x04,
    MatchGreater = 0x05,
}

impl MatchOpcode {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        match data[0] {
            x if x == Self::MTR as u8 => Some((Self::MTR, 1)),
            x if x == Self::MatchEqual as u8 => Some((Self::MatchEqual, 1)),
            x if x == Self::MatchLessEqual as u8 => Some((Self::MatchLessEqual, 1)),
            x if x == Self::MatchLess as u8 => Some((Self::MatchLess, 1)),
            x if x == Self::MatchGreaterEqual as u8 => Some((Self::MatchGreaterEqual, 1)),
            x if x == Self::MatchGreater as u8 => Some((Self::MatchGreater, 1)),
            _ => None,
        }
    }
}

#[derive(EnumNewMacro)]
pub enum ExpressionOpcode {
    Acquire(DefAcquire),
    Add(DefAdd),
    And(DefAnd),
    Buffer(DefBuffer),
    Concat(DefConcat),
    ConcatRes(DefConcatRes),
    CondRefOf(DefCondRefOf),
    CopyObject(DefCopyObject),
    Decrement(DefDecrement),
    DerefOf(DefDerefOf),
    Divide(DefDivide),
    FindSetLeftBit(DefFindSetLeftBit),
    FindSetRightBit(DefFindSetRightBit),
    FromBcd(DefFromBcd),
    Increment(DefIncrement),
    Index(DefIndex),
    LAnd(DefLAnd),
    LEqual(DefLEqual),
    LGreater(DefLGreater),
    LLess(DefLLess),
    Mid(DefMid),
    LNot(DefLNot),
    LoadTable(DefLoadTable),
    Load(DefLoad), //not in specification?
    LOr(DefLOr),
    Match(DefMatch),
    Mod(DefMod),
    Multiply(DefMultiply),
    Nand(DefNand),
    Nor(DefNor),
    Not(DefNot),
    ObjectType(DefObjectType),
    Or(DefOr),
    Package(DefPackage),
    VarPackage(DefVarPackage),
    RefOf(DefRefOf),
    ShiftLeft(DefShiftLeft),
    ShiftRight(DefShiftRight),
    SizeOf(DefSizeOf),
    Store(DefStore),
    Subtract(DefSubtract),
    Timer(DefTimer),
    ToBcd(DefToBcd),
    ToBuffer(DefToBuffer),
    ToDecString(DefToDecString),
    ToHexString(DefToHexString),
    ToInteger(DefToInteger),
    ToString(DefToString),
    Wait(DefWait),
    Xor(DefXor),
    MethodInvocation(MethodInvocation),
}

struct DefAcquire {
    //TODO: MutexObject
    timeout: u16,
}

impl DefAcquire {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        //sanity check
        debug_assert_eq!(data[0], ACQUIRE_OP[0]);
        debug_assert_eq!(data[1], ACQUIRE_OP[1]);
        let timeout = u16::from_le_bytes([data[2], data[3]]);
        return Some((Self { timeout }, 4));
    }
}

#[derive(StructNewMacro)]
#[op_prefix(ADD_OP)]
struct DefAdd {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(AND_OP)]
struct DefAnd {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

pub struct DefBuffer {
    pkg_length: super::package::PkgLength,
    byte_list: Vec<u8>,
}

impl DefBuffer {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        debug_assert_eq!(data[0], BUFFER_OP); //sanity check
        todo!();
    }
}

#[derive(StructNewMacro)]
#[op_prefix(CONCAT_OP)]
struct DefConcat {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(CONCAT_RES_OP)]
struct DefConcatRes {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[ext_op_prefix(COND_REF_OF_OP)]
struct DefCondRefOf {
    name: SuperName,
    target: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(COPY_OBJECT_OP)]
struct DefCopyObject {
    source: TermArg,
    destination: SimpleName,
}

#[derive(StructNewMacro)]
#[op_prefix(DECREMENT_OP)]
struct DefDecrement {
    operand: SuperName,
}

pub trait Dereferencable {
    //TODO:
}

#[derive(StructNewMacro)]
#[op_prefix(DEREF_OF_OP)]
struct DefDerefOf {
    operator: TermArg,
}

#[derive(StructNewMacro)]
#[op_prefix(DIVIDE_OP)]
struct DefDivide {
    dividend: TermArg,
    divisor: TermArg,
    remainder: Target,
    quotient: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(FIND_SET_LEFT_BIT_OP)]
struct DefFindSetLeftBit {
    operand: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(FIND_SET_RIGHT_BIT_OP)]
struct DefFindSetRightBit {
    operand: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[ext_op_prefix(FROM_BCD_OP)]
struct DefFromBcd {
    operand: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(INCREMENT_OP)]
struct DefIncrement {
    operand: SuperName,
}

#[derive(StructNewMacro)]
#[op_prefix(INDEX_OP)]
struct DefIndex {
    operand: TermArg,
    index: TermArg,
}

#[derive(StructNewMacro)]
#[op_prefix(L_AND_OP)]
struct DefLAnd {
    operand1: TermArg,
    operand2: TermArg,
}

#[derive(StructNewMacro)]
#[op_prefix(L_EQUAL_OP)]
struct DefLEqual {
    operand1: TermArg,
    operand2: TermArg,
}

#[derive(StructNewMacro)]
#[op_prefix(L_GREATER_OP)]
struct DefLGreater {
    operand1: TermArg,
    operand2: TermArg,
}

#[derive(StructNewMacro)]
#[op_prefix(L_LESS_OP)]
struct DefLLess {
    operand1: TermArg,
    operand2: TermArg,
}

#[derive(StructNewMacro)]
#[op_prefix(L_NOT_OP)]
struct DefLNot {
    operand: TermArg,
}

#[derive(StructNewMacro)]
#[ext_op_prefix(LOAD_OP)]
struct DefLoad {
    name: super::name_objects::NameString,
    target: Target,
}

#[derive(StructNewMacro)]
#[ext_op_prefix(LOAD_TABLE_OP)]
struct DefLoadTable {
    //TODO: check documentation
    arg0: TermArg,
    arg1: TermArg,
    arg2: TermArg,
    arg3: TermArg,
    arg4: TermArg,
    arg5: TermArg,
}

#[derive(StructNewMacro)]
#[op_prefix(L_OR_OP)]
struct DefLOr {
    operand1: TermArg,
    operand2: TermArg,
}

#[derive(StructNewMacro)]
#[op_prefix(MATCH_OP)]
struct DefMatch {
    search_pkg: TermArg,
    operand1: TermArg,
    operator: MatchOpcode,
    operand2: TermArg,
    start_index: TermArg,
}

#[derive(StructNewMacro)]
#[op_prefix(MID_OP)]
struct DefMid {
    arg1: TermArg,
    arg2: TermArg,
    arg3: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(MOD_OP)]
struct DefMod {
    dividend: TermArg,
    divisor: TermArg,
    remainder: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(MULTIPLY_OP)]
struct DefMultiply {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(NAND_OP)]
struct DefNand {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(NOR_OP)]
struct DefNor {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(NOT_OP)]
struct DefNot {
    operand: TermArg,
    target: Target,
}

struct DefObjectType {
    //TODO:
}

impl DefObjectType {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        //sanity check
        debug_assert_eq!(data[0], OBJECT_TYPE_OP);
        todo!();
    }
}

#[derive(StructNewMacro)]
#[op_prefix(OR_OP)]
struct DefOr {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

pub struct DefPackage {
    elemtn_list: PackageElementList,
}

impl DefPackage {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        //sanity check
        debug_assert_eq!(data[0], PACKAGE_OP);
        let (pkg_length, skip_pkg_len) = PkgLength::new(&data[1..]);
        let num_elements = data[skip_pkg_len + 1];
        let (element_list, skip_element_list) =
            PackageElementList::new_with_len(&data[skip_pkg_len + 2..], num_elements as usize)?;
        debug_assert!(skip_element_list + skip_pkg_len + 2 == pkg_length.get_length() as usize);
        return Some((
            Self {
                elemtn_list: element_list,
            },
            pkg_length.get_length() as usize + 1,
        ));
    }
}

struct PackageElementList {
    elements: Vec<PackageElement>,
}

impl PackageElementList {
    pub fn new_with_len(data: &[u8], num_items: usize) -> Option<(Self, usize)> {
        let mut elements = Vec::new();
        let mut skip = 0;
        for _ in 0..num_items {
            let (element, skip_element) = PackageElement::aml_new(&data[skip..])?;
            elements.push(element);
            skip += skip_element;
        }
        return Some((Self { elements }, skip));
    }

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
        return Some((Self { elements }, skip));
    }
}

#[derive(EnumNewMacro)]
enum PackageElement {
    DataRefObject(super::data_object::DataRefObject),
    NameString(super::name_objects::NameString),
}

pub struct DefVarPackage {
    num_elements: TermArg,
    element_list: PackageElementList,
}

impl DefVarPackage {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        //sanity check
        debug_assert_eq!(data[0], PACKAGE_OP);
        let (pkg_length, skip_pkg_len) = PkgLength::new(&data[1..]);
        let (num_elements_arg, skip_num_elements_arg) = TermArg::aml_new(&data[skip_pkg_len + 1..]).unwrap();
        let (element_list, skip) =
            PackageElementList::aml_new(&data[skip_pkg_len + 1 + skip_num_elements_arg..pkg_length.get_length() as usize + 1])?;
        return Some((
            Self {
                element_list,
                num_elements: num_elements_arg,
            },
            skip,
        ));
    }
}

#[derive(StructNewMacro)]
#[op_prefix(REF_OF_OP)]
struct DefRefOf {
    operand: SuperName,
}

#[derive(StructNewMacro)]
#[op_prefix(SHIFT_LEFT_OP)]
struct DefShiftLeft {
    operand: TermArg,
    count: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(SHIFT_RIGHT_OP)]
struct DefShiftRight {
    operand: TermArg,
    count: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(SIZE_OF_OP)]
struct DefSizeOf {
    operand: SuperName,
}

#[derive(StructNewMacro)]
#[op_prefix(STORE_OP)]
struct DefStore {
    value: TermArg,
    target: SuperName,
}

#[derive(StructNewMacro)]
#[op_prefix(SUBTRACT_OP)]
struct DefSubtract {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[ext_op_prefix(TIMER_OP)]
struct DefTimer; //no fields

#[derive(StructNewMacro)]
#[ext_op_prefix(TO_BCD_OP)]
struct DefToBcd {
    operand: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(TO_BUFFER_OP)]
struct DefToBuffer {
    operand: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(TO_DEC_STRING_OP)]
struct DefToDecString {
    operand: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(TO_HEX_STRING_OP)]
struct DefToHexString {
    operand: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(TO_INTEGER_OP)]
struct DefToInteger {
    operand: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[op_prefix(TO_STRING_OP)]
struct DefToString {
    operand: TermArg,
    length_arg: TermArg,
    target: Target,
}

#[derive(StructNewMacro)]
#[ext_op_prefix(WAIT_OP)]
struct DefWait {
    //TODO:
    //event
    operand: TermArg,
}

#[derive(StructNewMacro)]
#[op_prefix(XOR_OP)]
struct DefXor {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}
