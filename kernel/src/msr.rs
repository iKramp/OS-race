pub fn get_msr(msr_id: u32) -> u64 {
    let mut eax: u32;
    let mut edx: u32;
    unsafe {
        core::arch::asm!(
            "mov ecx, {msr_id:e}",
            "rdmsr",
            out("edx") edx,
            out("ecx") _,
            out("eax") eax,
            msr_id = in(reg) msr_id
        );
    }
    (edx as u64) << 32 | eax as u64
}

pub fn set_msr(msr_id: u32, value: u64) {
    let eax = value as u32;
    let edx = (value >> 32) as u32;
    unsafe {
        core::arch::asm!(
            "mov ecx, {msr_id:e}",
            "wrmsr",
            in("edx") edx,
            out("ecx") _,
            in("eax") eax,
            msr_id = in(reg) msr_id
        );
    }
}

pub fn get_mtrr_cap() -> u64 {
    get_msr(0xFE)
}

pub fn get_mtrr_def_type() -> u64 {
    get_msr(0x2FF)
}

pub fn set_mtrr_def_type(val: u64) {
    set_msr(0x2FF, val)
}
