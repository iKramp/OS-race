use std::{boxed::Box, mem_utils::VirtAddr};
use bitfield::bitfield;

use crate::proc::MemoryContext;

pub const MAX_PROC_STACK_SIZE_PAGES: usize = 0x4; // 16KB

bitfield! {
    #[derive(Copy, Clone)]
    pub struct MemoryRegionFlags(u32);
    impl Debug;
    pub is_writeable, set_is_writeable: 1;
    pub is_executable, set_is_executable: 2;
}

pub struct MemoryRegionDescriptor {
    start: VirtAddr,
    size_pages: usize,
    flags: MemoryRegionFlags,
}

impl MemoryRegionDescriptor {
    pub fn new(
        start: VirtAddr,
        size_pages: usize,
        flags: MemoryRegionFlags,
    ) -> Result<Self, MemoryRegionError> {
        if start.0 % 0x1000 != 0 {
            return Err(MemoryRegionError::StartNotPageAligned);
        }
        
        Ok(Self {
            start,
            size_pages,
            flags,
        })
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        let self_start = self.start.0;
        let self_end = self.start.0 + (self.size_pages as u64 * 0x1000);
        let other_start = other.start.0;
        let other_end = other.start.0 + (other.size_pages as u64 * 0x1000);

        (self_end < other_start) || (self_start > other_end)
    }

    pub fn start(&self) -> VirtAddr {
        self.start
    }

    pub fn size_pages(&self) -> usize {
        self.size_pages
    }

    pub fn flags(&self) -> MemoryRegionFlags {
        self.flags
    }
}

pub enum MemoryRegionError {
    StartNotPageAligned,
}

///Describes a memory context of a process. For it to be valid, mem_init regions all have to be
///included in the mem_regions
pub struct ContextInfo<'a> {
    is_32_bit: bool,
    stack_size_pages: Option<u8>,
    mem_regions: Box<[MemoryRegionDescriptor]>,
    mem_init: Box<[(VirtAddr, &'a [u8])]>,
    entry_point: VirtAddr,
    cmdline: Box<str>,
}

impl<'a> ContextInfo<'a> {
    pub fn new(
        is_32_bit: bool,
        stack_size_pages: Option<u8>,
        mem_regions: Box<[MemoryRegionDescriptor]>,
        mem_init: Box<[(VirtAddr, &'a [u8])]>,
        entry_point: VirtAddr,
        cmdline: Box<str>,
    ) -> Result<Self, ContextInfoError> {
        for (i, region) in mem_regions.iter().enumerate() {
            for j in (i + 1)..mem_regions.len() {
                if region.overlaps(&mem_regions[j]) {
                    return Err(ContextInfoError::MemoryRegionOverlap);
                }
            }
        }
        if let Some(stack_size) = stack_size_pages {
            if stack_size > MAX_PROC_STACK_SIZE_PAGES as u8 {
                return Err(ContextInfoError::StackSizeTooBig);
            }
        }
        if !mem_regions.iter().any(|region| region.start == entry_point) {
            return Err(ContextInfoError::EntryPointNotMapped);
        }

        Ok(Self {
            is_32_bit,
            stack_size_pages,
            mem_regions,
            mem_init,
            entry_point,
            cmdline,
        })
    }

    pub fn is_32_bit(&self) -> bool {
        self.is_32_bit
    }

    pub fn stack_size_pages(&self) -> Option<u8> {
        self.stack_size_pages
    }

    pub fn mem_regions(&self) -> &[MemoryRegionDescriptor] {
        &self.mem_regions
    }

    pub fn mem_init(&self) -> &[(VirtAddr, &'a [u8])] {
        &self.mem_init
    }

    pub fn entry_point(&self) -> VirtAddr {
        self.entry_point
    }

    pub fn cmdline(&self) -> &str {
        &self.cmdline
    }

    pub fn cmdline_as_bytes(&self) -> &[u8] {
        self.cmdline.as_bytes()
    }
}

pub enum ContextInfoError {
    MemoryRegionOverlap,
    StackSizeTooBig,
    EntryPointNotMapped
}

impl Drop for MemoryContext {
    fn drop(&mut self) {
        //unmap higher half and any shared regions
        todo!("All processes running with this context have been dropped. Clean up")
    }
}
