const CONFIG_ADDRESS: u16 = 0x0CF8;
const CONFIG_DATA: u16 = 0x0CFC;
use std::{println, Vec};

use crate::utils::{dword_from_port, dword_to_port};

use super::device_config::PciDevice;

pub fn enumerate_devices() -> Vec<PciDevice> {
    let mut devices = Vec::new();
    for bus in 0..=255 {
        for device in 0..32 {
            for function in 0..8 {
                let config_address = get_config_address(true, bus, device, function, 0);
                dword_to_port(CONFIG_ADDRESS, config_address);
                let first_dword = dword_from_port(CONFIG_DATA);
                let vendor_id = first_dword as u16;
                if vendor_id == 0xFFFF {
                    if function == 0 {
                        break;
                    } else {
                        continue;
                    }
                }
                let config_address = get_config_address(true, bus, device, function, 4);
                dword_to_port(CONFIG_ADDRESS, config_address);
                let second_dword = dword_from_port(CONFIG_DATA);

                let config_address = get_config_address(true, bus, device, function, 8);
                dword_to_port(CONFIG_ADDRESS, config_address);
                let third_dword = dword_from_port(CONFIG_DATA);

                let config_address = get_config_address(true, bus, device, function, 12);
                dword_to_port(CONFIG_ADDRESS, config_address);
                let fourth_dword = dword_from_port(CONFIG_DATA);

                let device = PciDevice::new(
                    first_dword as u64 | ((second_dword as u64) << 32),
                    third_dword as u64 | ((fourth_dword as u64) << 32),
                    bus,
                    device,
                    function,
                );
                devices.push(device);
            }
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
