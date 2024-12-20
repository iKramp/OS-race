use super::{data_object::DataRefObject, name_objects::NameString, term_objects::TermList};


const ALIAS_OP: u8 = 0x06;
const NAME_OP: u8 = 0x08;
const SCOPE_OP: u8 = 0x10;

pub enum NameSpaceModifierObj {
    Alias(DefAlias),
    Name(DefName),
    Scope(DefScope),
}

impl NameSpaceModifierObj {
    pub fn new(data: &[u8]) -> Option<(Self, usize)> {
        match data[0] {
            ALIAS_OP => {
                let (alias, skip) = DefAlias::new(data);
                Some((Self::Alias(alias), skip))
            }
            NAME_OP => {
                let (name, skip) = DefName::new(data);
                Some((Self::Name(name), skip))
            }
            SCOPE_OP => {
                let (scope, skip) = DefScope::new(data);
                Some((Self::Scope(scope), skip))
            }
            _ => None,
        }
    }
}

struct DefAlias {
    source: NameString,
    target: NameString,
}

impl DefAlias {
    pub fn new(data: &[u8]) -> (Self, usize) {
        //sanity check
        debug_assert_eq!(data[0], ALIAS_OP);
        let (source, skip_source) = NameString::aml_new(&data[1..]).unwrap();
        let (target, skip_target) = NameString::aml_new(&data[1 + skip_source as usize..]).unwrap();
        return (Self {
            source,
            target,
        }, skip_source as usize + skip_target as usize + 1);
    }
}

struct DefName {
    name: NameString,
    obj: DataRefObject
}

impl DefName {
    pub fn new(data: &[u8]) -> (Self, usize) {
        //sanity check
        debug_assert_eq!(data[0], NAME_OP);
        let (name, skip_name) = NameString::aml_new(&data[1..]).unwrap();
        let (obj, skip_obj) = DataRefObject::aml_new(&data[1 + skip_name as usize..]).unwrap();
        return (Self {
            name,
            obj,
        }, skip_name as usize + skip_obj as usize + 1);
    }
}

struct DefScope {
    name: NameString,
    terms: TermList,
}

impl DefScope {
    pub fn new(data: &[u8]) -> (Self, usize) {
        //sanity check
        debug_assert_eq!(data[0], SCOPE_OP);
        
        let (pkg_length, skip_pkg_len) = super::package::PkgLength::new(&data[1..]);
        let (name, skip_name) = NameString::aml_new(&data[skip_pkg_len + 1..]).unwrap();

        let term_list_start = skip_pkg_len + skip_name as usize + 1;
        let term_list_end = pkg_length.get_length() as usize + 1;
        let term_list = TermList::new(&data[term_list_start..term_list_end]);
        
        return (Self {
            name,
            terms: term_list,
        }, term_list_end);
    }
}
