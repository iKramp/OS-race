use std::{Box, Vec};

use super::data_object::{ComputationalData, EXT_OP_PREFIX};
use super::name_objects::{SimpleName, SuperName, Target};
use super::package::PkgLength;
use super::term_objects::{MethodInvocation, TermArg};
use super::Integer;

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
const TO_DEC_STRING_OP: u8 = 0x98;
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
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            x if x == Self::MTR as u8 => Some(Self::MTR),
            x if x == Self::MatchEqual as u8 => Some(Self::MatchEqual),
            x if x == Self::MatchLessEqual as u8 => Some(Self::MatchLessEqual),
            x if x == Self::MatchLess as u8 => Some(Self::MatchLess),
            x if x == Self::MatchGreaterEqual as u8 => Some(Self::MatchGreaterEqual),
            x if x == Self::MatchGreater as u8 => Some(Self::MatchGreater),
            _ => None,
        }
    }
}

//#[derive(EnumNewMacro)]
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
    Load(DefLoad),//not in specification?
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

#[new_aml_struct(ADD_OP)]
struct DefAdd {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

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
    pub fn new(data: &[u8]) -> (Self, usize) {
        debug_assert_eq!(data[0], BUFFER_OP);//sanity check
        todo!();
    }
}

struct DefConcat {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

struct DefConcatRes {
    //TODO:
    //operand1
    //operand2
    target: Target,
}

struct DefCondRefOf {
    //TODO:
    //name
    target: Target,
}

struct DefCopyObject {
    source: TermArg,
    //TODO:
    destination: SimpleName,
}

struct DefDecrement {
    operand: SuperName,
}

pub trait Dereferencable {
    //TODO:
}

struct DefDerefOf {
    operator: TermArg,
}

struct DefDivide {
    dividend: TermArg,
    divisor: TermArg,
    remainder: Target,
    quotient: Target,
}

struct DefFindSetLeftBit {
    operand: TermArg,
    target: Target,
}

struct DefFindSetRightBit {
    operand: TermArg,
    target: Target,
}

struct DefFromBcd {
    operand: TermArg,
    target: Target,
}

struct DefIncrement {
    operand: SuperName,
}

pub trait Indexable {
    //TODO:
}

struct DefIndex {
    operand: TermArg,
    index: TermArg,
}

struct DefLAnd {
    operand1: TermArg,
    operand2: TermArg,
}

struct DefLEqual {
    operand1: TermArg,
    operand2: TermArg,
}

struct DefLGreater {
    operand1: TermArg,
    operand2: TermArg,
}

struct DefLLess {
    operand1: TermArg,
    operand2: TermArg,
}

struct DefLNot {
    operand: TermArg,
}

struct DefLoad {
    name: super::name_objects::NameString,
    target: Target,
}

struct DefLoadTable {//TODO: check documentation
    arg0: TermArg,
    arg1: TermArg,
    arg2: TermArg,
    arg3: TermArg,
    arg4: TermArg,
    arg5: TermArg,
}

struct DefLOr {
    operand1: TermArg,
    operand2: TermArg,
}

struct DefMatch {
    search_pkg: TermArg,
    operand1: TermArg,
    operator: MatchOpcode,
    operand2: TermArg,
    start_index: TermArg,
}

enum MidObj {
    Buffer(),//TODO:
    String(),//TODO:
}

struct DefMid {
    arg1: TermArg,
    arg2: TermArg,
    arg3: TermArg,
    target: Target,
}

struct DefMod {
    dividend: TermArg,
    divisor: TermArg,
    remainder: Target,
}

struct DefMultiply {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

struct DefNand {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

struct DefNor {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

struct DefNot {
    operand: TermArg,
    target: Target,
}

struct DefObjectType {
    //TODO:
}

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
        let (element_list, skip_element_list) = PackageElementList::new_with_len(&data[skip_pkg_len + 2..], num_elements as usize);
        debug_assert!(skip_element_list + skip_pkg_len + 2 == pkg_length.get_length() as usize);
        return Some((Self { elemtn_list: element_list }, pkg_length.get_length() as usize + 1));
    }
}

struct PackageElementList {
    elements: Vec<PackageElement>,
}

impl PackageElementList {
    pub fn new_with_len(data: &[u8], num_items: usize) -> (Self, usize) {
        let mut elements = Vec::new();
        let mut skip = 0;
        for _ in 0..num_items {
            let (element, skip_element) = PackageElement::new(&data[skip..]);
            elements.push(element);
            skip += skip_element;
        }
        return (Self { elements }, skip);
    }

    //reads whole buffer
    pub fn new(data: &[u8]) -> Self {
        let mut elements = Vec::new();
        let mut skip = 0;
        while skip < data.len() {
            let (element, skip_element) = PackageElement::new(&data[skip..]);
            elements.push(element);
            skip += skip_element;
        }
        //sanity check
        debug_assert_eq!(skip, data.len());
        return Self { elements };
    }
}


enum PackageElement {
    DataRefObject(super::data_object::DataRefObject),
    NameString(super::name_objects::NameString),
}

impl PackageElement {
    pub fn new(data: &[u8]) -> (Self, usize) {
        let ref_object = super::data_object::DataRefObject::aml_new(data);
        match ref_object {
            Some((obj, skip)) => return (Self::DataRefObject(obj), skip),
            None => {
                let name_string = super::name_objects::NameString::aml_new(data).unwrap();
                return (Self::NameString(name_string.0), name_string.1.into());
            }
        }
    }
}

pub struct DefVarPackage {
    num_elements: TermArg,
    element_list: PackageElementList
}

impl DefVarPackage {
    pub fn new(data: &[u8]) -> (Self, usize) {
        //sanity check
        debug_assert_eq!(data[0], PACKAGE_OP);
        let (pkg_length, skip_pkg_len) = PkgLength::new(&data[1..]);
        let (num_elements_arg, skip_num_elements_arg) = TermArg::aml_new(&data[skip_pkg_len + 1..]).unwrap();
        let element_list = PackageElementList::new(&data[skip_pkg_len + 1 + skip_num_elements_arg..pkg_length.get_length() as usize + 1]);
        return (Self { element_list, num_elements: num_elements_arg }, pkg_length.get_length() as usize + 1);
    }
}

struct DefRefOf {
    operand: SuperName,
}

struct DefShiftLeft {
    operand: TermArg,
    count: TermArg,
    target: Target,
}

struct DefShiftRight {
    operand: TermArg,
    count: TermArg,
    target: Target,
}

struct DefSizeOf {
    operand: SuperName,
}

struct DefStore {
    value: TermArg,
    target: SuperName,
}

struct DefSubtract {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}

struct DefTimer;//no fields

struct DefToBcd {
    operand: TermArg,
    target: Target,
}

struct DefToBuffer {
    operand: TermArg,
    target: Target,
}

struct DefToDecString {
    operand: TermArg,
    target: Target,
}

struct DefToHexString {
    operand: TermArg,
    target: Target,
}

struct DefToInteger {
    operand: TermArg,
    target: Target,
}

struct DefToString {
    operand: TermArg,
    length_arg: TermArg,
    target: Target,
}

struct DefWait {
    //TODO:
    //event
    operand: TermArg,
}

struct DefXor {
    operand1: TermArg,
    operand2: TermArg,
    target: Target,
}
