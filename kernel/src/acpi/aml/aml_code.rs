use std::{print, println, string::ToString};

use super::{namespace, term_objects::TermList};
use crate::acpi::sdt::AcpiSdtHeader;

pub struct AmlCode {
    pub header: &'static AcpiSdtHeader,
    pub term_list: TermList,
}

impl AmlCode {
    pub fn new(data: *const u8) -> Self {
        namespace::create_namespace();
        let header = unsafe { &*(data as *const AcpiSdtHeader) };
        let start_index = core::mem::size_of::<AcpiSdtHeader>();
        let data_len = header.length as usize - start_index;
        let data = unsafe { core::slice::from_raw_parts(data.add(start_index), data_len) };
        namespace::get_namespace().scan_for_methods(data);

        if !namespace::get_namespace().current_namespace.is_empty() {
            panic!("Namespace not empty after scanning for methods");
        }
        println!("namespacce scanned for methods");
        //namespace::get_namespace().print_methods();

        let (term_list, _term_list_len) = TermList::aml_new(data).unwrap();

        let message = namespace::get_namespace().root.methods_have_bodies("ROOT".to_string());
        print!("{}", message);
        Self { header, term_list }
    }
}
