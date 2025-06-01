use std::{boxed::Box, println, printlnc};

use device_config::{MassStorageController, RegularPciDevice};

use crate::{
    drivers::ahci::disk::AhciController,
    interrupts::{InterruptProcessorState, handlers::apic_eoi},
};

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
            let mut ahci_disk = AhciController::new(device);
            let ports = ahci_disk.init();
            for port in ports {
                crate::vfs::add_disk(Box::new(port));
            }
        }
        printlnc!((51, 153, 10), "Device configured");
    }
}

pub static mut PCI_DEVICE_INTERRUPTS: [(u8, u8, u8); 256] = [(255, 255, 255); 256];

//pci interrupt handler
pub extern "C" fn pci_interrupt(_proc_data: &mut InterruptProcessorState) {
    println!("PCI interrupt. HOW THE HELL DO I KNOW WHAT DEVICE THIS IS FOR?");
    apic_eoi();
}
