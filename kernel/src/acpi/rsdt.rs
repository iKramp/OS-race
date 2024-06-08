use std::mem_utils::*;

pub trait RootSystemDescriptorTable: std::fmt::Debug {
    fn validate(&self) -> bool {
        let addr = self as *const Self as *const u8 as u64;
        let mut sum: u16 = 0;
        for i in 0..self.length() {
            unsafe {
                sum += *get_at_virtual_addr::<u8>(VirtAddr(addr + i as u64)) as u16;
            }
        }
        sum & 0xFF == 0
    }

    fn length(&self) -> u32;
    fn get_table(&self, signature: [u8; 4]) -> Option<PhysAddr>;
    fn print_tables(&self);
    fn print_signature(&self);
}

pub fn get_rsdt(rsdp: &super::rsdp::Rsdp) -> &'static dyn RootSystemDescriptorTable {
    unsafe {
        let address = rsdp.address();
        match rsdp {
            super::rsdp::Rsdp::V1(_) => get_at_physical_addr::<Rsdt>(address),
            super::rsdp::Rsdp::V2(_) => get_at_physical_addr::<Xsdt>(address),
        }
    }
}

#[repr(C)]
#[derive(Debug)]
struct Rsdt {
    header: super::sdt::AcpiSdtHeader,
}

impl RootSystemDescriptorTable for Rsdt {
    fn length(&self) -> u32 {
        self.header.length
    }
    fn get_table(&self, signature: [u8; 4]) -> Option<PhysAddr> {
        unsafe {
            let start_table_ptr = VirtAddr((self as *const Self) as u64 + 36);
            let num_entries = (self.length() - 36) / 4;
            for entry_index in 0..num_entries {
                let table_entry_ptr = start_table_ptr + VirtAddr(entry_index as u64 * 4);
                let table_ptr = PhysAddr(*get_at_virtual_addr::<u32>(table_entry_ptr) as u64);
                let header = get_at_physical_addr::<super::sdt::AcpiSdtHeader>(table_ptr);
                if header.signature == signature {
                    return Some(table_ptr);
                }
            }
            None
        }
    }
    fn print_tables(&self) {
        unsafe {
            let start_table_ptr = VirtAddr((self as *const Self) as u64 + 36);
            let num_entries = (self.length() - 36) / 4;
            for entry_index in 0..num_entries {
                let table_entry_ptr = start_table_ptr + VirtAddr(entry_index as u64 * 4);
                let table_ptr = PhysAddr(*get_at_virtual_addr::<u32>(table_entry_ptr) as u64);
                let header = get_at_physical_addr::<super::sdt::AcpiSdtHeader>(table_ptr);
                crate::println!("{:?}", header.signature.map(|a| a as char))
            }
        }
    }
    fn print_signature(&self) {
        crate::println!("{:?}", self.header.signature.map(|a| a as char))
    }
}

#[repr(C)]
#[derive(Debug)]
struct Xsdt {
    header: super::sdt::AcpiSdtHeader,
}

impl RootSystemDescriptorTable for Xsdt {
    fn length(&self) -> u32 {
        self.header.length
    }
    fn get_table(&self, signature: [u8; 4]) -> Option<PhysAddr> {
        unsafe {
            let start_table_ptr = VirtAddr((self as *const Self) as u64 + 36);
            let num_entries = (self.length() - 36) / 8;
            for entry_index in 0..num_entries {
                let table_entry_ptr = start_table_ptr + VirtAddr(entry_index as u64 * 8);
                let table_ptr = PhysAddr(*get_at_virtual_addr::<u64>(table_entry_ptr));
                let header = get_at_physical_addr::<super::sdt::AcpiSdtHeader>(table_ptr);
                if header.signature == signature {
                    return Some(table_ptr);
                }
            }
            None
        }
    }
    fn print_tables(&self) {
        unsafe {
            let start_table_ptr = VirtAddr((self as *const Self) as u64 + 36);
            let num_entries = (self.length() - 36) / 8;
            for entry_index in 0..num_entries {
                let table_entry_ptr = start_table_ptr + VirtAddr(entry_index as u64 * 8);
                let table_ptr = PhysAddr(*get_at_virtual_addr::<u64>(table_entry_ptr));
                let header = get_at_physical_addr::<super::sdt::AcpiSdtHeader>(table_ptr);
                crate::println!("{:?}", header.signature.map(|a| a as char))
            }
        }
    }
    fn print_signature(&self) {
        crate::println!("{:?}", self.header.signature.map(|a| a as char))
    }
}
