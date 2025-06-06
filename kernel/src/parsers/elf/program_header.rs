#![allow(non_camel_case_types)]

use super::ParseError;
use bitfield::bitfield;

pub struct Elf64_Phdr {
    pub p_type: u32,
    pub p_flags: PFlags,
    ///in file offset to the segment
    pub p_offset: u64,
    pub p_vaddr: u64,
    ///unused
    pub p_paddr: u64,
    ///segment size on disk. Differs from in memory for uninitialized data 
    ///(not on disk, but reserved in memory)
    pub p_filesz: u64,
    ///segment size on memory
    pub p_memsz: u64,
    pub p_align: u64,
}

pub(super) fn get_segment_table(data: &[u8], e_shoff: u64, e_shentsize: u16, e_shnum: u16) -> Result<&[Elf64_Phdr], ParseError> {
    if e_shentsize as usize != core::mem::size_of::<Elf64_Phdr>() {
        return Err(ParseError::InvalidData);
    }
    if e_shoff as usize + (e_shentsize + e_shnum) as usize > data.len() {
        return Err(ParseError::InvalidData);
    }
    unsafe {
        let first_ptr = data.as_ptr().add(e_shoff as usize) as *const Elf64_Phdr;
        let slice = core::slice::from_raw_parts(first_ptr, e_shnum as usize);
        Ok(slice)
    }
}


#[repr(u32)]
#[derive(Debug)]
pub enum PType {
    PT_NULL = 0,
    PT_LOAD = 1,
    PT_DYNAMIC = 2,
    PT_INTER = 3,
    PT_NOTE = 4,
    PT_SHLIB = 5,
    PT_PHDR = 6,
    PT_TLS = 7,
    NUM_P_TYPES = 8,
}

bitfield! {
    pub struct PFlags(u32);
    impl Debug;
    pub execute, _: 0;
    pub write, _: 1;
    pub read, _: 2;
}
