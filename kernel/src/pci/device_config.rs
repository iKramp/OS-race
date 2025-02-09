#![allow(clippy::enum_variant_names)]

use std::{mem_utils::VirtAddr, println, vec::Vec};

use super::port_access;


#[derive(Debug, Clone)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub capabilities: Vec<Capability>,
}

impl PciDevice {
    pub fn new(bus: u8, device: u8, function: u8) -> Self {
        Self {
            bus,
            device,
            function,
            capabilities: Vec::new(),
        }
    }

    pub fn get_dword(&self, offset: u8) -> u32 {
        port_access::get_dword(self.bus, self.device, self.function, offset)
    }

    pub fn set_dword(&self, offset: u8, value: u32) {
        port_access::set_dword(self.bus, self.device, self.function, offset, value)
    }

    pub fn get_vendor_id(&self) -> u16 {
        self.get_dword(0) as u16
    }

    pub fn get_device_id(&self) -> u16 {
        (self.get_dword(0) >> 16) as u16
    }

    pub fn get_command(&self) -> u16 {
        self.get_dword(4) as u16
    }

    pub fn set_command(&self, value: u16) {
        let dword = self.get_dword(4);
        let dword = (dword & 0xFFFF0000) | value as u32;
        self.set_dword(4, dword);
    }

    pub fn get_status(&self) -> u16 {
        (self.get_dword(4) >> 16) as u16
    }

    pub fn get_revision_id(&self) -> u8 {
        self.get_dword(8) as u8
    }

    pub fn get_progif(&self) -> u8 {
        (self.get_dword(8) >> 8) as u8
    }

    pub fn get_class(&self) -> PciClass {
        let class_subclass = self.get_dword(8) >> 16;
        let class = (class_subclass >> 8) as u8;
        let subclass = class_subclass as u8;
        PciClass::from(class, subclass)
    }

    pub fn get_header_type(&self) -> u8 {
        (self.get_dword(0xC) >> 16) as u8
    }

    pub fn get_bist(&self) -> u8 {
        (self.get_dword(0xC) >> 24) as u8
    }

    pub fn get_latency_timer(&self) -> u8 {
        (self.get_dword(0xC) >> 8) as u8
    }

    pub fn get_cache_line_size(&self) -> u8 {
        self.get_dword(0xC) as u8
    }

    pub fn get_bar(&self, index: u8) -> u32 {
        #[cfg(debug_assertions)]
        {
            let header_type = self.get_header_type() & 0x7F;
            if header_type == 0 {
                assert!(index < 6, "Invalid BAR index for header type 0: {}", index);
            } else if header_type == 1 {
                assert!(index < 2, "Invalid BAR index for header type 1: {}", index);
            } else {
                panic!("Header type {} does not conatin BARs", header_type);
            }
        }
        self.get_dword(0x10 + index * 4)
    }

    pub fn get_capabilities_pointer(&self) -> u8 {
        (self.get_dword(0x34) & 0b11111100) as u8
    }

    pub fn get_capabilities_list(&mut self) -> &Vec<Capability> {
        let status = self.get_status();
        if (status & 0x10) == 0 {
            return &self.capabilities;
        }
        let mut capabilities = Vec::new();
        let mut pointer = self.get_capabilities_pointer();
        while pointer != 0 {
            let capability_first_dword = self.get_dword(pointer);
            let capability_id = capability_first_dword as u8;
            capabilities.push(Capability {
                id: capability_id,
                pointer,
            });
            pointer = (capability_first_dword >> 8) as u8;
        }
        self.capabilities = capabilities;
        return &self.capabilities;
    }

    //MSI functions
    //
    pub fn get_msi_64_bit(&self) -> bool {
        let capabilities = &self.capabilities;
        let msi_cap = capabilities.iter().find(|cap| cap.id == 5).cloned();
        let Some(msi_cap) = msi_cap else {
            panic!("Device does not support MSI");
        };

        let dword = self.get_dword(msi_cap.pointer) >> 16;
        return (dword & 0x80) != 0;
    }
    
