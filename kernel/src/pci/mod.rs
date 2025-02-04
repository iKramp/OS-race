use std::println;

mod device_config;
mod port_access;

pub fn enumerate_devices() {
    let devices = port_access::enumerate_devices();
    for device in devices {
        println!("{:x?}", device);
    }
}
