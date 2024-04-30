use crate::println;

use super::idt::TablePointer;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 1;
const GDT_LEN: usize = 8;

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

static mut TSS: TaskStateSegment = TaskStateSegment {
    padding_1: 0,
    privilege_stack_table: [0; 3],
    padding_2: 0,
    interrupt_stack_table: [0; 7],
    padding_3: 0,
    padding_4: 0,
    io_map_base_address: 0,
};

pub fn init_tss() {
    unsafe {
        TSS.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = core::ptr::addr_of!(STACK) as u64;
            stack_start + STACK_SIZE as u64
        }
    }
}

#[derive(Debug)]
#[repr(C, align(8))]
pub struct GlobalDescriptorTable {
    table: [u64; GDT_LEN],
    len: usize,
}

pub struct Selectors {
    pub kernel_code_selector: u16,
    kernel_data_selector: u16,
    user_code_selector: u16,
    user_data_selector: u16,
    tss_selector: u16,
}

static mut GDT_POINTER: TablePointer = TablePointer { limit: 0, base: 0 };

pub static mut GDT: (GlobalDescriptorTable, Selectors) = (
    GlobalDescriptorTable {
        table: [0; GDT_LEN],
        len: 1,
    },
    Selectors {
        kernel_code_selector: 0,
        kernel_data_selector: 0,
        user_code_selector: 0,
        user_data_selector: 0,
        tss_selector: 0,
    },
);

impl GlobalDescriptorTable {
    fn load(&'static self) {
        unsafe {
            GDT_POINTER = self.pointer();
        }
        unsafe {
            core::arch::asm!("lgdt [{}]", in(reg) core::ptr::addr_of!(GDT_POINTER), options(readonly, nostack, preserves_flags));
        }
    }

    fn pointer(&self) -> TablePointer {
        TablePointer {
            limit: (self.len * 8 - 1) as u16,
            base: self.table.as_ptr() as u64,
        }
    }
    fn append_64(&mut self, data: u64) -> usize {
        if self.len >= GDT_LEN {
            panic!("gdt size exceeded");
        }
        self.table[self.len] = data;
        self.len += 1;
        (self.len - 1) << 3
    }

    fn append_128(&mut self, data: (u64, u64)) -> usize {
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

#[repr(C, packed)]
struct SegmentDescriptor {
    limit_low: u16,
    base_low: u16,
    base_mid: u8,
    access_byte: u8,
    lim_h_flags: u8,
    base_high: u8,
}

fn create_128_segment_descriptor(base: u64, limit: u32, access_byte: u8, flags: u8) -> (u64, u64) {
    let low = create_segment_descriptor(base, limit, access_byte, flags);
    let high = (base & 0xFFFFFFFF00000000) >> 32;
    (low, high)
}

fn create_segment_descriptor(base: u64, limit: u32, access_byte: u8, flags: u8) -> u64 {
    let descriptor = SegmentDescriptor {
        limit_low: (limit & 0xFFFF) as u16,
        base_low: (base & 0xFFFF) as u16,
        base_mid: ((base & 0xFF0000) >> 16) as u8,
        access_byte,
        lim_h_flags: ((limit & 0xF0000) >> 16) as u8 | ((flags & 0xF) << 4),
        base_high: ((base & 0xFF000000) >> 24) as u8,
    };

    unsafe { core::mem::transmute(descriptor) }
}

pub fn init_gdt() {
    unsafe {
        let ptr = core::ptr::addr_of!(TSS) as *const _ as u64;
        GDT.1.kernel_code_selector = GDT.0.append_64(create_segment_descriptor(0, 0xFFFFF, 0x0A, 0xA)) as u16;
        GDT.1.kernel_data_selector = GDT.0.append_64(create_segment_descriptor(0, 0xFFFFF, 0x92, 0xC)) as u16;
        //will add the others later when loading gdt works
        //GDT.1.user_code_selector = GDT.0.append_64(create_segment_descriptor(0, 0xFFFFF, 0xFA, 0xA)) as u16;
        //GDT.1.user_data_selector = GDT.0.append_64(create_segment_descriptor(0, 0xFFFFF, 0xF2, 0xC)) as u16;
        /*GDT.1.tss_selector = GDT.0.append_128(create_128_segment_descriptor(
            ptr,
            (size_of::<TaskStateSegment>() - 1) as u32,
            0x89,
            0x0,
        )) as u16;*/
    };

    unsafe {
        GDT.0.load();
        //can probably remain commented because it has the same offset as the bootloader defined cs
        set_cs(GDT.1.kernel_code_selector, GDT.1.kernel_data_selector); //TODO implement later
        core::arch::asm!("ltr {0:x}", in(reg) GDT.1.tss_selector, options(nostack, preserves_flags));
    }
}

fn set_cs(code_seg: u16, data_seg: u16) {
    unsafe {
        core::arch::asm!(
            "push {code_seg}",
            "lea {tmp}, [2f + rip]",
            "push {tmp}",
            "retfq",
            "2:",
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",
            code_seg = in(reg) u64::from(code_seg),
            in("ax") u64::from(data_seg),
            tmp = lateout(reg) _,
            options(preserves_flags),
        );
    }
}