    pub fn init_msi_interrupt(&self) {
        let is_64_capable = self.get_msi_64_bit();
        let capabilities = &self.capabilities;
        let msi_cap = capabilities.iter().find(|cap| cap.id == 5).cloned().expect("Device does not support MSI");
        let first_dword = self.get_dword(msi_cap.pointer);
        let mut message_control = (first_dword >> 16) as u16;

        self.set_msi_address(&msi_cap);

        //get number of requested interrupts, and allow max...?
        let requested_interrupts_power = u16::min((message_control & 0b1110) >> 1, 5);
        let requested_interrupts = 1 << requested_interrupts_power;
        println!("Requested interrupts: {}", requested_interrupts);
        message_control &= !0b1110000;
        //give number of vectors
        message_control |= requested_interrupts_power << 4; 

        //get mext available irq with bottom bits set to 0
        let mut current_free_irq = unsafe { crate::interrupts::idt::CUSTOM_INTERRUPT_VECTOR };
        current_free_irq += requested_interrupts - 1;
        current_free_irq &= !(requested_interrupts - 1);

        let data_dword_offset = if is_64_capable { 0xC } else { 0x8 };
        let data_dword = self.get_dword(msi_cap.pointer + data_dword_offset);
        self.set_dword(msi_cap.pointer + data_dword_offset, data_dword & 0xFFFF_0000 | current_free_irq as u32);

        unsafe {
            crate::interrupts::idt::CUSTOM_INTERRUPT_VECTOR = current_free_irq + 1;
        }
        
        //enable MSI
        message_control |= 0x1;

        self.set_dword(msi_cap.pointer, (message_control as u32) << 16 | (first_dword & 0xFFFF));
    }

    fn set_msi_address(&self, msi_cap: &Capability) {
        let platform_info = crate::acpi::get_platform_info();
        let destination_mode = 0;
        let destination_id = platform_info.boot_processor.apic_id as u32;

        let irq_address = 0xFFE << 20 |
            destination_mode << 2 |
            destination_id << 12;

        let low_address = irq_address;
        let high_address = 0;

        self.set_dword(msi_cap.pointer + 4, low_address);
        self.set_dword(msi_cap.pointer + 8, high_address);
    }

}

#[derive(Debug, Clone)]
pub struct Capability {
    pub id: u8,
    pub pointer: u8,
}

#[derive(Debug)]
pub enum PciClass {
    Unclassified(Unclassified),
    MassStorageController(MassStorageController),
    NetworkController(NetworkController),
    DisplayController(DisplayController),
    MultimediaController(MultimediaController),
    MemoryController(MemoryController),
    BridgeDevice(BridgeDevice),
    SimpleCommunicationController(SimpleCommunicationController),
    BaseSystemPeripheral(BaseSystemPeripheral),
    InputDeviceController(InputDeviceController),
    DockingStation,
    Processor(Processor),
    SerialBusController(SerialBusController),
    WirelessController(WirelessController),
    IntelligentController,
    SatelliteCommunicationController(SatelliteCommunicationController),
    EncryptionController(EncryptionController),
    SignalProcessingController(SignalProcessingController),
    ProcessingAccelerator,
    NonEssentialInstrumentation,
    Coprocessor,
}

