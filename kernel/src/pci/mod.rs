use std::{boxed::Box, println, printlnc};

use device_config::{MassStorageController, RegularPciDevice};

use crate::{drivers::ahci::GenericHostControl, interrupts::handlers::{apic_eoi, ExceptionStackFrame}};

pub mod device_config;
mod port_access;

pub fn enumerate_devices() {
    let mut devices = port_access::enumerate_devices();
    for device in &mut devices {
        let class = device.get_class();
        let capabilities = device.get_capabilities_list();
        if !capabilities.iter().any(|cap| cap.id == 5) {
            continue;
        }
        printlnc!((51, 153, 10), "configuring pci device: {:#x?}", class);

        device.init_msi_interrupt();
        let device = RegularPciDevice::new(device.clone());
        
        if matches!(
            class,
            device_config::PciClass::MassStorageController(MassStorageController::SerialATAController)
        ) {
            let ahci_disk = crate::drivers::ahci::AhciDisk::new(device);
            crate::disk::add_disk(Box::new(ahci_disk));
        }
        printlnc!((51, 153, 10), "Device configured");
    }
}


pub static mut PCI_DEVICE_INTERRUPTS: [(u8, u8, u8); 256] = [(255, 255, 255); 256];

//pci interrupt handler
pub extern "x86-interrupt" fn pci_interrupt(_stack_frame: ExceptionStackFrame) {
    println!("PCI interrupt. HOW THE HELL DO I KNOW WHAT DEVICE THIS IS FOR?");
    apic_eoi();
}
