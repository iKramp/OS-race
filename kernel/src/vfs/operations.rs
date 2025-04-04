use std::{boxed::Box, format, mem_utils::PhysAddr, println, string::{String, ToString}, vec::Vec};

use uuid::Uuid;

use crate::drivers::{disk::{DirEntry, Disk, MountedPartition, PartitionSchemeDriver}, gpt::GPTDriver};

use super::{fs_tree, resolve_path, DeviceDetails, InodeType, ResolvedPath, ROOT_INODE_INDEX, VFS};


pub fn add_disk(mut disk: Box<dyn Disk + Send>) {
    //for now only GPT
    let gpt_driver = GPTDriver {};
    let guid = gpt_driver.guid(&mut *disk);
    let partitions = gpt_driver.partitions(&mut *disk);
    let partition_guids: Vec<Uuid> = partitions.iter().map(|(guid, _)| *guid).collect();

    let mut vfs = VFS.lock();

    vfs.disks.insert(guid, (disk, partition_guids));

    for partition in partitions {
        let device = partition.1.device.clone();
        vfs.available_partitions.insert(partition.0, partition.1);
        vfs.devices.insert(
            device,
            DeviceDetails {
                drive: guid,
                partition: partition.0,
            },
        );
    }
}

//called after unmounting all partitions or when it was forcibly removed
fn remove_disk(uuid: Uuid) {
    let mut vfs = VFS.lock();
    for partition in vfs.disks.remove(&uuid).unwrap().1.iter() {
        vfs.available_partitions.remove(partition);
    }
}

pub fn mount_partition_working_dir(part_id: Uuid, mountpoint: &str, working_dir: &str) -> Result<(), String> {
    let mountpoint = resolve_path(mountpoint, working_dir);
    mount_partition_resolved(part_id, mountpoint)
}

pub fn mount_partition(part_id: Uuid, mountpoint: &str) -> Result<(), String> {
    mount_partition_working_dir(part_id, mountpoint, "/")
}

pub fn mount_partition_resolved(part_id: Uuid, mountpoint: ResolvedPath) -> Result<(), String> {
    let mut vfs = VFS.lock();
    let Some(partition) = vfs.available_partitions.get(&part_id) else {
        return Err("Partition not found".to_string());
    };
    let partition = partition.clone();

    let device_detail = vfs.devices.get(&partition.device).unwrap();
    let drive_id = device_detail.drive;
    let disk = vfs.disks.get_mut(&drive_id).unwrap();
    let disk = &raw mut *disk.0;
    let disk: &'static mut dyn Disk = unsafe { &mut *disk };
    let Some(fs_factory) = vfs.filesystem_driver_factories.get(&partition.fs_uuid) else {
        return Err(format!(
            "No filesystem driver loaded for \n
                partition type: {}, \n
                partition: {}",
            partition.fs_uuid, part_id
        )
        .to_string());
    };

    let current_num = fs_tree::CURRENT_NUM.load(core::sync::atomic::Ordering::Relaxed);

    //mounting root. This is the first FS cache operation and can only happen once per boot
    if mountpoint.0.is_empty() && current_num != 0 {
        return Err("Root already mounted".to_string());
    }

    if !mountpoint.0.is_empty() {
        panic!("mounting non-root not implemented yet");
    }

    let mounted_partition = MountedPartition { disk, partition };
    let mut fs = fs_factory.mount(mounted_partition);
    let inode = fs.stat(ROOT_INODE_INDEX);
    fs_tree::init(inode);
    vfs.mounted_partitions.insert(part_id, fs);

    
    Ok(())
}

pub fn unmount_partition(part_id: Uuid) {
    let mut vfs = VFS.lock();
    let mut partition = vfs.mounted_partitions.remove(&part_id).unwrap();
    partition.unmount();
}

pub fn get_dir_entries(path: ResolvedPath) -> Result<Box<[DirEntry]>, String> {
    let inode_num = fs_tree::get_inode_num(path).ok_or("Path not found")?;
    let inode = fs_tree::get_inode(inode_num).ok_or("Inode not found")?;
    let mut vfs = VFS.lock();
    let device_details = vfs.devices.get(&inode.device).ok_or("Device not found")?;
    let partition_id = device_details.partition;
    let fs = vfs.mounted_partitions.get_mut(&partition_id).ok_or("FS not found")?;
    Ok(fs.read_dir(&inode))
}

pub fn create_file(path: ResolvedPath, name: &str, inode_type: InodeType) {
    let parent_inode_num = fs_tree::get_inode_num(path).unwrap();
    let parent_inode = fs_tree::get_inode(parent_inode_num).unwrap();
    let mut vfs = VFS.lock();
    let device_details = vfs.devices.get(&parent_inode.device).unwrap();
    let partition_id = device_details.partition;
    let fs = vfs.mounted_partitions.get_mut(&partition_id).unwrap();
    let (file_inode, parent_inode) = fs.create(name, parent_inode.index, inode_type, 0, 0);
    fs_tree::update_inode(parent_inode_num, parent_inode);
    fs_tree::insert_inode(parent_inode_num, name.to_string().into_boxed_str(), file_inode);
}

pub fn write_file(path: ResolvedPath, content: &[PhysAddr], offset: u64, size: u64) {
    let inode_num = fs_tree::get_inode_num(path).unwrap();
    let inode = fs_tree::get_inode(inode_num).unwrap();
    let mut vfs = VFS.lock();
    let device_details = vfs.devices.get(&inode.device).unwrap();
    let partition_id = device_details.partition;
    let fs = vfs.mounted_partitions.get_mut(&partition_id).unwrap();
    fs.write(inode.index, offset, size, content);
}

pub fn read_file(path: ResolvedPath, buffer: &[PhysAddr], offset: u64, size: u64) {
    let inode_num = fs_tree::get_inode_num(path).unwrap();
    let inode = fs_tree::get_inode(inode_num).unwrap();
    let mut vfs = VFS.lock();
    let device_details = vfs.devices.get(&inode.device).unwrap();
    let partition_id = device_details.partition;
    let fs = vfs.mounted_partitions.get_mut(&partition_id).unwrap();
    fs.read(inode.index, offset, size, buffer);
}
