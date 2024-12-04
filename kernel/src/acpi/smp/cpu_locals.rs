pub struct CpuLocals {
    pub stack_addr: u64,
    pub stack_size: u64,
    pub apic_id: u8,
    pub processor_id: u8,
}

impl CpuLocals {
    pub fn new(stack_addr: u64, stack_size: u64, apic_id: u8, processor_id: u8) -> Self {
        Self {
            stack_addr,
            stack_size,
            apic_id,
            processor_id,
        }
    }

    pub fn get() -> &'static mut Self {
        unsafe {
            let cpu_locals: *mut Self;
            core::arch::asm!(
                "mov {cpu_locals}, gs:0",
                cpu_locals = out(reg) cpu_locals
            );
            &mut *cpu_locals
        }
    }
}
