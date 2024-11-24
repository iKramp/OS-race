
pub fn get_mtrr() -> u64 {
    let mut eax = 10;
    let mut edx = 0;
    unsafe {
        core::arch::asm!(
            "mov ecx, 0xFE",
            "rdmsr",
            out("edx") edx,
            out("ecx") _,
            out("eax") eax,
        );
    }
    edx << 32 | eax
}
