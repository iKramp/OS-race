use core::mem::MaybeUninit;
use std::vec::Vec;

use super::context::info::ContextInfo;

mod elf;

static mut PROCESS_LOADERS: MaybeUninit<Vec<ProcessLoader>> = MaybeUninit::uninit();

pub fn init_process_loaders() {
    unsafe {
        let proc_vec = PROCESS_LOADERS.write(Vec::new());
        proc_vec.push(elf::proc_loader());
    }
}

struct ProcessLoader {
    is_this_type: fn(&[u8]) -> bool,
    load_process: fn(&[u8]) -> Result<ContextInfo, ProcessLoadError>,
}

#[derive(Debug)]
pub enum ProcessLoadError {
    UnparseableFile,
    UnsupportedProcessFormat, //32 bit for example
}
