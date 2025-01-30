use core::time::Duration;
use macros::{op_prefix, EnumNewMacro};
use std::{println, thread, Box, Vec};
use traits::*;

use crate::acpi::aml::namespace::get_namespace;

use super::arg_local_obj::{ArgObj, LocalObj};
use super::data_object::DataObject;
use super::named_objects::NamedObj;
use super::namespace_modifier::NameSpaceModifierObj;
use super::statement_opcodes::StatementOpcode;
use super::{expression_opcodes::ExpressionOpcode, name_objects::NameString};

#[derive(EnumNewMacro, Debug)]
enum TermObj {
    Object(Object),
    StatementOpcode(StatementOpcode),
    ExpressionOpcode(ExpressionOpcode),
}

#[derive(Debug)]
#[repr(transparent)]
pub struct TermList {
    term_list: Vec<TermObj>,
}

impl TermList {
    //reads data completely
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        let mut skip = 0;
        let mut term_list = Vec::new();
        while skip < data.len() {
            if let Some((term_obj, term_skip)) = TermObj::aml_new(&data[skip..]) {
                term_list.push(term_obj);
                skip += term_skip;
            } else {
                let min_index = skip.checked_sub(100).unwrap_or(0);
                let max_index = usize::min(skip + 100, data.len());
                panic!(
                    "TermList::new: could not read term object.\nPreceeding data: {:x?}\nPreceeding objects: {:#x?}\nFollowing data: {:x?}", 
                    &data[min_index..skip],
                    term_list,
                    &data[skip..max_index]
                );
            }
        }
        //sanity check
        if skip != data.len() {
            panic!(
                "TermList::new: did not read all data: {} != {}\n{:x?}",
                skip,
                data.len(),
                data
            );
        }
        Some((TermList { term_list }, skip))
    }
}

#[derive(Debug, EnumNewMacro)]
pub enum TermArg {
    DataObject(Box<DataObject>),
    ArgObj(ArgObj),
    LocalObj(LocalObj),
    ExpressionOpcode(Box<ExpressionOpcode>),
}

#[derive(Debug)]
pub struct MethodInvocation {
    name: NameString,
    args: Vec<TermArg>,
}

impl AmlNew for MethodInvocation {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        let (name, name_skip) = NameString::aml_new(data)?;
        let n_args = get_namespace().get_method(&name)?.0;
        let mut args = std::Vec::new();
        let mut skip = name_skip;
        for _ in 0..n_args {
            if let Some((term_arg, term_skip)) = TermArg::aml_new(&data[skip..]) {
                args.push(term_arg);
                skip += term_skip;
            } else {
                panic!("MethodInvocation::new: could not read term arg");
            }
        }
        Some((MethodInvocation { name, args }, skip))

    }
}

#[derive(Debug)]
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
            panic!(
                "TermArgList::new: did not read all data: {} != {}\n{:x?}",
                skip,
                data.len(),
                data
            );
        }
        TermArgList { args }
    }
}

#[derive(EnumNewMacro, Debug)]
enum Object {
    NameSpaceModifierObj(NameSpaceModifierObj),
    NamedObj(NamedObj),
}
