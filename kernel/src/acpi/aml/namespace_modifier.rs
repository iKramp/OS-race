use macros::*;
use functions::*;

use super::{data_object::DataRefObject, name_objects::NameString, term_objects::TermList};


const ALIAS_OP: u8 = 0x06;
const NAME_OP: u8 = 0x08;
const SCOPE_OP: u8 = 0x10;

#[derive(EnumNewMacro)]
pub enum NameSpaceModifierObj {
    Alias(DefAlias),
    Name(DefName),
    Scope(DefScope),
}

#[derive(StructNewMacro)]
#[op_prefix(ALIAS_OP)]
struct DefAlias {
    source: NameString,
    target: NameString,
}

#[derive(StructNewMacro)]
#[op_prefix(NAME_OP)]
struct DefName {
    name: NameString,
    obj: DataRefObject
}

struct DefScope {
    name: NameString,
    terms: TermList,
}

impl DefScope {
    pub fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        if data[0] != SCOPE_OP {
            return None;
        }
        
        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[1..]);
        let (name, skip_name) = NameString::aml_new(&data[skip_pkg_len + 1..]).unwrap();

        let term_list_start = skip_pkg_len + skip_name as usize + 1;
        let term_list_end = pkg_length.get_length() as usize + 1;
        let term_list = TermList::new(&data[term_list_start..term_list_end]);
        
        return Some((Self {
            name,
            terms: term_list,
        }, term_list_end));
    }
}
