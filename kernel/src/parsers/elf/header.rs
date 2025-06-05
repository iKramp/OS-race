#![allow(non_camel_case_types)] //these types conform to the ELF specification. Will be re-exported

use super::ParseError;

/*
    ElfN_Addr       Unsigned program address, uintN_t
    ElfN_Off        Unsigned file offset, uintN_t
    ElfN_Section    Unsigned section index, uint16_t
    ElfN_Versym     Unsigned version symbol information, uint16_t
    Elf_Byte        unsigned char
    ElfN_Half       uint16_t
    ElfN_Sword      int32_t
    ElfN_Word       uint32_t
    ElfN_Sxword     int64_t
    ElfN_Xword      uint64_t
*/

pub struct Elf64_Ehdr {
    e_ident: e_ident,
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

impl Elf64_Ehdr {
    fn parse(data: &[u8]) -> Result<&Self, ParseError> {
        if data.len() < core::mem::size_of::<Self>() {
            return Err(ParseError::IncompleteData);
        }
        e_ident::verify(data)?;
        let header = unsafe { &*(data.as_ptr() as *const Elf64_Ehdr) };
        Ok(header)
    }
}

pub struct e_ident {
    /// (0x7f)ELF magic number
    pub ei_magic: [u8; 4],
    /// ELF class (0: invalid, 1: 32-bit, 2: 64-bit)
    pub ei_class: u8,
    /// Data encoding (0: invalid, 1: little-endian, 2: big-endian)
    pub ei_data: u8,
    /// ELF version (0: invalid, 1: original)
    pub ei_version: u8,
    /// OS/ABI identification
    /// 0: None
    /// 1: System V
    /// 2: HP-UX
    /// 3: NetBSD
    /// 4: Linux
    /// 5: Solaris
    /// 6: Irix
    /// 7: FreeBSD
    /// 8: Tru64
    /// 9: Arm
    /// 10: Standalone
    pub ei_osabi: u8,
    /// ABI version
    pub ei_abiversion: u8,
    pub ei_pad: [u8; 6],
    pub ei_nident: u8,
}

impl e_ident {
    fn verify(data: &[u8]) -> Result<(), ParseError> {
        let header = unsafe { &*(data.as_ptr() as *const e_ident) };
        if header.ei_magic != [0x7f, b'E', b'L', b'F'] {
            return Err(ParseError::InvalidMagic);
        }
        if header.ei_class != 2 {
            return Err(ParseError::InvalidClass);
        }
        if header.ei_data != 1 {
            return Err(ParseError::InvalidDataEncoding);
        }
        if header.ei_version != 1 {
            return Err(ParseError::InvalidVersion);
        }
        if header.ei_nident != core::mem::size_of::<Self>() as u8 {
            return Err(ParseError::Other);
        }
        Ok(())
    }
}
