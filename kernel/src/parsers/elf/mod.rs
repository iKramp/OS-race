use std::{boxed::Box, println, vec::Vec};

use crate::memory;

mod header;
mod program_header;
mod section;


#[derive(Debug)]
pub enum ParseError {
    InvalidMagic,
    InvalidClass,
    InvalidDataEncoding,
    InvalidVersion,
    IncompleteData,
    InvalidData,
    Other,
}

pub fn parse(data: &[u8]) -> Result<(), ParseError> {
    let header = header::Elf64_Ehdr::parse(data)?;
    let section_headers = section::get_section_table(data, header.e_shoff, header.e_shentsize, header.e_shnum)?;
    let segment_headers = program_header::get_segment_table(data, header.e_phoff, header.e_phentsize, header.e_phnum)?;
    let string_section_header = &section_headers[header.e_shstrndx as usize];
    let start_shstr = string_section_header.sh_offset as usize;
    let end_shstr = start_shstr + string_section_header.sh_size as usize;
    let shstrtab = &data[start_shstr..end_shstr];

    let mut sections_names: Vec<Box<str>> = Vec::new();
    let shstrtab_ptr = shstrtab.as_ptr();
    let mut start_str = 0;
    for i in 0..shstrtab.len() {
        let character = shstrtab[i];
        if character == b'\0' {
            let new_str = unsafe { core::str::from_raw_parts(shstrtab_ptr.add(start_str), i - start_str) };
            sections_names.push(new_str.into());
            start_str = i + 1;
        }
    }
    println!("{:?}", sections_names);
    


    Ok(())
}

pub fn parse_unaligned(data: &[u8]) -> Result<(), ParseError> {
    let size_pages = data.len().div_ceil(0x1000);
    let virt_addr = unsafe { memory::PAGE_TREE_ALLOCATOR.allocate_contigious(size_pages as u64, None, false) };

    let source = data.as_ptr();
    let dest = virt_addr.0 as *mut u8;
    unsafe { core::ptr::copy(source, dest, data.len()) }

    let data = unsafe { core::slice::from_raw_parts(dest, data.len()) };
    parse(data)
}