impl PciClass {
    pub fn from(class: u8, subclass: u8) -> Self {
        match class {
            0x00 => match subclass {
                0x00 => Self::Unclassified(Unclassified::NonVgaCompatibleDevice),
                0x01 => Self::Unclassified(Unclassified::VgaCompatibleDevice),
                _ => panic!("Invalid subclass for class 0x00: {:x}", subclass),
            },
            0x01 => match subclass {
                0x00 => Self::MassStorageController(MassStorageController::SCSIController),
                0x01 => Self::MassStorageController(MassStorageController::IDEController),
                0x02 => Self::MassStorageController(MassStorageController::FloppyDiskController),
                0x03 => Self::MassStorageController(MassStorageController::IPIController),
                0x04 => Self::MassStorageController(MassStorageController::RAIDController),
                0x05 => Self::MassStorageController(MassStorageController::ATAController),
                0x06 => Self::MassStorageController(MassStorageController::SerialATAController),
                0x07 => Self::MassStorageController(MassStorageController::SerialAttachedSCSIController),
                0x08 => Self::MassStorageController(MassStorageController::NonVolatileMemoryController),
                0x80 => Self::MassStorageController(MassStorageController::Other),
                _ => panic!("Invalid subclass for class 0x01: {:x}", subclass),
            },
            0x02 => match subclass {
                0x00 => Self::NetworkController(NetworkController::EthernetController),
                0x01 => Self::NetworkController(NetworkController::TokenRingController),
                0x02 => Self::NetworkController(NetworkController::FDDIController),
                0x03 => Self::NetworkController(NetworkController::ATMController),
                0x04 => Self::NetworkController(NetworkController::ISDNController),
                0x05 => Self::NetworkController(NetworkController::WorldFipController),
                0x06 => Self::NetworkController(NetworkController::PICMGController),
                0x07 => Self::NetworkController(NetworkController::InfinibandController),
                0x08 => Self::NetworkController(NetworkController::FabricController),
                0x80 => Self::NetworkController(NetworkController::Other),
                _ => panic!("Invalid subclass for class 0x02: {:x}", subclass),
            },
            0x03 => match subclass {
                0x00 => Self::DisplayController(DisplayController::VGACompatibleController),
                0x01 => Self::DisplayController(DisplayController::XGAController),
                0x02 => Self::DisplayController(DisplayController::ThreeDController),
                0x80 => Self::DisplayController(DisplayController::Other),
                _ => panic!("Invalid subclass for class 0x03: {:x}", subclass),
            },
            0x04 => match subclass {
                0x00 => Self::MultimediaController(MultimediaController::MultimediaVideoController),
                0x01 => Self::MultimediaController(MultimediaController::MultimediaAudioController),
                0x02 => Self::MultimediaController(MultimediaController::ComputerTelephonyDevice),
                0x03 => Self::MultimediaController(MultimediaController::AudioDevice),
                0x80 => Self::MultimediaController(MultimediaController::Other),
                _ => panic!("Invalid subclass for class 0x04: {:x}", subclass),
            },
            0x05 => match subclass {
                0x00 => Self::MemoryController(MemoryController::RAMController),
                0x01 => Self::MemoryController(MemoryController::FlashController),
                0x80 => Self::MemoryController(MemoryController::Other),
                _ => panic!("Invalid subclass for class 0x05: {:x}", subclass),
            },
            0x06 => match subclass {
                0x00 => Self::BridgeDevice(BridgeDevice::HostBridge),
                0x01 => Self::BridgeDevice(BridgeDevice::ISAbridge),
                0x02 => Self::BridgeDevice(BridgeDevice::EISAbridge),
                0x03 => Self::BridgeDevice(BridgeDevice::MCAbridge),
                0x04 => Self::BridgeDevice(BridgeDevice::PCItoPCIbridge),
                0x05 => Self::BridgeDevice(BridgeDevice::PCMCIAbridge),
                0x06 => Self::BridgeDevice(BridgeDevice::NuBusbridge),
                0x07 => Self::BridgeDevice(BridgeDevice::CardBusbridge),
                0x08 => Self::BridgeDevice(BridgeDevice::RACEwaybridge),
                0x09 => Self::BridgeDevice(BridgeDevice::PCItoPCIbridgeSemiTransparent),
                0x0A => Self::BridgeDevice(BridgeDevice::InfiniBandtoPCIHostBridge),
                0x80 => Self::BridgeDevice(BridgeDevice::Other),
                _ => panic!("Invalid subclass for class 0x06: {:x}", subclass),
            },
            0x07 => match subclass {
                0x00 => Self::SimpleCommunicationController(SimpleCommunicationController::SerialController),
                0x01 => Self::SimpleCommunicationController(SimpleCommunicationController::ParallelController),
                0x02 => Self::SimpleCommunicationController(SimpleCommunicationController::MultiportSerialController),
                0x03 => Self::SimpleCommunicationController(SimpleCommunicationController::Modem),
                0x04 => Self::SimpleCommunicationController(SimpleCommunicationController::GPIBController),
                0x05 => Self::SimpleCommunicationController(SimpleCommunicationController::SmardCardController),
                0x80 => Self::SimpleCommunicationController(SimpleCommunicationController::Other),
                _ => panic!("Invalid subclass for class 0x07: {:x}", subclass),
            },
            0x08 => match subclass {
                0x00 => Self::BaseSystemPeripheral(BaseSystemPeripheral::PIC),
                0x01 => Self::BaseSystemPeripheral(BaseSystemPeripheral::DMAController),
                0x02 => Self::BaseSystemPeripheral(BaseSystemPeripheral::Timer),
                0x03 => Self::BaseSystemPeripheral(BaseSystemPeripheral::RTC),
                0x04 => Self::BaseSystemPeripheral(BaseSystemPeripheral::PCIHotPlugController),
                0x05 => Self::BaseSystemPeripheral(BaseSystemPeripheral::SDHostController),
                0x06 => Self::BaseSystemPeripheral(BaseSystemPeripheral::IOMMU),
                0x80 => Self::BaseSystemPeripheral(BaseSystemPeripheral::Other),
                _ => panic!("Invalid subclass for class 0x08: {:x}", subclass),
            },
            0x09 => match subclass {
                0x00 => Self::InputDeviceController(InputDeviceController::KeyboardController),
                0x01 => Self::InputDeviceController(InputDeviceController::DigitizerPen),
                0x02 => Self::InputDeviceController(InputDeviceController::MouseController),
                0x03 => Self::InputDeviceController(InputDeviceController::ScannerController),
                0x04 => Self::InputDeviceController(InputDeviceController::GameportController),
                0x80 => Self::InputDeviceController(InputDeviceController::Other),
                _ => panic!("Invalid subclass for class 0x09: {:x}", subclass),
            },
            0x0A => Self::DockingStation,
            0x0B => match subclass {
                0x00 => Self::Processor(Processor::i386),
                0x01 => Self::Processor(Processor::i486),
                0x02 => Self::Processor(Processor::Pentium),
                0x10 => Self::Processor(Processor::Alpha),
                0x20 => Self::Processor(Processor::PowerPC),
                0x30 => Self::Processor(Processor::MIPS),
                0x40 => Self::Processor(Processor::CoProcessor),
                0x80 => Self::Processor(Processor::Other),
                _ => panic!("Invalid subclass for class 0x0B: {:x}", subclass),
            },
            0x0C => match subclass {
                0x00 => Self::SerialBusController(SerialBusController::FireWireController),
                0x01 => Self::SerialBusController(SerialBusController::ACCESSBusController),
                0x02 => Self::SerialBusController(SerialBusController::SSA),
                0x03 => Self::SerialBusController(SerialBusController::USBController),
                0x04 => Self::SerialBusController(SerialBusController::FibreChannelController),
                0x05 => Self::SerialBusController(SerialBusController::SMBus),
                0x06 => Self::SerialBusController(SerialBusController::InfiniBandController),
                0x07 => Self::SerialBusController(SerialBusController::IPMIController),
                0x80 => Self::SerialBusController(SerialBusController::Other),
                _ => panic!("Invalid subclass for class 0x0C: {:x}", subclass),
            },
            0x0D => match subclass {
                0x00 => Self::WirelessController(WirelessController::IRController),
                0x01 => Self::WirelessController(WirelessController::ConsumerIRController),
                0x10 => Self::WirelessController(WirelessController::RFController),
                0x11 => Self::WirelessController(WirelessController::BluetoothController),
                0x12 => Self::WirelessController(WirelessController::BroadbandController),
                0x20 => Self::WirelessController(WirelessController::EthernetController),
                0x80 => Self::WirelessController(WirelessController::Other),
                _ => panic!("Invalid subclass for class 0x0D: {:x}", subclass),
            },
            0x0E => Self::IntelligentController,
            0x0F => match subclass {
                0x00 => Self::SatelliteCommunicationController(SatelliteCommunicationController::TVController),
                0x01 => Self::SatelliteCommunicationController(SatelliteCommunicationController::AudioController),
                0x02 => Self::SatelliteCommunicationController(SatelliteCommunicationController::VoiceController),
                0x03 => Self::SatelliteCommunicationController(SatelliteCommunicationController::DataController),
                _ => panic!("Invalid subclass for class 0x0F: {:x}", subclass),
            },
            0x10 => match subclass {
                0x00 => Self::EncryptionController(EncryptionController::NetworkAndComputingEncryptionDevice),
                0x10 => Self::EncryptionController(EncryptionController::EntertainmentEncryptionDevice),
                0x80 => Self::EncryptionController(EncryptionController::Other),
                _ => panic!("Invalid subclass for class 0x10: {:x}", subclass),
            },
            0x11 => match subclass {
                0x00 => Self::SignalProcessingController(SignalProcessingController::DPIOmodule),
                0x01 => Self::SignalProcessingController(SignalProcessingController::PerformanceCounters),
                0x80 => Self::SignalProcessingController(SignalProcessingController::Other),
                _ => panic!("Invalid subclass for class 0x11: {:x}", subclass),
            },
            0x12 => Self::ProcessingAccelerator,
            0x13 => Self::NonEssentialInstrumentation,
            0x40 => Self::Coprocessor,
            _ => panic!("Invalid class: {}", class),
        }
    }
}

