use std::mem_utils::{get_at_virtual_addr, VirtAddr};

use crate::proc::context::info::ContextInfo;

use super::ProcessLoader;


pub(super) fn proc_loader() -> ProcessLoader {
    ProcessLoader {
        is_this_type: is_elf,
        load_process: load_elf_process,
    }
}

fn is_elf(data: &[u8]) -> bool {
    data.len() >= 4 && &data[0..4] == b"\x7fELF"
}

fn load_elf_process(data: &[u8]) -> Result<ContextInfo, super::ProcessLoadError> {
    todo!("Load ELF process from data: {:?}", data);
}
