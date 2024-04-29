use core::mem::size_of;

use super::idt::TablePointer;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
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

pub fn init_tts() {
    unsafe {
        TSS.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = &STACK as *const _ as u64;
            stack_start + STACK_SIZE as u64
        }
    }
}

struct GlobalDescriptorTable {
    table: [u64; GDT_LEN],
    len: usize,
}

impl GlobalDescriptorTable {
    fn append_64(&mut self, data: u64) -> usize {
        if self.len >= GDT_LEN {
            panic!("gdt size exceeded");
        }
        self.table[self.len] = data;
        self.len += 1;
        self.len - 1
    }

    fn append_128(&mut self, data_low: u64, data_high: u64) -> usize {
        if self.len >= GDT_LEN - 1 {
            panic!("gdt size exceeded");
        }
        self.table[self.len] = data_low;
        self.len += 1;
        self.table[self.len] = data_high;
        self.len += 1;
        self.len - 2
    }
}

static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable {
    table: [0; GDT_LEN],
    len: 1,
};

impl GlobalDescriptorTable {
    fn load(&'static self) {
        let gdt = &self.pointer();
        unsafe {
            core::arch::asm!("lgdt [{}]", in(reg) gdt, options(readonly, nostack, preserves_flags));
        }
    }

    fn pointer(&self) -> TablePointer {
        use core::mem::size_of;
        TablePointer {
            base: self.table.as_ptr() as u64,
            limit: (self.len * size_of::<u64>() - 1) as u16,
        }
    }
}

pub fn init_gdt() {
    const COMMON: u64 = 1 << 44 | 1 << 47 | 1 << 41 | 1 << 40 | 0xFFFF | 0xF << 48 | 1 << 55;
    unsafe {
        GDT.append_64(COMMON | 1 << 43 | 1 << 53); //adding kernel code descriptor, will
                                                   //probably break everthing
        let ptr = core::ptr::addr_of!(TSS) as *const _ as u64;
        let mut low = 1 << 47;
        low |= (ptr & 0xFFFFFF) << 16;
        low |= (ptr & 0xFF000000) << (56 - 24);
        low |= ((size_of::<TaskStateSegment>() - 1) as u64) & 0xFFFF;
        low |= 0b1001 << 40;

        let high = (ptr & 0xFFFFFFFF00000000) >> 32;

        GDT.append_128(low, high);
    };

    unsafe { GDT.load() };
}
