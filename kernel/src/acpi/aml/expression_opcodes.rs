use std::{Box, Vec};

use super::data_object::{ComputationalData, EXT_OP_PREFIX};
use super::package::PkgLength;
use super::term_objects::TermArg;
use super::Integer;

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

struct DefAcquire {
    //TODO: MutexObject
    timeout: u16,
}

struct DefAdd {
    operand1: TermArg<Integer>,
    operand2: TermArg<Integer>,
    //TODO:
    //target
}

struct DefAnd {
    operand1: TermArg<Integer>,
    operand2: TermArg<Integer>,
    //TODO:
    //target
}

pub struct DefBuffer {
    pkg_length: super::package::PkgLength,
    byte_list: Vec<u8>,
}

impl DefBuffer {
    pub fn new(data: &[u8]) -> (Self, usize) {
        debug_assert_eq!(data[0], BUFFER_OP);//sanity check
        let pkg_length = super::package::PkgLength::new(data);
        let (buf_size, skip_buf_size) = (0, 0); //evaluate term arg
        let mut buf_data = Vec::new();
        for i in 0..buf_size {
            buf_data.push(data[i as usize]);
        }
        return (Self {
            pkg_length,
            byte_list: buf_data,
        }, buf_size + 1 + skip_buf_size);
    }
}

struct DefConcat {
    operand1: TermArg<ComputationalData>,
    operand2: TermArg<ComputationalData>,
    //TODO:
    //target
}

struct DefConcatRes {
    //TODO:
    //operand1
    //operand2
    //target
}

struct DefCondRefOf {
    //TODO:
    //name
    //target
}

struct DefCopyObject<T> {
    source: TermArg<T>,
    //TODO:
    //destination: SimpleName
}

struct DefDecrement {
    //TODO: operand: SuperName,
}

pub trait Dereferencable {
    //TODO:
}

struct DefDerefOf {
    operator: TermArg<Box<dyn Dereferencable>>,
}

struct DefDivide {
    dividend: TermArg<Integer>,
    divisor: TermArg<Integer>,
    //TODO:
    //remainder: target
    //quotient: target
}

struct DefFindSetLeftBit {
    operand: TermArg<Integer>,
    //TODO:
    //target
}

struct DefFindSetRightBit {
    operand: TermArg<Integer>,
    //TODO:
    //target
}

struct DefFromBcd {
    operand: TermArg<Integer>,
    //TODO:
    //target
}

struct DefIncrement {
    //TODO: operand: SuperName,
}

pub trait Indexable {
    //TODO:
}

struct DefIndex {
    operand: TermArg<Box<dyn Indexable>>,
    index: TermArg<Integer>,
}

struct DefLAnd {
    operand1: TermArg<Integer>,
    operand2: TermArg<Integer>,
}

struct DefLEqual {
    operand1: TermArg<Integer>,
    operand2: TermArg<Integer>,
}

struct DefLGreater {
    operand1: TermArg<Integer>,
    operand2: TermArg<Integer>,
}

struct DefLLess {
    operand1: TermArg<Integer>,
    operand2: TermArg<Integer>,
}

struct DefLNot {
    operand: TermArg<Integer>,
}

struct DefLoad {
    name: super::name_objects::NameString,
    //TODO:
    //target
}

struct DefLoadTable {//TODO: check documentation
    arg0: TermArg<()>,
    arg1: TermArg<()>,
    arg2: TermArg<()>,
    arg3: TermArg<()>,
    arg4: TermArg<()>,
    arg5: TermArg<()>,
}

struct DefLOr {
    operand1: TermArg<Integer>,
    operand2: TermArg<Integer>,
}

struct DefMatch {
    search_pkg: TermArg<super::package::Packgage>,
    operand1: TermArg<Integer>,
    operator: MatchOpcode,
    operand2: TermArg<Integer>,
    start_index: TermArg<Integer>,
}

enum MidObj {
    Buffer(),//TODO:
    String(),//TODO:
}

struct DefMid {
    arg1: TermArg<MidObj>,//TODO:
    arg2: TermArg<()>,
    arg3: TermArg<()>,
    //TODO:
    //target
}

struct DefMod {
    dividend: TermArg<Integer>,
    divisor: TermArg<Integer>,
    //TODO:
    //target
}

struct DefMultiply {
    operand1: TermArg<Integer>,
    operand2: TermArg<Integer>,
    //TODO:
    //target
}

struct DefNand {
    operand1: TermArg<Integer>,
    operand2: TermArg<Integer>,
    //TODO:
    //target
}

struct DefNor {
    operand1: TermArg<Integer>,
    operand2: TermArg<Integer>,
    //TODO:
    //target
}

struct DefNot {
    operand: TermArg<Integer>,
    //TODO:
    //target
}

struct DefObjectType {
    //TODO:
}

struct DefOr {
    operand1: TermArg<Integer>,
    operand2: TermArg<Integer>,
    //TODO:
    //target
}

struct DefPackage {
    length: PkgLength,
    num_elements: u8,
    //TODO: package_element_list: 
}
