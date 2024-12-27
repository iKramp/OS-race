
use crate::acpi::sdt::AcpiSdtHeader;

use super::term_objects::TermList;

pub struct AmlCode {
    pub header: &'static AcpiSdtHeader,
    pub term_list: TermList
}

impl AmlCode {
    pub fn new(data: *const u8) -> Self {
        let header = unsafe { &*(data as *const AcpiSdtHeader) };
        let start_index = core::mem::size_of::<AcpiSdtHeader>();
        let end_index = header.length as usize;
        let data = unsafe { core::slice::from_raw_parts(data.add(start_index), end_index) };
        let term_list = TermList::new(&data);
        Self {
            header,
            term_list
        }
    }
}
