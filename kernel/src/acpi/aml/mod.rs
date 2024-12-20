mod aml_code;
mod name_objects;
mod data_object;
mod package;
mod term_objects;
mod expression_opcodes;
mod statement_opcodes;
mod namespace_modifier;
mod arg_local_obj;
mod named_objects;

struct Integer {
    val_64: u64,
    val_32: u32,
}

//TODO: implement operations
