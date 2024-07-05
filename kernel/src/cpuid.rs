pub use core::arch::asm;

static mut MAX_LEAF: Option<u64> = None;

#[inline]
fn check_for_overflow(leaf: u64) {
    unsafe {
        if MAX_LEAF.is_none() {
            let max_leaf;
            asm!(
                "push rbx",
                "mov eax, 0",
                "cpuid",
                "pop rbx",
                out("eax") max_leaf,
                out("ecx") _,
                out("edx") _,
            );
            MAX_LEAF = Some(max_leaf);
        }
        assert!(leaf <= MAX_LEAF.unwrap());
    }
}

pub fn get_manufacturer_id() -> [u8; 12] {
    check_for_overflow(0);
    let ebx: u32;
    let edx: u32;
    let ecx: u32;
    unsafe {
        asm!(
            "push rbx",
            "mov eax, 0",
            "cpuid",
            "mov eax, ebx",
            "pop rbx",
            out("eax") ebx,
            out("edx") edx,
            out("ecx") ecx,
        );
        std::mem::transmute((ebx, edx, ecx))
    }
}
