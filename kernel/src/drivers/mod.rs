use core::fmt::Debug;

pub mod ahci;

pub trait PCIDriver: Debug {
    fn class(&self) -> super::pci::device_config::PciClass;
    fn vendor_id(&self) -> Option<u16> {
        None
    }
    fn device_id(&self) -> Option<u16> {
        None
    }
}

pub trait DiskDriver {
    fn read(&self, sector: u64, buffer: &mut [u8]);
    fn write(&self, sector: u64, buffer: &[u8]);
}
