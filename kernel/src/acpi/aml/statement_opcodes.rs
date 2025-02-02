use macros::*;
use std::Box;
use traits::*;

use super::{
    data_object::{ByteData, DWordData, EXT_OP_PREFIX},
    name_objects::SuperName,
    package::PkgLength,
    term_objects::{TermArg, TermList},
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

#[derive(EnumNewMacro, Debug)]
pub enum StatementOpcode {
    Break(DefBreak),
    BreakPoint(DefBreakPoint),
    Continue(DefContinue),
    Fatal(DefFatal),
    IfElse(Box<DefIfElse>),
    Noop(DefNoop),
    Notify(Box<DefNotify>),
    Release(DefRelease),
    Reset(DefReset),
    Return(DefReturn),
    Signal(DefSignal),
    Sleep(DefSleep),
    Stall(DefStall),
    While(Box<DefWhile>),
}

#[derive(StructNewMacro, Debug)]
#[op_prefix(BREAK_OP)]
pub struct DefBreak;

#[derive(StructNewMacro, Debug)]
#[op_prefix(CONTINUE_OP)]
pub struct DefContinue;

#[derive(StructNewMacro, Debug)]
#[op_prefix(BREAK_POINT_OP)]
pub struct DefBreakPoint;

#[derive(StructNewMacro, Debug)]
#[op_prefix(NOOP_OP)]
pub struct DefNoop;

#[derive(Debug)]
struct ElseBranch {
    term_list: TermList,
}

#[derive(Debug)]
enum DefElse {
    Else(ElseBranch),
    NoElse,
}

impl DefElse {
    pub fn aml_new(data: &[u8]) -> (Self, usize) {
        //sanity check prefix
        if data.is_empty() || data[0] != ELSE_OP {
            return (Self::NoElse, 0);
        }
        let mut skip = 1;
        let (pkg_length, pkg_len_skip) = PkgLength::new(&data[1..]);
        skip += pkg_len_skip;
        let (term_list, term_list_skip) = TermList::aml_new(&data[skip..1 + pkg_length.get_length()]).unwrap();
        skip += term_list_skip;
        (Self::Else(ElseBranch { term_list }), skip)
    }
}

#[derive(Debug, StructNewMacro)]
#[ext_op_prefix(FATAL_OP)]
pub struct DefFatal {
    fatal_type: ByteData,
    fatal_code: DWordData,
    datal_arg: TermArg,
}

#[derive(Debug)]
pub struct DefIfElse {
    predicate: TermArg,
    if_branch: TermList,
    else_branch: DefElse,
}

impl AmlNew for DefIfElse {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != IF_OP {
            return None;
        }
        let mut skip = 1;

        let (pkg_length, pkg_skip) = PkgLength::new(&data[skip..]);
        skip += pkg_skip;
        let (predicate, pred_skip) = TermArg::aml_new(&data[skip..]).unwrap();
        skip += pred_skip;
        let (if_branch, if_branch_skip) = TermList::aml_new(&data[skip..1 + pkg_length.get_length()]).unwrap();
        skip += if_branch_skip;
        let (else_branch, else_skip) = DefElse::aml_new(&data[skip..]);
        skip += else_skip;
        Some((
            Self {
                predicate,
                if_branch,
                else_branch,
            },
            skip,
        ))
    }
}

#[derive(StructNewMacro, Debug)]
#[op_prefix(NOTIFY_OP)]
pub struct DefNotify {
    event_object: SuperName,
    notify_value: TermArg,
}

#[derive(Debug, StructNewMacro)]
#[ext_op_prefix(RELEASE_OP)]
pub struct DefRelease {
    mutex: SuperName,
}

#[derive(Debug, StructNewMacro)]
#[ext_op_prefix(RESET_OP)]
pub struct DefReset {
    event_object: SuperName,
}

#[derive(Debug, StructNewMacro)]
#[op_prefix(RETURN_OP)]
pub struct DefReturn {
    return_value: TermArg,
}

#[derive(Debug, StructNewMacro)]
#[ext_op_prefix(SIGNAL_OP)]
pub struct DefSignal {
    event_object: SuperName,
}

#[derive(Debug, StructNewMacro)]
#[ext_op_prefix(SLEEP_OP)]
pub struct DefSleep {
    sleep_millis: TermArg,
}

#[derive(StructNewMacro, Debug)]
#[ext_op_prefix(STALL_OP)]
pub struct DefStall {
    stall_seconds: TermArg,
}

#[derive(Debug)]
pub struct DefWhile {
    predicate: TermArg,
    term_list: TermList,
}

impl AmlNew for DefWhile {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != WHILE_OP {
            return None;
        }
        let mut skip = 1;
        let (pkg_length, pkg_skip) = PkgLength::new(&data[skip..]);
        skip += pkg_skip;
        let (predicate, pred_skip) = TermArg::aml_new(&data[skip..]).unwrap();
        skip += pred_skip;
        let (term_list, term_list_skip) = TermList::aml_new(&data[skip..1 + pkg_length.get_length()]).unwrap();
        skip += term_list_skip;
        Some((Self { predicate, term_list }, skip))
    }
}
