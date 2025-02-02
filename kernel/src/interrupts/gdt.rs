use super::idt::TablePointer;

pub const DOUBLE_FAULT_IST: u16 = 1;
pub const NMI_IST: u16 = 2;
pub const MACHINE_CHECK_IST: u16 = 3;

const GDT_LEN: usize = 7;
#[used]
pub static mut GDT_POINTER: TablePointer = TablePointer { limit: 0, base: 0 };

#[repr(C, packed)]
struct TaskStateSegment {
    padding_1: u32,
    privilege_stack_table: [u64; 3],
    padding_2: u64,
    interrupt_stack_table: [u64; 7],
    padding_3: u64,
    padding_4: u16,
    io_map_base_address: u16,
}

const STACK_SIZE: usize = 4096 * 5;

//wrapped to align
#[repr(align(16))]
struct Ist {
    #[allow(unused)] //used
    stack: [u8; STACK_SIZE],
}

#[used]
static mut TSS: TaskStateSegment = TaskStateSegment {
    padding_1: 0,
    privilege_stack_table: [0; 3],
    padding_2: 0,
    interrupt_stack_table: [0; 7],
    padding_3: 0,
    padding_4: 0,
    io_map_base_address: core::mem::size_of::<TaskStateSegment>() as u16,
};

fn init_tss() {
    unsafe {
        TSS.interrupt_stack_table[DOUBLE_FAULT_IST as usize - 1] = {
            #[used]
            static mut STACK: Ist = Ist { stack: [0; STACK_SIZE] };
            core::ptr::addr_of!(STACK) as u64 + STACK_SIZE as u64
        };
        TSS.interrupt_stack_table[NMI_IST as usize - 1] = {
            #[used]
            static mut STACK: Ist = Ist { stack: [0; STACK_SIZE] };
            core::ptr::addr_of!(STACK) as u64 + STACK_SIZE as u64
        };
        TSS.interrupt_stack_table[MACHINE_CHECK_IST as usize - 1] = {
            #[used]
            static mut STACK: Ist = Ist { stack: [0; STACK_SIZE] };
            core::ptr::addr_of!(STACK) as u64 + STACK_SIZE as u64
        };
    }
}

#[derive(Debug)]
#[repr(C, align(8))]
struct GlobalDescriptorTable {
    table: [SegmentDescriptor; GDT_LEN],
    len: usize,
}

#[used]
static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable {
    table: [
        create_segment_descriptor(0, 0, 0, 0),
        create_segment_descriptor(0, 0xFFFFF, 0x9A, 0xA),
        create_segment_descriptor(0, 0xFFFFF, 0x92, 0xC),
        create_segment_descriptor(0, 0xFFFFF, 0xFB, 0xA),
        create_segment_descriptor(0, 0xFFFFF, 0xF2, 0xC),
        create_segment_descriptor(0, 0x0, 0x0, 0x0),
        create_segment_descriptor(0, 0x0, 0x0, 0x0),
    ],
    len: 5,
};

impl GlobalDescriptorTable {
    fn load(&'static self) {
        unsafe {
            GDT_POINTER = self.pointer();
            core::arch::asm!("lgdt [{}]", in(reg) core::ptr::addr_of!(GDT_POINTER), options(readonly, nostack, preserves_flags));
        }
    }

    fn pointer(&self) -> TablePointer {
        TablePointer {
            limit: (GDT_LEN * 8 - 1) as u16,
            base: self.table.as_ptr() as u64,
        }
    }

    fn append_128(&mut self, data: (SegmentDescriptor, SegmentDescriptor)) -> usize {
        if self.len >= GDT_LEN - 1 {
            panic!("gdt size exceeded");
        }
        self.table[self.len] = data.0;
        self.len += 1;
        self.table[self.len] = data.1;
        self.len += 1;
        (self.len - 2) << 3
    }
}

#[derive(Debug)]
#[repr(C, packed)]
struct SegmentDescriptor {
    limit_low: u16,
    base_low: u16,
    base_mid: u8,
    access_byte: u8,
    lim_h_flags: u8,
    base_high: u8,
}

const fn create_128_segment_descriptor(base: u64, limit: u32, access_byte: u8, flags: u8) -> (SegmentDescriptor, SegmentDescriptor) {
    let low = create_segment_descriptor(base, limit, access_byte, flags);
    let high = create_segment_descriptor((base >> 48) & 0xFFFF, ((base >> 32) & 0xFFFF) as u32, 0, 0); //a bit of a hack, we're actually
                                                                                                       //doing a 32 bit base
    (low, high)
}

const fn create_segment_descriptor(base: u64, limit: u32, access_byte: u8, flags: u8) -> SegmentDescriptor {
    SegmentDescriptor {
        limit_low: (limit & 0xFFFF) as u16,
        base_low: (base & 0xFFFF) as u16,
        base_mid: ((base & 0xFF0000) >> 16) as u8,
        access_byte,
        lim_h_flags: ((limit & 0xF0000) >> 16) as u8 | ((flags & 0xF) << 4),
        base_high: ((base & 0xFF000000) >> 24) as u8,
    }
}

pub fn init_gdt() {
    init_tss();
    unsafe {
        GDT.append_128(create_128_segment_descriptor(
            core::ptr::addr_of!(TSS) as u64,
            (core::mem::size_of::<TaskStateSegment>() - 1) as u32,
            0x89,
            0x0,
        ));
    };

    unsafe {
        GDT.load();
        set_cs();
        core::arch::asm!("mov ax, 0x28", "ltr ax", out("ax") _, options(nostack, preserves_flags, raw));
    }
}

pub fn set_cs() {
    unsafe {
        core::arch::asm!(
            "push 0x08", //code segment
            "lea rax, [rip + 2f]", //load the address of the label
            "push rax", //push the address of the label
            "retfq", //return far qword

            "2:", //label
            "mov ax, 0x10", //data segment
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",
            out("rax") _,
            options(nostack, preserves_flags, raw)
        )
    }
}
