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

pub(super) fn load_process(data: &[u8]) -> Result<ContextInfo, ProcessLoadError> {
    unsafe {
        let loaders = PROCESS_LOADERS.assume_init_ref();
        for loader in loaders {
            if (loader.is_this_type)(data) {
                return (loader.load_context)(data);
            }
        }
    }
    Err(ProcessLoadError::UnsupportedProcessFormat)
}

struct ProcessLoader {
    is_this_type: fn(&[u8]) -> bool,
    load_context: fn(&[u8]) -> Result<ContextInfo, ProcessLoadError>,
    //potentially a post load hook
}

#[derive(Debug)]
pub enum ProcessLoadError {
    UnparseableFile,
    InvalidFile,
    UnsupportedProcessFormat, //32 bit, different arch,...
}
