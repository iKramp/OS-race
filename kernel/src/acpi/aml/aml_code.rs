use std::mem_utils::get_at_virtual_addr;

use crate::acpi::sdt::AcpiSdtHeader;

use super::term_objects::TermList;

pub struct AmlCode {
    pub header: &'static AcpiSdtHeader,
    pub term_list: TermList
}

impl AmlCode {
    pub fn new(data: &[u8]) -> Self {
        let header = unsafe { &*(data as *const _ as *const u8 as *const AcpiSdtHeader) };
        let start_index = core::mem::size_of::<AcpiSdtHeader>();
        let end_index = header.length as usize;
        let term_list = TermList::new(&data[start_index..end_index]);
        Self {
            header,
            term_list
        }
    }
}