#[derive(Debug)]
pub enum Unclassified {
    NonVgaCompatibleDevice,
    VgaCompatibleDevice,
}

#[derive(Debug)]
pub enum MassStorageController {
    SCSIController,
    IDEController,
    FloppyDiskController,
    IPIController,
    RAIDController,
    ATAController,
    SerialATAController,
    SerialAttachedSCSIController,
    NonVolatileMemoryController,
    Other,
}

#[derive(Debug)]
pub enum NetworkController {
    EthernetController,
    TokenRingController,
    FDDIController,
    ATMController,
    ISDNController,
    WorldFipController,
    PICMGController,
    InfinibandController,
    FabricController,
    Other,
}

#[derive(Debug)]
pub enum DisplayController {
    VGACompatibleController,
    XGAController,
    ThreeDController,
    Other,
}

#[derive(Debug)]
pub enum MultimediaController {
    MultimediaVideoController,
    MultimediaAudioController,
    ComputerTelephonyDevice,
    AudioDevice,
    Other,
}

#[derive(Debug)]
pub enum MemoryController {
    RAMController,
    FlashController,
    Other,
}

#[derive(Debug)]
pub enum BridgeDevice {
    HostBridge,
    ISAbridge,
    EISAbridge,
    MCAbridge,
    PCItoPCIbridge,
    PCMCIAbridge,
    NuBusbridge,
    CardBusbridge,
    RACEwaybridge,
    PCItoPCIbridgeSemiTransparent,
    InfiniBandtoPCIHostBridge,
    Other,
}

