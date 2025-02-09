use std::{boxed::Box, println};

use device_config::MassStorageController;

use crate::interrupts::handlers::{apic_eoi, ExceptionStackFrame};

pub mod device_config;
mod port_access;

pub fn enumerate_devices() {
    let mut devices = port_access::enumerate_devices();
    for device in &mut devices {
        let capabilities = device.get_capabilities_list();
        if !capabilities.iter().any(|cap| cap.id == 5) {
            continue;
        }
        device.init_msi_interrupt();
        if matches!(
            device.get_class(),
            device_config::PciClass::MassStorageController(MassStorageController::SerialATAController)
        ) {
            println!("Device: {:#x?}", device);
            crate::disk::add_pci_disk(device.clone(), Box::new(crate::drivers::ahci::AhciDriver {}));
        }
    }
    crate::disk::print_disks();
}


pub static mut PCI_DEVICE_INTERRUPTS: [(u8, u8, u8); 256] = [(255, 255, 255); 256];

//pci interrupt handler
pub extern "x86-interrupt" fn pci_interrupt(_stack_frame: ExceptionStackFrame) {
    println!("PCI interrupt. HOW THE HELL DO I KNOW WHAT DEVICE THIS IS FOR?");
    apic_eoi();
}
