use std::{boxed::Box, println};

use device_config::MassStorageController;

pub mod device_config;
mod port_access;

pub fn enumerate_devices() {
    let devices = port_access::enumerate_devices();
    for device in devices {
        if matches!(device.get_class(), device_config::PciClass::MassStorageController(MassStorageController::SerialATAController)) {
            println!("Device: {:#x?}", device);
            println!("Capabilities: {:#x?}", device.get_capabilities_list());
            crate::disk::add_pci_disk(device, Box::new(crate::drivers::ahci::AhciDriver {}));
        }
    }
    crate::disk::print_disks();
}
