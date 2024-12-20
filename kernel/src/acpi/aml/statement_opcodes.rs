use std::{Box, Vec};
use functions::EnumNew;

use super::{
    data_object::EXT_OP_PREFIX,
    name_objects::SuperName,
    term_objects::{TermArg, TermList},
    Integer,
};

const BREAK_OP: u8 = 0xA5;
const BREAK_POINT_OP: u8 = 0xCC;
const CONTINUE_OP: u8 = 0x9F;
const ELSE_OP: u8 = 0xA1;
const FATAL_OP: [u8; 2] = [EXT_OP_PREFIX, 0x32];
const IF_OP: u8 = 0xA0;
const NOOP_OP: u8 = 0xA3;
const NOTIFY_OP: u8 = 0x86;
const RELEASE_OP: [u8; 2] = [EXT_OP_PREFIX, 0x27];
const RESET_OP: [u8; 2] = [EXT_OP_PREFIX, 0x26];
const RETURN_OP: u8 = 0xA4;
const SIGNAL_OP: [u8; 2] = [EXT_OP_PREFIX, 0x24];
const SLEEP_OP: [u8; 2] = [EXT_OP_PREFIX, 0x22];
const STALL_OP: [u8; 2] = [EXT_OP_PREFIX, 0x21];
const WHILE_OP: u8 = 0xA2;

pub enum StatementOpcode {
    Break,
    BreakPoint,
    Continue,
    Else(DefElse),
    Fatal(DefFatal),
    IfElse(DefIfElse),
    Noop,
    Notify(DefNotify),
    Release(DefRelease),
    Reset(DefReset),
    Return(DefReturn),
    Signal(DefSignal),
    Sleep(DefSleep),
    Stall(DefStall),
    While(DefWhile),
}

impl StatementOpcode {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        todo!()
    }
}

struct ElseBranch {
    term_list: TermList,
}

enum DefElse {
    Else(ElseBranch),
    NoElse,
}

struct DefFatal {
    fatal_code: u32,
    datal_arg: TermArg,
}

struct DefIfElse {
    predicate: TermArg,
    if_branch: TermList,
    else_branch: DefElse,
}

struct DefNotify {
    //TODO:
}

struct DefRelease {
    mutex: SuperName,
}

impl DefRelease {
    pub fn new(data: &[u8]) -> (Self, usize) {
        //sanity check prefix
        debug_assert_eq!(data[0], RELEASE_OP[0]);
        debug_assert_eq!(data[1], RELEASE_OP[1]);
        let (name, skip) = SuperName::aml_new(&data[2..]).unwrap();
        (Self {
            mutex: name,
        }, skip + 2)
    }
}

struct DefReset {
    event_object: SuperName,
}

impl DefReset {
    pub fn new(data: &[u8]) -> (Self, usize) {
        //sanity check prefix
        debug_assert_eq!(data[0], RESET_OP[0]);
        debug_assert_eq!(data[1], RESET_OP[1]);
        let (name, skip) = SuperName::aml_new(&data[2..]).unwrap();
        (Self {
            event_object: name,
        }, skip + 2)
    }
}

struct DefReturn {
    //TODO: return_value: TermArg
}

struct DefSignal {
    //TODO:
}

struct DefSleep {
    sleep_millis: TermArg,
}

impl DefSleep {
    pub fn new(data: &[u8]) -> (Self, usize) {
        //sanity check prefix
        debug_assert_eq!(data[0], SLEEP_OP[0]);
        debug_assert_eq!(data[1], SLEEP_OP[1]);
        let (term, skip) = TermArg::aml_new(&data[2..]).unwrap();
        (Self {
            sleep_millis: term,
        }, skip + 2)
    }
}

struct DefStall {
    stall_seconds: TermArg,
}

impl DefStall {
    pub fn new(data: &[u8]) -> (Self, usize) {
        //sanity check prefix
        debug_assert_eq!(data[0], STALL_OP[0]);
        debug_assert_eq!(data[1], STALL_OP[1]);
        let (term, skip) = TermArg::aml_new(&data[2..]).unwrap();
        (Self {
            stall_seconds: term,
        }, skip + 2)
    }
}

struct DefWhile {
    predicate: TermArg,
    term_list: TermList,
}
