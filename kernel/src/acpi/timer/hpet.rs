use std::{mem_utils::PhysAddr, printlnc};

use bitfield::bitfield;

use crate::memory::{PAGE_TREE_ALLOCATOR, paging::LiminePat};

use super::Timer;

pub(super) static mut HPET: HpetWrapper = HpetWrapper {
    registers: core::ptr::null_mut(),
    is_64_bit: false,
    started: std::time::UNIX_EPOCH,
    cmp_value: 0,
};

pub(super) struct HpetWrapper {
    registers: *mut HpetRegisters,
    started: std::time::Instant,
    is_64_bit: bool,
    cmp_value: u64,
}

impl HpetWrapper {
    fn get_registers(table: &HpetTable) -> bool {
        let phys_addr = PhysAddr(table.base_addr.addr);
        let virt_addr = unsafe { PAGE_TREE_ALLOCATOR.allocate(Some(phys_addr), false) };
        let entry = unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(virt_addr)
                .expect("Failed to get page table entry for HPET")
        };
        entry.set_pat(LiminePat::UC);

        let general_cap = unsafe { (virt_addr.0 as *const GeneralCap).read_volatile() };
        let period_femptosecs = general_cap.counter_clk_period();
        let counter_size_bits = 32 * (1 + general_cap.count_size_cap() as u64);
        let counter_size = 2_u64.pow(counter_size_bits as u32 - 1);
        let mult = period_femptosecs.checked_mul(counter_size);
        let is_ok = if let Some(mult) = mult {
            mult > 10_u64.pow(15) // 1 second in femtoseconds
        } else {
            //overflow, timer is more than capable of 1 second intervals
            true
        };
        if !is_ok {
            printlnc!((255, 0, 0), "HPET: not capable of 1 second intervals");
            return false;
        }

        let periods_in_second = 10_u64.pow(15) / period_femptosecs;

        unsafe {
            HPET.registers = virt_addr.0 as *mut HpetRegisters;
            HPET.is_64_bit = general_cap.count_size_cap();
            HPET.cmp_value = periods_in_second;
        }
        true
    }

    fn start_timer(&mut self) -> bool {
        unsafe {
            let timer_conf = self.registers.byte_offset(0x40) as *mut TimerConfig;
            (timer_conf.byte_offset(0x8) as *mut u64).write_volatile(self.cmp_value);
            let mut conf_reg = (timer_conf as *mut TimerConfAndCap).read_volatile();
            if !conf_reg.periodic_capable() {
                return false;
            }
            conf_reg.set_int_type(false); //edge triggered
            conf_reg.set_int_enable(true); //enable interrupts
            conf_reg.set_type(true); //periodic
            const IO_APIC_ROUTE: u8 = 0x3; //with offset 32, is interrupt 35
            conf_reg.set_int_route(IO_APIC_ROUTE as u64); //route to IO APIC
            (timer_conf as *mut TimerConfAndCap).write_volatile(conf_reg);

            let mut gen_conf = (self.registers.byte_offset(0x10) as *mut GeneralConfig).read_volatile();
            gen_conf.set_enabled(true);
            (self.registers.byte_offset(0x10) as *mut GeneralConfig).write_volatile(gen_conf);
        }
        true
    }
}

impl Timer for HpetWrapper {
    fn start(&self, now: std::time::Instant) -> bool {
        let hpet_regs = unsafe { super::HPET_ACPI_TABLE.assume_init_read() };
        if !Self::get_registers(hpet_regs) {
            return false;
        }
        unsafe {
            HPET.started = now;
            HPET.start_timer()
        }
    }

    fn get_time() -> std::time::Instant {
        todo!()
    }
}

#[repr(C)]
struct HpetRegisters {
    general_capabilities: GeneralCap,
    res_0: u64,
    general_configuration: GeneralConfig,
    res_1: u64,
    interrupt_status: IntStatus,
    res_2: u64,
    ///only write when timer is halted
    ///reads will return the current count value
    main_counter_value: u64,
    res_3: u64,
    timer_0: TimerConfig,
    timer_1: TimerConfig,
    timer_2: TimerConfig,
}

bitfield! {
    struct GeneralCap(u64);
    impl Debug;
    rev_id, _: 7, 0;
    num_tim_cap, _: 12, 8;
    count_size_cap, _: 13;
    leg_route_cap, _: 15;
    vnedor_id, _: 31, 16;
    counter_clk_period, _: 63, 32;
}

bitfield! {
    struct GeneralConfig(u64);
    enabled, set_enabled: 0;
    ///legacy routing to IRQ2 in IO APIC. Don't do this
    leg_rt, set_leg_rt: 1;
}

bitfield! {
    struct IntStatus(u64);
    timer_0, clear_timer_0: 0;
    timer_1, clear_timer_1: 1;
    timer_2, clear_timer_2: 2;
}

#[repr(C)]
struct TimerConfig {
    conf_and_cap: TimerConfAndCap,
    cmp_value: u64,
    fsb_int_route: u64,
    res: u64,
}

bitfield! {
    struct TimerConfAndCap(u64);
    impl Debug;
    ///0: edge tiggered
    ///1: level triggered
    int_type, set_int_type: 1;
    ///only controls interrupt, not operation of the timer
    int_enable, set_int_enable: 2;
    ///1: one-shot
    ///2: periodic
    _type, set_type: 3;
    periodic_capable, _: 4;
    ///0: 32 bits
    ///1: 64 bits
    size, _: 5;
    ///Set in periodic mode BEFORE setting the value of the timer
    _, velue_set: 6;
    _32_bit, set_32_bit: 8;
    ///route in the IO APIC
    int_route, set_int_route: 13, 9;
    //more useless fields
}

#[repr(C, packed)]
pub(in crate::acpi) struct HpetTable {
    header: crate::acpi::sdt::AcpiSdtHeader,
    et_block_id: EventTimerBlockID,
    base_addr: AcpiMemoryDescriptor,
    hpet_number: u8,
    min_count: u16,
    page_prot_attr: u8,
}

bitfield! {
    struct EventTimerBlockID(u32);
    impl Debug;
    pci_vendor_id, _: 31, 16;
    legacy_replacement_capable, _: 15;
    count_size_cap, _: 13;
    num_comparators, _: 12, 8;
    hardware_rev_id, _: 7, 0;
}

#[repr(C, packed)]
struct AcpiMemoryDescriptor {
    mem_space: AcpiMemorySpace,
    reg_bit_width: u8,
    reg_bit_offset: u8,
    rreserved: u8,
    addr: u64,
}

#[repr(u8)]
enum AcpiMemorySpace {
    SystemMemory = 0,
    SystemIO = 1,
}
