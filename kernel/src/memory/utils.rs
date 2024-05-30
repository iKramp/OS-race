static mut PHYSICAL_OFFSET: PhysOffset = PhysOffset(0);
static mut MEM_INITIALIZED: bool = false;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PhysOffset(pub u64);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PhysAddr(pub u64);

impl core::ops::Add for PhysAddr {
    type Output = PhysAddr;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl core::ops::Add<PhysOffset> for PhysAddr {
    type Output = VirtAddr;
    fn add(self, rhs: PhysOffset) -> Self::Output {
        VirtAddr(self.0 + rhs.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VirtAddr(pub u64);

impl core::ops::Add for VirtAddr {
    type Output = VirtAddr;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

///SAFETY: the address must be valid and there are no other references to the
///data
pub unsafe fn get_at_physical_addr<T>(addr: PhysAddr) -> &'static mut T {
    #[cfg(debug_assertions)]
    assert!(MEM_INITIALIZED);
    let data: *mut T = (addr + PHYSICAL_OFFSET).0 as *mut T;
    &mut *data
}

//SAFETY: must be a valid addr (with no other data there)
pub unsafe fn set_at_physical_addr<T>(addr: PhysAddr, data: T) {
    #[cfg(debug_assertions)]
    assert!(MEM_INITIALIZED);
    let data_to_replace: *mut T = (addr + PHYSICAL_OFFSET).0 as *mut T;
    *data_to_replace = data;
}

pub fn get_physical_offset() -> PhysOffset {
    unsafe {
        #[cfg(debug_assertions)]
        assert!(MEM_INITIALIZED);
        PHYSICAL_OFFSET
    }
}

///SAFETY: the address must be valid and there are no other references to the
///data
pub unsafe fn get_at_virtual_addr<T>(addr: VirtAddr) -> &'static mut T {
    #[cfg(debug_assertions)]
    assert!(MEM_INITIALIZED);
    let data: *mut T = addr.0 as *mut T;
    &mut *data
}

//SAFETY: must be a valid addr (with no other data there)
pub unsafe fn set_at_virtual_addr<T>(addr: VirtAddr, data: T) {
    #[cfg(debug_assertions)]
    assert!(MEM_INITIALIZED);
    let data_to_replace: *mut T = addr.0 as *mut T;
    *data_to_replace = data;
}

///SAFETY: the physical address offset must be correct
pub unsafe fn set_physical_offset(addr: PhysOffset) {
    MEM_INITIALIZED = true;
    PHYSICAL_OFFSET = addr;
}

pub unsafe fn translate_virt_phys_addr(addr: VirtAddr) -> Option<PhysAddr> {
    let mut page_addr = PhysAddr(0);
    core::arch::asm!(
        "mov {}, cr3",
        out(reg) page_addr.0,
    );
    #[allow(clippy::unusual_byte_groupings)] //they are grouped by section masks
    let mut final_mask: u64 = 0b111111111_111111111_111111111_111111111_111111111111;
    let mask = 0b111_111_111_000;
    for level in (1..5).rev() {
        let offset = PhysAddr((addr.0 >> (level * 9)) & mask);
        final_mask >>= 9;
        let page_entry = *get_at_physical_addr::<u64>(page_addr + offset);
        let present = page_entry & 1 != 0;
        if !present {
            return None;
        }
        page_addr = PhysAddr(page_entry & 0xFFFFFFFFFF000);
        let huge_page = page_entry & 0b10000000 != 0;
        if huge_page {
            break;
        }
    }
    //here we have the page of the data
    Some(page_addr + PhysAddr(addr.0 & final_mask))
}

pub fn translate_phys_virt_addr(addr: PhysAddr) -> VirtAddr {
    unsafe {
        #[cfg(debug_assertions)]
        assert!(MEM_INITIALIZED);
        addr + PHYSICAL_OFFSET
    }
}

