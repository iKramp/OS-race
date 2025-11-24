use std::print;
use std::{sync::no_int_spinlock::NoIntSpinlock, vec::Vec};
use std::boxed::Box;

use crate::vfs::{DeviceId, InodeType};

use super::VfsAdapterTrait;



#[derive(Debug)]
pub(super) struct TtyAdapter {
    buffered_input: NoIntSpinlock<Vec<u8>>,
    ready_input: NoIntSpinlock<Vec<u8>>,
}

impl TtyAdapter {
    pub fn new() -> Self {
        TtyAdapter {
            buffered_input: NoIntSpinlock::new(Vec::new()),
            ready_input: NoIntSpinlock::new(Vec::new()),
        }
    }

    fn get_inode(&self) -> crate::vfs::Inode {
        crate::vfs::Inode {
            index: 0,
            device: DeviceId::new(0),
            type_mode: InodeType::new_file(0o777),
            link_cnt: 1,
            uid: 0,
            gid: 0,
            device_represented: None,
            size: self.ready_input.lock().len() as u64,
            access_time: 0,
            modification_time: 0,
            stat_change_time: 0,
            preferred_block_size: 512,
            blocks: u32::MAX,
        }
    }
}

#[async_trait::async_trait]
impl VfsAdapterTrait for TtyAdapter {
    async fn read(&self, _inode: crate::vfs::InodeIndex, _offset_bytes: u64, size_bytes: u64, buffer: &[std::mem_utils::PhysAddr]) {
        let mut ready_input = self.ready_input.lock();
        let mut block = 0;
        loop {
            if size_bytes == 0 || ready_input.is_empty() {
                break;
            }
            let size_to_read = size_bytes.min(4096).min(ready_input.len() as u64);
            let Some(phys_ptr) = buffer.get(block as usize) else {
                break;
            };
            let ptr = std::mem_utils::translate_phys_virt_addr(*phys_ptr).0 as *mut u8;
            let slice = unsafe { core::slice::from_raw_parts_mut(ptr, size_to_read as usize) };
            slice.copy_from_slice(&ready_input[..size_to_read as usize]);
            ready_input.drain(..size_to_read as usize);
            block += 1;
        }
    }

    async fn read_dir(&self, _inode: crate::vfs::InodeIndex) -> std::boxed::Box<[crate::drivers::disk::DirEntry]> {
        panic!("TTY does not support read_dir");
    }

    async fn write(&self, _inode: crate::vfs::InodeIndex, _offset: u64, size: u64, buffer: &[std::mem_utils::PhysAddr]) -> crate::vfs::Inode {
        for i in 0..(size / 4096) {
            let Some(phys_ptr) = buffer.get(i as usize) else {
                return self.get_inode();
            };
            let ptr = std::mem_utils::translate_phys_virt_addr(*phys_ptr).0 as *const u8;
            let str = unsafe { core::str::from_raw_parts(ptr, 4096) };
            print!("{}", str);
        }
        let Some(phys_ptr) = buffer.last() else {
            return self.get_inode();
        };
        let ptr = std::mem_utils::translate_phys_virt_addr(*phys_ptr).0 as *const u8;
        let str = unsafe { core::str::from_raw_parts(ptr, (size % 4096) as usize) };
        print!("{}", str);

        self.get_inode()
    }

    async fn stat(&self, _inode: crate::vfs::InodeIndex) -> crate::vfs::Inode {
        self.get_inode()
    }
}
