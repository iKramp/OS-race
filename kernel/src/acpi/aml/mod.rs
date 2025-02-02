mod aml_code;
mod namespace;
use std::vec::Vec;

pub use aml_code::AmlCode;
mod arg_local_obj;
mod data_object;
mod expression_opcodes;
mod name_objects;
mod named_objects;
mod namespace_modifier;
mod package;
mod statement_opcodes;
mod term_objects;

///test documetnation
struct Integer {
    val_64: u64,
    val_32: u32,
    a: Vec<u32>,
}

//TODO: implement operations
