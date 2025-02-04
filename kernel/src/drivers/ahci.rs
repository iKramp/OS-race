use super::{DiskDriver, PCIDriver};
use crate::pci::device_config;


#[derive(Debug, Clone)]
pub struct AhciDriver {

}

impl PCIDriver for AhciDriver {
    fn class(&self) -> device_config::PciClass {
        device_config::PciClass::MassStorageController(device_config::MassStorageController::SerialATAController)
    }
}

impl DiskDriver for AhciDriver {
    fn read(&self, sector: u64, buffer: &mut [u8]) {
        todo!()
    }

    fn write(&self, sector: u64, buffer: &[u8]) {
        todo!()
    }
}
