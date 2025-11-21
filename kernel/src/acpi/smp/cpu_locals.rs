use core::mem::MaybeUninit;
use std::{
    boxed::Box,
    mem_utils::{VirtAddr, get_at_virtual_addr},
    sync::arc::Arc,
    sync::lock_info::{LockInfo, set_lock_info_func},
    vec::Vec,
};

use crate::{
    acpi::platform_info::PlatformInfo,
    interrupts::{self, idt::TablePointer},
    memory::stack::{KERNEL_STACK_SIZE_PAGES, prepare_kernel_stack},
    proc::ProcessData,
    task_runner::AsyncTaskData,
};

pub static mut CPU_LOCALS: MaybeUninit<Box<[VirtAddr]>> = MaybeUninit::uninit();

#[repr(C)]
pub struct CpuLocals {
    //keep this here
    pub self_addr: VirtAddr,
    //keep this here for syscall reasons
    pub kernel_stack_base: VirtAddr,
    //keep this here for syscall reasons
    pub userspace_stack_base: u64,
    pub stack_size_pages: u64,
    /// Points to TablePointer with base and limit of GDT
    pub gdt_ptr: TablePointer,
    pub current_process: Option<Arc<ProcessData>>,
    pub apic_id: u8,
    pub processor_id: u8,
    pub int_depth: u32,
    pub proc_initialized: bool,
    pub atomic_context: bool,
    pub async_task_data: AsyncTaskData,
    pub lock_info: LockInfo,
}

pub fn init(platform_info: &PlatformInfo) {
    let num_cpus = platform_info.application_processors.len() + 1;
    #[allow(clippy::slow_vector_initialization)] //it's non const ffs
    let mut vec = Vec::with_capacity(num_cpus);
    vec.resize(num_cpus, VirtAddr(0));
    let mut locals_boxed_slice = MaybeUninit::new(vec.into_boxed_slice());
    unsafe { std::mem::swap(&mut locals_boxed_slice, &mut CPU_LOCALS) }
    let old_bsp_local = unsafe { locals_boxed_slice.assume_init_mut()[0] };

    unsafe {
        let cpu_locals = CPU_LOCALS.assume_init_mut();
        let apic_id = platform_info.boot_processor.apic_id;
        cpu_locals[apic_id as usize] = old_bsp_local;
        let bsp_local = get_at_virtual_addr::<CpuLocals>(old_bsp_local); //same addr
        bsp_local.apic_id = apic_id;
        bsp_local.processor_id = platform_info.boot_processor.processor_id;
        let bsp_local_ptr_addr = VirtAddr(cpu_locals[apic_id as usize].0);
        crate::msr::set_msr(0xC0000101, bsp_local_ptr_addr.0);
    }

    //explicitly drop so compiler doesn't drop before writing to msr
    #[allow(clippy::drop_non_drop)]
    drop(locals_boxed_slice);
}

pub fn init_dummy_cpu_locals() {
    #[allow(clippy::slow_vector_initialization)] //it's non const ffs
    let mut vec = Vec::with_capacity(1);
    vec.resize(1, VirtAddr(0));
    unsafe { CPU_LOCALS = MaybeUninit::new(vec.into_boxed_slice()) }

    let bsp_stack_ptr = prepare_kernel_stack(KERNEL_STACK_SIZE_PAGES);
    let bsp_gdt = interrupts::create_new_gdt(bsp_stack_ptr);
    interrupts::load_gdt(bsp_gdt);
    let bsp_local = super::cpu_locals::CpuLocals::new(bsp_stack_ptr, KERNEL_STACK_SIZE_PAGES as u64, 0, 0, bsp_gdt);
    let bsp_local_ptr = add_cpu_locals(bsp_local);
    crate::msr::set_msr(0xC0000101, bsp_local_ptr.0);

    set_lock_info_func(CpuLocals::get_lock_info);
}

pub fn add_cpu_locals(locals: super::cpu_locals::CpuLocals) -> VirtAddr {
    unsafe {
        let apic_id = locals.apic_id;
        let cpu_locals = CPU_LOCALS.assume_init_mut();
        let ptr = std::Box::leak(std::Box::new(locals)) as *mut _ as *mut u64;
        ptr.write_volatile(ptr as u64); //write self pointer

        cpu_locals[apic_id as usize] = VirtAddr(ptr as u64);
        VirtAddr(cpu_locals[apic_id as usize].0)
    }
}

impl CpuLocals {
    pub fn new(kernel_stack_base: VirtAddr, stack_size_pages: u64, apic_id: u8, processor_id: u8, gdt_ptr: TablePointer) -> Self {
        Self {
            kernel_stack_base,
            stack_size_pages,
            apic_id,
            processor_id,
            gdt_ptr,
            current_process: None,
            async_task_data: AsyncTaskData::new(),
            proc_initialized: false,
            int_depth: 0,
            atomic_context: false,
            userspace_stack_base: 0,
            self_addr: VirtAddr(0), //will be set later
            lock_info: LockInfo::new(),
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

    pub fn get_lock_info() -> &'static mut LockInfo {
        &mut Self::get().lock_info
    }
}

//FS register contains thread local storage of a process
