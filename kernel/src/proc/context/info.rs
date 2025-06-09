use bitfield::bitfield;
use std::{boxed::Box, mem_utils::VirtAddr, vec::Vec};

use crate::proc::MemoryContext;

pub const MAX_PROC_STACK_SIZE_PAGES: usize = 0x4; // 16KB

bitfield! {
    #[derive(Copy, Clone)]
    pub struct MemoryRegionFlags(u32);
    impl Debug;
    pub is_writeable, set_is_writeable: 0;
    pub is_executable, set_is_executable: 1;
}

#[derive(Debug, Clone)]
pub struct MemoryRegionDescriptor {
    ///Guaranteed to be page aligned
    start: VirtAddr,
    size_pages: usize,
    flags: MemoryRegionFlags,
}

impl MemoryRegionDescriptor {
    pub fn new(start: VirtAddr, size_pages: usize, flags: MemoryRegionFlags) -> Result<Self, MemoryRegionError> {
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

        let other_overlaps_self_start = other_start < self_end && other_start >= self_start;
        let self_overlaps_other_start = self_start < other_end && self_start >= other_start;

        other_overlaps_self_start || self_overlaps_other_start
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

#[derive(Debug)]
pub enum MemoryRegionError {
    StartNotPageAligned,
}

///Describes a memory context of a process. For it to be valid, mem_init regions all have to be
///included in the mem_regions
#[derive(Debug)]
pub struct ContextInfo<'a> {
    is_32_bit: bool,
    ///Sorted by start address, no overlapping regions
    mem_regions: Box<[MemoryRegionDescriptor]>,
    mem_init: Box<[(VirtAddr, &'a [u8])]>,
    entry_point: VirtAddr,
    cmdline: Box<str>,
}

impl<'a> ContextInfo<'a> {
    pub fn new(
        is_32_bit: bool,
        mem_regions: &mut [MemoryRegionDescriptor],
        mut mem_init: Box<[(VirtAddr, &'a [u8])]>,
        entry_point: VirtAddr,
        cmdline: Box<str>,
    ) -> Result<Self, ContextInfoError> {
        //Note: This prevents cases where 2 non overlapping regions are in fixed_regions, and a new
        //region that ovrelaps both is added. When sorted, any region added may only extend an
        //existing region, not connect two existing regions.
        mem_regions.sort_by(|lhs, rhs| lhs.start().0.cmp(&rhs.start().0));

        let mut fixed_regions: Vec<MemoryRegionDescriptor> = Vec::new();

        for region in mem_regions.iter() {
            'inner: for other_region in fixed_regions.iter_mut() {
                if region.overlaps(other_region) {
                    if region.flags().0 != other_region.flags().0 {
                        return Err(ContextInfoError::MemoryRegionOverlap);
                    } else {
                        let start_diff = region.start().0 - other_region.start().0;
                        let start_page_diff = start_diff as usize / 0x1000;
                        other_region.size_pages = other_region.size_pages.max(region.size_pages + start_page_diff);
                        break 'inner;
                    }
                }
            }
            fixed_regions.push(region.clone());
        }

        mem_init.sort_by(|lhs, rhs| lhs.0.0.cmp(&rhs.0.0));

        for (i, init_1) in mem_init.iter().enumerate() {
            for init_2 in mem_init[i + 1..].iter() {
                let self_start = init_1.0.0;
                let self_end = init_1.0.0 + (init_1.1.len() as u64);
                let other_start = init_2.0.0;
                let other_end = init_2.0.0 + (init_2.1.len() as u64);

                let other_overlaps_self_start = other_start < self_end && other_start >= self_start;
                let self_overlaps_other_start = self_start < other_end && self_start >= other_start;

                let regions_pverlap = other_overlaps_self_start || self_overlaps_other_start;
                if regions_pverlap {
                    return Err(ContextInfoError::MemoryRegionOverlap);
                }
            }
        }

        Ok(Self {
            is_32_bit,
            mem_regions: fixed_regions.into_boxed_slice(),
            mem_init,
            entry_point,
            cmdline,
        })
    }

    pub fn is_32_bit(&self) -> bool {
        self.is_32_bit
    }

    ///Returns the memory regions of the process. The regions are guaranteed to be sorted by start
    ///address and do not overlap.
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

#[derive(Debug)]
pub enum ContextInfoError {
    MemoryRegionOverlap,
    StackSizeTooBig,
    EntryPointNotMapped,
}

impl Drop for MemoryContext {
    fn drop(&mut self) {
        //unmap higher half and any shared regions
        todo!("All processes running with this context have been dropped. Clean up")
    }
}
