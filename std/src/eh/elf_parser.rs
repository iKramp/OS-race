use crate::{eh::int3, mem_utils::VirtAddr, println};

//we only support 64 bit
#[repr(C)]
#[derive(Debug)]
pub struct ElfHeader {
    magic: [u8; 4],
    class: u8, //always 2 for 64 bit
    endianness: u8,
    version_0: u8, //always 1
    os_abi: u8,
    padding: [u8; 7],
    file_type: u16, //apparently relocatable for me?
    machine: u16,
    version_1: u32, //always 1
    entry: u64,
    program_header_offset: u64,
    section_header_table_offset: u64,
    flags: u32,
    eh_size: u16,
    program_header_entry_size: u16,
    program_header_entries: u16,
    section_header_entry_size: u16,
    section_header_entries: u16,
    shstr_index: u16,
}

impl ElfHeader {
    pub fn new(file: VirtAddr) -> &'static Self {
        let header = unsafe { crate::mem_utils::get_at_virtual_addr::<ElfHeader>(file) };
        assert!(header.magic == [0x7f, 0x45, 0x4c, 0x46]);
        header
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct SectionHeader {
    section_header_name_offset: u32,
    section_type: u32,
    flags: u64,
    memory_address: u64,
    file_offset: u64,
    size: u64,
    link: u32,
    info: u32,
    address_align: u64,
    entry_size: u64,
}

#[derive(Debug)]
pub struct Section {
    header: &'static SectionHeader,
    name: &'static str,
    content: &'static [u8],
}

impl Section {
    pub fn new(file: VirtAddr, header: &'static SectionHeader, name: &'static str) -> Self {
        Self {
            header,
            name,
            content: unsafe {
                core::slice::from_raw_parts(
                    (file.0 as usize + header.file_offset as usize) as *const u8,
                    header.size as usize,
                )
            },
        }
    }
}

#[derive(Debug)]
pub struct ElfFile {
    header: &'static ElfHeader,
    shstrtab_section: Section,
    debug_abbrev_section: Section,
    debug_info_section: Section,
    debug_aranges_section: Section,
    debug_ranges_section: Section,
    debug_str_section: Section,
    debug_frame_section: Section,
    debug_line_section: Section,
    debug_loc_section: Section,
}

impl ElfFile {
    pub fn new(file: VirtAddr) -> Self {
        let header = ElfHeader::new(file);
        let shstrtab_section = unsafe {
                crate::mem_utils::get_at_virtual_addr::<SectionHeader>(
                    file + VirtAddr(header.section_header_table_offset + header.shstr_index as u64 * core::mem::size_of::<SectionHeader>() as u64),
                )
        };
        let mut debug_abbrev_section = None;
        let mut debug_info_section = None;
        let mut debug_aranges_section = None;
        let mut debug_ranges_section = None;
        let mut debug_str_section = None;
        let mut debug_frame_section = None;
        let mut debug_line_section = None;
        let mut debug_loc_section = None;
        for i in 0..header.section_header_entries {
            let section_header = unsafe {
                crate::mem_utils::get_at_virtual_addr::<SectionHeader>(
                    file + VirtAddr(header.section_header_table_offset + i as u64 * core::mem::size_of::<SectionHeader>() as u64),
                )
            };
            let section_name = unsafe {
                core::ffi::CStr::from_ptr(
                    (file.0 + shstrtab_section.file_offset + section_header.section_header_name_offset as u64) as usize
                        as *const core::ffi::c_char,
                )
            };
            let Ok(section_name) = section_name.to_str() else {
                panic!("Failed to convert section name to str: {:#x?}", section_name);
            };
            
            match section_name {
                ".debug_abbrev" => debug_abbrev_section = Some(Section::new(file, section_header, section_name)),
                ".debug_info" | ".debug_info.dwo" => debug_info_section = Some(Section::new(file, section_header, section_name)),
                ".debug_aranges" => debug_aranges_section = Some(Section::new(file, section_header, section_name)),
                ".debug_ranges" => debug_ranges_section = Some(Section::new(file, section_header, section_name)),
                ".debug_str" => debug_str_section = Some(Section::new(file, section_header, section_name)),
                ".debug_frame" => debug_frame_section = Some(Section::new(file, section_header, section_name)),
                ".debug_line" => debug_line_section = Some(Section::new(file, section_header, section_name)),
                ".debug_loc" => debug_loc_section = Some(Section::new(file, section_header, section_name)),
                _ => {}
            }
        }
        Self {
            header,
            shstrtab_section: Section::new(file, shstrtab_section, ".shstrtab"),
            debug_abbrev_section: debug_abbrev_section.unwrap(),
            debug_info_section: debug_info_section.unwrap(),
            debug_aranges_section: debug_aranges_section.unwrap(),
            debug_ranges_section: debug_ranges_section.unwrap(),
            debug_str_section: debug_str_section.unwrap(),
            debug_frame_section: debug_frame_section.unwrap(),
            debug_line_section: debug_line_section.unwrap(),
            debug_loc_section: debug_loc_section.unwrap(),
        }
    }
}
