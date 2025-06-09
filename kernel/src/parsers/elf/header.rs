#![allow(non_camel_case_types)] //these types conform to the ELF specification. Will be re-exported
use super::ParseError;

#[repr(C)]
#[derive(Debug)]
pub struct Elf64_Ehdr {
    pub e_ident: EIdent,
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

impl Elf64_Ehdr {
    pub(super) fn parse(data: &[u8]) -> Result<&Self, ParseError> {
        if data.len() < core::mem::size_of::<Self>() {
            return Err(ParseError::IncompleteData);
        }
        EIdent::verify(data)?;
        let header = unsafe { &*(data.as_ptr() as *const Elf64_Ehdr) };
        if header.e_ehsize != core::mem::size_of::<Self>() as u16 {
            return Err(ParseError::Other);
        }
        Ok(header)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct EIdent {
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
    pub ei_osabi: u8,
    /// ABI version
    pub ei_abiversion: u8,
    pub ei_pad: [u8; 6],
    pub ei_nident: u8,
}

impl EIdent {
    fn verify(data: &[u8]) -> Result<(), ParseError> {
        let header = unsafe { &*(data.as_ptr() as *const Self) };
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
        Ok(())
    }
}

#[repr(u16)]
pub enum EType {
    ET_NONE = 0,
    ET_REL = 1,
    ET_EXEC = 2,
    ET_DYN = 3,
    ET_CORE = 4,
}

#[repr(u8)]
pub enum EiOsAbi {
    None = 0,
    SystemV = 1,
    HpUx = 2,
    NetBSD = 3,
    Linux = 4,
    Solaris = 5,
    Irix = 6,
    FreeBSD = 7,
    Tru64 = 8,
    Arm = 9,
    Standalone = 10,
}
