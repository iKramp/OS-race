use super::{DiskDriver, PCIDriver};
use crate::pci::device_config;

enum FisType {
    RegisterH2D = 0x27,
    RegisterD2H = 0x34,
    DMAActivate = 0x39,
    DMASetup = 0x41,
    Data = 0x46,
    BIST = 0x58,
    PIOSetup = 0x5F,
    SetDeviceBits = 0xA1,
}

#[repr(C, packed)]
struct DataFis {
    fis_type: FisType,
    port_multiplier: u8,
    reserved: [u8; 2],
    data: [u8],
}


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
