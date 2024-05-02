use super::gdt::{DOUBLE_FAULT_IST, MACHINE_CHECK_IST, NMI_IST};
use super::handlers::*;
use crate::println;
use crate::vga::vga_text::set_vga_text_foreground;
use core::arch::asm;

macro_rules! interrupt_message {
    ($name: expr) => {{
        extern "x86-interrupt" fn wrapper(_stack_frame: ExceptionStackFrame) -> ! {
            set_vga_text_foreground((0, 0, 255));
            println!("{} exception", $name);
            set_vga_text_foreground((255, 255, 255));
            loop {}
        }
        wrapper
    }};
}

#[repr(C, packed)]
pub struct TablePointer {
    pub limit: u16,
    pub base: u64,
}

#[used]
static mut IDT_POINTER: TablePointer = TablePointer { limit: 0, base: 0 };

const IDT_SIZE: usize = 256;

pub struct Idt {
    entry_table: [Entry; IDT_SIZE],
}

#[used]
static mut IDT: Idt = Idt::new();

impl Idt {
    pub const fn new() -> Self {
        Idt {
            entry_table: [Entry::missing(); IDT_SIZE],
        }
    }

    fn set(&mut self, entry: Entry, index: usize) {
        self.entry_table[index] = entry;
    }

    pub fn load(&'static self) {
        unsafe {
            IDT_POINTER = TablePointer {
                base: self as *const _ as u64,
                limit: (core::mem::size_of::<Self>() - 1) as u16,
            };
            asm!("lidt [{}]", "sti", in(reg) core::ptr::addr_of!(IDT_POINTER));
        }
    }

    pub fn set_entries(&mut self) {
        self.set(Entry::diverging_(interrupt_message!("Divide by zero")), 0);
        self.set(Entry::diverging_(interrupt_message!("bebug")), 1);
        self.set(Entry::ist_index_(NMI_IST, interrupt_message!("non maskable interrupt")), 2);
        self.set(Entry::converging(breakpoint), 3);
        self.set(Entry::diverging_(interrupt_message!("overflow")), 4);
        self.set(Entry::diverging_(interrupt_message!("bound range exceeded")), 5);
        self.set(Entry::diverging_(invalid_opcode), 6);
        self.set(Entry::diverging_(interrupt_message!("device not available")), 7);
        self.set(Entry::ist_index_(DOUBLE_FAULT_IST, interrupt_message!("double fault")), 8);
        self.set(Entry::diverging_(interrupt_message!("coprocessor segment overrun")), 9);
        self.set(Entry::diverging_(interrupt_message!("invalid tss")), 10);
        self.set(Entry::diverging_(interrupt_message!("segment not present")), 11);
        self.set(Entry::diverging_(interrupt_message!("stack segment fault")), 12);
        self.set(Entry::diverging_(interrupt_message!("general protection fault")), 13);
        self.set(Entry::with_error(page_fault), 14);
        self.set(Entry::diverging_(interrupt_message!("reserved")), 15);
        self.set(Entry::diverging_(interrupt_message!("FPU error")), 16);
        self.set(Entry::diverging_(interrupt_message!("alignment check")), 17);
        self.set(Entry::ist_index_(MACHINE_CHECK_IST, interrupt_message!("machine check")), 18);
        self.set(Entry::diverging_(interrupt_message!("simd fp")), 19);
        self.set(Entry::diverging_(interrupt_message!("virtualization")), 20);
        self.set(Entry::diverging_(interrupt_message!("control")), 21);
        self.set(Entry::diverging_(interrupt_message!("reserved")), 22);
        self.set(Entry::diverging_(interrupt_message!("reserved")), 23);
        self.set(Entry::diverging_(interrupt_message!("reserved")), 24);
        self.set(Entry::diverging_(interrupt_message!("reserved")), 25);
        self.set(Entry::diverging_(interrupt_message!("reserved")), 26);
        self.set(Entry::diverging_(interrupt_message!("reserved")), 27);
        self.set(Entry::diverging_(interrupt_message!("reserved")), 28);
        self.set(Entry::diverging_(interrupt_message!("reserved")), 29);
        self.set(Entry::diverging_(interrupt_message!("reserved")), 30);
        self.set(Entry::diverging_(interrupt_message!("reserved")), 31);
        self.set(Entry::diverging_(interrupt_message!("reserved")), 31);

        for i in 32..256 {
            self.set(Entry::diverging_(interrupt_message!("other interrupt")), i);
        }
    }
}

