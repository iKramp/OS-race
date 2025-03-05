use core::fmt::Debug;

pub mod ahci;
pub mod gpt;
pub mod disk;
pub mod rfs;

pub trait PCIDriver: Debug {
    fn class(&self) -> super::pci::device_config::PciClass;
    fn vendor_id(&self) -> Option<u16> {
        None
    }
    fn device_id(&self) -> Option<u16> {
        None
    }
}