#[derive(Debug)]
pub enum SimpleCommunicationController {
    SerialController,
    ParallelController,
    MultiportSerialController,
    Modem,
    GPIBController,
    SmardCardController,
    Other,
}

#[derive(Debug)]
pub enum BaseSystemPeripheral {
    PIC,
    DMAController,
    Timer,
    RTC,
    PCIHotPlugController,
    SDHostController,
    IOMMU,
    Other,
}

#[derive(Debug)]
pub enum InputDeviceController {
    KeyboardController,
    DigitizerPen,
    MouseController,
    ScannerController,
    GameportController,
    Other,
}

#[derive(Debug)]
pub enum Processor {
    i386,
    i486,
    Pentium,
    Alpha,
    PowerPC,
    MIPS,
    CoProcessor,
    Other,
}

#[derive(Debug)]
pub enum SerialBusController {
    FireWireController,
    ACCESSBusController,
    SSA,
    USBController,
    FibreChannelController,
    SMBus,
    InfiniBandController,
    IPMIController,
    Other,
}

#[derive(Debug)]
pub enum WirelessController {
    IRController,
    ConsumerIRController,
    RFController,
    BluetoothController,
    BroadbandController,
    EthernetController,
    Other,
}

#[derive(Debug)]
pub enum SatelliteCommunicationController {
    TVController,
    AudioController,
    VoiceController,
    DataController,
}

#[derive(Debug)]
pub enum EncryptionController {
    NetworkAndComputingEncryptionDevice,
    EntertainmentEncryptionDevice,
    Other,
}

#[derive(Debug)]
pub enum SignalProcessingController {
    DPIOmodule,
    PerformanceCounters,
    Other,
}