fn byte_to_port(port: u16, byte: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") byte);
    }
}

pub fn init_idt() {
    unsafe {
        IDT.set_entries();

        const PIC1: u16 = 0x20;
        const PIC2: u16 = 0xA0; /* IO base address for slave PIC */
        const PIC1_COMMAND: u16 = PIC1;
        const PIC1_DATA: u16 = PIC1 + 1;
        const PIC2_COMMAND: u16 = PIC2;
        const PIC2_DATA: u16 = PIC2 + 1;

        byte_to_port(PIC1_COMMAND, 0x11);
        byte_to_port(PIC2_COMMAND, 0x11);

        byte_to_port(PIC1_DATA, 0x20);
        byte_to_port(PIC2_DATA, 0x28);

        byte_to_port(PIC1_DATA, 0x04);
        byte_to_port(PIC2_DATA, 0x02);

        byte_to_port(PIC1_DATA, 0x01);
        byte_to_port(PIC2_DATA, 0x01);

        byte_to_port(PIC1_DATA, 0x03);
        byte_to_port(PIC2_DATA, 0x03);

        IDT.load();
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Entry {
    pointer_low: u16,
    gdt_selector: u16,
    options: u16,
    pointer_middle: u16,
    pointer_high: u32,
    reserved: u32,
}

impl Entry {
    fn new_diverging(
        gdt_selector: u16,
        handler: extern "x86-interrupt" fn(stack_frame: ExceptionStackFrame) -> !,
        options: u16,
    ) -> Self {
        let pointer = handler as usize;
        Self {
            gdt_selector,
            pointer_low: (pointer & 0xFFFF) as u16,
            pointer_middle: ((pointer & 0xFFFF0000) >> 16) as u16,
            pointer_high: ((pointer & 0xFFFFFFFF00000000) >> 32) as u32,
            options,
            reserved: 0,
        }
    }

    fn new_converging(gdt_selector: u16, handler: extern "x86-interrupt" fn(_: ExceptionStackFrame), options: u16) -> Self {
        let pointer = handler as usize;
        Self {
            gdt_selector,
            pointer_low: (pointer & 0xFFFF) as u16,
            pointer_middle: ((pointer & 0xFFFF0000) >> 16) as u16,
            pointer_high: ((pointer & 0xFFFFFFFF00000000) >> 32) as u32,
            options,
            reserved: 0,
        }
    }

    fn diverging_(handler: extern "x86-interrupt" fn(_: ExceptionStackFrame) -> !) -> Self {
        Self::new_diverging(0x8, handler, construct_entry_options(0, false, 0, true))
    }

    fn with_error(handler: extern "x86-interrupt" fn(_: ExceptionStackFrame, _: u64) -> !) -> Self {
        let pointer = handler as usize;
        Self {
            gdt_selector: 0x8,
            pointer_low: (pointer & 0xFFFF) as u16,
            pointer_middle: ((pointer & 0xFFFF0000) >> 16) as u16,
            pointer_high: ((pointer & 0xFFFFFFFF00000000) >> 32) as u32,
            options: construct_entry_options(0, false, 0, true),
            reserved: 0,
        }
    }

    fn converging(handler: extern "x86-interrupt" fn(_: ExceptionStackFrame)) -> Self {
        Self::new_converging(0x8, handler, construct_entry_options(0, false, 0, true))
    }

    fn ist_index_(ist_index: u16, handler: extern "x86-interrupt" fn(stack_frame: ExceptionStackFrame) -> !) -> Self {
        Self::new_diverging(0x8, handler, construct_entry_options(ist_index, false, 0, true))
    }

    const fn missing() -> Self {
        Self {
            gdt_selector: 0,
            pointer_high: 0,
            pointer_middle: 0,
            pointer_low: 0,
            options: 0,
            reserved: 0,
        }
    }
}

const fn construct_entry_options(
    interrupt_stack_table_index: u16,
    interrupt_gate: bool,
    descriptor_privilege_level: u16,
    present: bool,
) -> u16 {
    assert!(interrupt_stack_table_index < 8);
    assert!(descriptor_privilege_level < 4);
    let mut num: u16 = 0b0000111000000000 | interrupt_stack_table_index | (descriptor_privilege_level << 13);
    if present {
        num |= 1 << 15;
    }
    if interrupt_gate {
        num |= 1 << 8;
    }
    num
}
