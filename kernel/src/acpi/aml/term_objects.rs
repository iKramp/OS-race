use std::{Vec, Box};
use macros::{op_prefix, EnumNewMacro};
use functions::EnumNew;

use super::arg_local_obj::{ArgObj, LocalObj};
use super::data_object::DataObject;
use super::named_objects::NamedObj;
use super::namespace_modifier::NameSpaceModifierObj;
use super::statement_opcodes::StatementOpcode;
use super::Integer;
use super::{expression_opcodes::ExpressionOpcode, name_objects::NameString};


#[derive(EnumNewMacro)]
enum TermObj {
    Object(Object),
    StatementOpcode(StatementOpcode),
    ExpressionOpcode(ExpressionOpcode),
}

#[repr(transparent)]
pub struct TermList {
    term_list: Vec<Box<TermObj>>,//wrappers for types or something, or remove generics
}

impl TermList {
    //reads data completely
    pub fn new(data: &[u8]) -> Self {
        let mut skip = 0;
        let mut term_list = Vec::new();
        while skip < data.len() {
            if let Some((term_obj, term_skip)) = TermObj::aml_new(&data[skip..]) {
                term_list.push(Box::new(term_obj));
                skip += term_skip;
            } else {
                panic!("TermList::new: could not read term object");
            }
        }
        //sanity check
        if skip != data.len() {
            panic!("TermList::new: did not read all data");
        }
        TermList {
            term_list,
        }
    }
}

#[derive(EnumNewMacro)]
pub enum TermArg {
    ExpressionOpcode(Box::<ExpressionOpcode>),
    DataObject(Box::<DataObject>),
    ArgObj(ArgObj),
    LocalObj(LocalObj),
}

pub struct MethodInvocation {
    name: NameString,
}

impl MethodInvocation {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        todo!()
    }
}

#[repr(transparent)]
struct TermArgList {
    args: Vec<TermArg>,
}

impl TermArgList {
    pub fn aml_new(data: &[u8]) -> Self {
        let mut skip = 0;
        let mut args = Vec::new();
        while skip < data.len() {
            if let Some((term_arg, term_skip)) = TermArg::aml_new(&data[skip..]) {
                args.push(term_arg);
                skip += term_skip;
            } else {
                panic!("TermArgList::new: could not read term arg");
            }
        }
        //sanity check
        if skip != data.len() {
            panic!("TermArgList::new: did not read all data");
        }
        TermArgList {
            args,
        }
    }
}


#[derive(EnumNewMacro)]
enum Object {
    NameSpaceModifierObj(NameSpaceModifierObj),
    NamedObj(NamedObj)
}
