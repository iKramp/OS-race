use std::{boxed::Box, println, vec::Vec};

use crate::drivers::DiskDriver;


pub struct PciDisk {
    device: crate::pci::device_config::PciDevice,
    driver: Box<dyn DiskDriver>,
}

static mut PCI_DISKS: Vec<PciDisk> = Vec::new();

pub fn add_pci_disk(device: crate::pci::device_config::PciDevice, driver: Box<dyn DiskDriver>) {
    unsafe {
        PCI_DISKS.push(PciDisk { device, driver });
    }
}

pub fn print_disks() {
    unsafe {
        for disk in PCI_DISKS.iter() {
            println!("Disk: {:#x?}", disk.device);
        }
    }
}
