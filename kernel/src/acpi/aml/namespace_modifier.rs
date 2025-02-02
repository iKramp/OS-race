use std::boxed::Box;

use macros::*;
use traits::*;

use crate::acpi::aml::namespace::*;

use super::{data_object::DataRefObject, name_objects::NameString, term_objects::TermList};

const ALIAS_OP: u8 = 0x06;
const NAME_OP: u8 = 0x08;
const SCOPE_OP: u8 = 0x10;

#[derive(EnumNewMacro, Debug)]
pub enum NameSpaceModifierObj {
    Alias(Box<DefAlias>),
    Name(Box<DefName>),
    Scope(Box<DefScope>),
}

#[derive(StructNewMacro, Debug)]
#[op_prefix(ALIAS_OP)]
pub struct DefAlias {
    source: NameString,
    target: NameString,
}

#[derive(StructNewMacro, Debug)]
#[op_prefix(NAME_OP)]
pub struct DefName {
    name: NameString,
    obj: DataRefObject,
}

#[derive(Debug)]
pub struct DefScope {
    name: NameString,
    terms: TermList,
}

impl DefScope {
    pub fn check_namespace(data: &[u8]) -> Option<(NameString, usize)> {
        if data[0] != SCOPE_OP {
            return None;
        }

        let mut skip = 1;

        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[skip..]);
        if pkg_length.get_length() + 1 > data.len() {
            return None;
        }
        skip += skip_pkg_len;
        let (name, _skip_name) = NameString::aml_new(&data[skip..])?;
        skip = 1 + pkg_length.get_length();

        Some((name, skip))
    }
}

impl AmlNew for DefScope {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != SCOPE_OP {
            return None;
        }

        let mut skip = 1;

        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[skip..]);
        skip += skip_pkg_len;
        let (name, skip_name) = NameString::aml_new(&data[skip..]).unwrap();
        skip += skip_name;

        let term_list_start = skip;
        let term_list_end = pkg_length.get_length() + 1;
        Namespace::push_namespace_string(get_namespace(), name.clone());
        let (term_list, term_list_len) = TermList::aml_new(&data[term_list_start..term_list_end]).unwrap();
        Namespace::pop_namespace(get_namespace());
        skip += term_list_len;

        Some((Self { name, terms: term_list }, skip))
    }
}
