const CONFIG_ADDRESS: u16 = 0x0CF8;
const CONFIG_DATA: u16 = 0x0CFC;
use std::Vec;

use crate::utils::{dword_from_port, dword_to_port};

use super::device_config::PciDevice;

pub fn enumerate_devices() -> Vec<PciDevice> {
    let mut devices = Vec::new();
    for bus in 0..=255 {
        for device in 0..32 {
            let mut functions = 0;
            for function in 0..8 {
                let first_dword = get_dword(bus, device, function, 0);
                let vendor_id = first_dword as u16;
                if vendor_id == 0xFFFF && function == 0 {
                    break;
                }
                functions |= 1 << function;
                if PciDevice::new(bus, device, 0).get_header_type() & 0x80 == 0 {
                    break;
                }
            }
            if functions == 0 {
                continue;
            }
            let device = PciDevice::new(bus, device, functions);
            devices.push(device);
        }
    }
    devices
}

fn get_config_address(enable: bool, bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    debug_assert!(offset & 0b11 == 0);
    debug_assert!(function < 8);
    debug_assert!(device < 32);
    (if enable { 1 } else { 0 } << 31)
        | (bus as u32) << 16
        | ((device & 0x1F) as u32) << 11
        | ((function & 0b111) as u32) << 8
        | (offset as u32)
}

pub fn get_dword(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let config_address = get_config_address(true, bus, device, function, offset);
    dword_to_port(CONFIG_ADDRESS, config_address);
    dword_from_port(CONFIG_DATA)
}

pub fn set_dword(bus: u8, device: u8, function: u8, offset: u8, value: u32) {
    let config_address = get_config_address(true, bus, device, function, offset);
    dword_to_port(CONFIG_ADDRESS, config_address);
    dword_to_port(CONFIG_DATA, value);
}
