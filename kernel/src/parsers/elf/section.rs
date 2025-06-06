#![allow(non_camel_case_types)]

use bitfield::bitfield;

use super::ParseError;

///Section header struct
#[repr(C)]
#[derive(Debug)]
pub struct Elf64_Shdr {
    ///index into string table
    pub sh_name: u32,
    pub sh_type: u32,
    pub sh_flags: u32,
    ///address of section at execution
    pub sh_addr: u64,
    ///section offset in file
    pub sh_offset: u64,
    ///section size
    pub sh_size: u64,
    pub sh_link: u32,
    pub sh_info: u32,
    pub sh_addralign: u64,
    ///entry size if section holds a table
    pub sh_entsize: u64,
}

pub(super) fn get_section_table(data: &[u8], e_phoff: u64, e_phentsize: u16, e_phnum: u16) -> Result<&[Elf64_Shdr], ParseError> {
    if e_phentsize as usize != core::mem::size_of::<Elf64_Shdr>() {
        return Err(ParseError::InvalidData);
    }
    if e_phoff as usize + (e_phentsize + e_phnum) as usize > data.len() {
        return Err(ParseError::InvalidData);
    }
    unsafe {
        let first_ptr = data.as_ptr().add(e_phoff as usize) as *const Elf64_Shdr;
        let slice = core::slice::from_raw_parts(first_ptr, e_phnum as usize);
        Ok(slice)
    }
}

#[repr(u32)]
#[derive(Debug)]
pub enum ShType {
    SHT_NULL = 0,            /* unused section */
    SHT_PROGBITS = 1,        /* Program data */
    SHT_SYMTAB = 2,          /* Symbol table */
    SHT_STRTAB = 3,          /* String table */
    SHT_RELA = 4,            /* Relocation entries with addends */
    SHT_HASH = 5,            /* Symbol hash table */
    SHT_DYNAMIC = 6,         /* Dynamic linking information */
    SHT_NOTE = 7,            /* Notes */
    SHT_NOBITS = 8,          /* Program space with no data (bss) */
    SHT_REL = 9,             /* Relocation entries, no addends */
    SHT_SHLIB = 10,          /* Reserved */
    SHT_DYNSYM = 11,         /* Dynamic linker symbol table */
    SHT_INIT_ARRAY = 14,     /* Array of constructors */
    SSHT_FINI_ARRAY = 15,    /* Array of destructors */
    SSHT_PREINIT_ARRAY = 16, /* Array of pre-constructors */
    SSHT_GROUP = 17,         /* Section group */
    SSHT_SYMTAB_SHNDX = 18,  /* Extended section indeces */
    NUM_SH_TYPES = 19,      /* Number of defined types */
}

bitfield! {
    struct ShFlags(u64);
    impl Debug;
    shf_write, _: 0;
    shf_alloc, _: 1;
    ///Is init section
    shf_execinstr, _: 2;
    shf_merge, _: 4;
    shf_strings, _: 5;
    shf_info_link, _: 6;
    shf_link_order, _: 7;
    shf_os_nonconfirming, _: 8;
    shf_group, _: 9;
    shf_tls, _: 10;
    shf_compressed, _: 11;
}
