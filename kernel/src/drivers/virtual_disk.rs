use std::collections::btree_map::BTreeMap;

use super::disk::Disk;


#[derive(Debug)]
pub struct VirtualDisk {
    pub data: BTreeMap<u32, [u8; 512]>,
}

impl Disk for VirtualDisk {
    fn read(&mut self, sector: usize, sec_count: usize, buffer: std::mem_utils::VirtAddr) -> u64 {
        for i in 0..sec_count {
            let mut ptr = buffer.0 as *mut u8;
            ptr = unsafe { ptr.add(i * 512) };
            let data = self.data.get(&(sector as u32 + i as u32)).unwrap_or(&[0; 512]);
            unsafe { (ptr as *mut [u8; 512]).write(*data) };
        }
        return 0;
    }

    fn write(&mut self, sector: usize, sec_count: usize, buffer: std::mem_utils::VirtAddr) -> u64 {
        for i in 0..sec_count {
            let mut ptr = buffer.0 as *mut u8;
            ptr = unsafe { ptr.add(i * 512) };
            let data = self.data.get_mut(&(sector as u32 + i as u32)).unwrap();
            *data = unsafe { *(ptr as *mut [u8; 512]) };
        }
        return 0;
    }

    fn clean_after_read(&mut self, _metadata: u64) {
        return;
    }

    fn clean_after_write(&mut self, _metadata: u64) {
        return;
    }
}
