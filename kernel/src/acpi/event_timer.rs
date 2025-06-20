//uses HPET

use core::mem::MaybeUninit;
use std::{collections::btree_set::{self, BTreeSet}, mem_utils::PhysAddr, sync::mutex::Mutex, vec::Vec};

use bitfield::bitfield;

use crate::{memory::{paging::LiminePat, PAGE_TREE_ALLOCATOR}, proc::Pid};

use super::cpu_locals;

struct ScheduledEvent {
    pub time: std::time::Instant,
    pub lapic_id: u8,
    pub cause: EventCause,
}

pub enum EventCause {
    Sleep,
    ProcSleep(Pid),
}

struct EventConfig {
    queue: Vec<ScheduledEvent>,
    hpet_registers: &'static mut HpetRegisters,
}

static EVENT_CONFIG: Mutex<MaybeUninit<EventConfig>> = Mutex::new(MaybeUninit::uninit());

pub fn init(table: &HpetTable) {
    let phys_addr = PhysAddr(table.base_addr.addr);
    let virt_addr = unsafe { PAGE_TREE_ALLOCATOR.allocate(Some(phys_addr), false) };
    let entry = unsafe {
        PAGE_TREE_ALLOCATOR
            .get_page_table_entry_mut(virt_addr)
            .expect("Failed to get page table entry for HPET")
    };
    entry.set_pat(LiminePat::UC);
    let hpet_ref = unsafe { &mut *(virt_addr.0 as *mut HpetRegisters) };
    EVENT_CONFIG.lock().write(EventConfig {
        queue: Vec::new(),
        hpet_registers: hpet_ref,
    });
}

pub fn schedule_event_duration(cause: EventCause, duration: std::time::Duration) {
    let now = std::time::Instant::now();
    let event_time = now + duration;
    schedule_event_instant(cause, event_time);
}

pub fn schedule_event_instant(cause: EventCause, instant: std::time::Instant) {
    let event = ScheduledEvent {
        time: instant,
        lapic_id: cpu_locals::CpuLocals::get().apic_id,
        cause,
    };
    let now = std::time::Instant::now();
    if now - event.time < core::time::Duration::from_micros(1) {
        handle_event(event);
        return;
    }
    let mut conf_lock = EVENT_CONFIG.lock();
    let mut config = unsafe { conf_lock.assume_init_mut() };
    let general_config = config.hpet_registers.general_configuration.enabled();
    let pos = config.queue.binary_search_by(|e| {
        e.time.cmp(&event.time)
    });
    let pos = match pos {
        Ok(pos) | Err(pos) => pos,
    };
    config.queue.insert(pos, event);
}

pub fn handle_events() {
    todo!("loop through events, find the ones to handle");
}

fn handle_event(event: ScheduledEvent) {
    match event.cause {
        EventCause::Sleep => {} //just returns and resumes execution
        EventCause::ProcSleep(_pid) => {
            // Handle process sleep event
            // This could involve waking up the process or similar
        }
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
pub(super) struct HpetTable {
    header: super::sdt::AcpiSdtHeader,
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
