use core::fmt::Debug;
use std::{boxed::Box, println, vec::Vec};


pub trait Disk: Debug {
    fn init(&mut self);
}

static mut DISKS: Vec<Box<dyn Disk>> = Vec::new();

pub fn add_disk(mut disk: Box<dyn Disk>) {
    disk.init();
    unsafe {
        DISKS.push(disk);
    }
}

pub fn print_disks() {
    unsafe {
        for disk in DISKS.iter() {
            println!("Disk: {:#x?}", disk);
        }
    }
}
