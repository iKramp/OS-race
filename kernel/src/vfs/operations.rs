use std::{
    boxed::Box,
    format,
    mem_utils::PhysAddr,
    string::{String, ToString},
    vec::Vec,
};

use uuid::Uuid;

use crate::drivers::{
    disk::{BlockDevice, DirEntry, MountedPartition, PartitionSchemeDriver},
    gpt::GPTDriver,
};

use super::{
    DeviceDetails, InodeType, ROOT_INODE_INDEX, ResolvedPath, ResolvedPathBorrowed, VFS,
    filesystem_trait::FileSystem,
    fs_tree::{self},
};

pub async fn add_disk(mut disk: Box<dyn BlockDevice + Send>) {
    //for now only GPT
    let gpt_driver = GPTDriver {};
    let guid = gpt_driver.guid(&mut *disk).await;
    let partitions = gpt_driver.partitions(&mut *disk).await;
    let partition_guids: Vec<Uuid> = partitions.iter().map(|(guid, _)| *guid).collect();

    let mut vfs = VFS.lock();

    vfs.disks.insert(guid, (disk, partition_guids));

    for partition in partitions {
        let device = partition.1.device;
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

pub async fn mount_blkdev_partition(part_id: Uuid, mountpoint: ResolvedPath) -> Result<(), String> {
    if !mountpoint.inner().is_empty() {
        let parent_path = mountpoint.index(0..mountpoint.inner().len() - 1);
        let parent = fs_tree::get_inode_index(parent_path).await.ok_or("Invalid mountpoint")?;
        let mountpoint = fs_tree::get_inode_index((&mountpoint).into()).await.ok_or("Invalid mountpoint")?;
        if parent.device_id != mountpoint.device_id {
            return Err("Mountpoint already used".to_string());
        }
    }

    let mut vfs = VFS.lock();
    let Some(partition) = vfs.available_partitions.get(&part_id) else {
        return Err("Partition not found".to_string());
    };
    let partition = partition.clone();

    let device_detail = vfs.devices.get(&partition.device).unwrap();
    let drive_id = device_detail.drive;
    let Some(disk) = vfs.disks.get_mut(&drive_id) else {
        return Err("Disk not found".to_string());
    };
    let disk = &raw mut *disk.0;
    let cloned_disk: &'static mut dyn BlockDevice = unsafe { &mut *disk };

    let Some(fs_factory) = vfs.filesystem_driver_factories.get(&partition.fs_type) else {
        return Err(format!(
            "No filesystem driver loaded for \n
                partition type: {}, \n
                partition: {}",
            partition.fs_type, part_id
        )
        .to_string());
    };

    let mounted_partition = MountedPartition {
        disk: cloned_disk,
        partition,
    };
    let mut fs = fs_factory.mount(mounted_partition).await;
    if let Err(e) = mount_filesystem(mountpoint, &mut fs).await {
        fs.unmount().await;
        Err(e)
    } else {
        vfs.mounted_partitions.insert(part_id, fs);
        Ok(())
    }
}

async fn mount_filesystem(mountpoint: ResolvedPath, fs: &mut Box<dyn FileSystem + Send>) -> Result<(), String> {
    if mountpoint.inner().is_empty() {
        //mounting root
        mount_new_root(fs).await;
        //anything else?
    }

    let fs_root_inode = fs.stat(ROOT_INODE_INDEX).await;
    //we disallow the mounting of root failing so no checks :3
    let parent_inode = fs_tree::get_inode_index((&mountpoint).into()).await.ok_or("mountpoint not found")?;
    fs_tree::mount_inode(parent_inode, fs_root_inode);

    Ok(())
}

async fn mount_new_root(fs: &mut Box<dyn FileSystem + Send>) {
    let inode = fs.stat(ROOT_INODE_INDEX).await;
    let inode_index = inode.index;
    fs_tree::init(inode);

    //root checks
    let root_dirs = fs.read_dir(inode_index).await;
    let required_dirs = ["dev", "proc"];
    for required_dir in required_dirs.iter() {
        if !root_dirs.iter().any(|entry| entry.name.as_ref() == *required_dir) {
            //create the required directory
            fs.create(required_dir, ROOT_INODE_INDEX, InodeType::new_dir(0o755), 0, 0).await;
        }
    }
}

pub async fn unmount(path: ResolvedPathBorrowed<'_>) {
    let parent_path = path.index(0..path.len() - 1);
    let parent_inode_num = fs_tree::get_inode_index(parent_path).await.unwrap();
    let current_name = path.get(path.len() - 1).unwrap();

    let inode_num = fs_tree::get_inode_index(path).await.unwrap();
    let last_part_mount = fs_tree::unmount_inode(parent_inode_num, current_name);
    if last_part_mount {
        let mut vfs = VFS.lock();
        let Some(device) = vfs.devices.get(&inode_num.device_id) else {
            return;
        };
        let partition_id = device.partition;
        let Some(partition) = vfs.mounted_partitions.remove(&partition_id) else {
            return;
        };
        partition.unmount().await;
    }
}

pub async fn get_dir_entries(path: ResolvedPathBorrowed<'_>) -> Result<Box<[DirEntry]>, String> {
    let inode_num = fs_tree::get_inode_index(path).await.ok_or("Path not found")?;
    let inode = fs_tree::get_inode(inode_num).ok_or("Inode not found")?;
    let mut vfs = VFS.lock();
    let device_details = vfs.devices.get(&inode.device).ok_or("Device not found")?;
    let partition_id = device_details.partition;
    let fs = vfs.mounted_partitions.get_mut(&partition_id).ok_or("FS not found")?;
    Ok(fs.read_dir(inode_num.index).await)
}

pub async fn create_file(path: ResolvedPathBorrowed<'_>, name: &str, inode_type: InodeType) {
    let parent_inode_num = fs_tree::get_inode_index(path).await.unwrap();
    let parent_inode = fs_tree::get_inode(parent_inode_num).unwrap();
    let mut vfs = VFS.lock();
    let device_details = vfs.devices.get(&parent_inode.device).unwrap();
    let partition_id = device_details.partition;
    let fs = vfs.mounted_partitions.get_mut(&partition_id).unwrap();
    let (file_inode, parent_inode) = fs.create(name, parent_inode.index, inode_type, 0, 0).await;
    fs_tree::update_inode(parent_inode_num, parent_inode);
    fs_tree::insert_inode(parent_inode_num, name.to_string().into_boxed_str(), file_inode);
}

pub async fn write_file(path: ResolvedPathBorrowed<'_>, content: &[PhysAddr], offset: u64, size: u64) {
    let inode_num = fs_tree::get_inode_index(path).await.unwrap();
    let inode = fs_tree::get_inode(inode_num).unwrap();
    let mut vfs = VFS.lock();
    let device_details = vfs.devices.get(&inode.device).unwrap();
    let partition_id = device_details.partition;
    let fs = vfs.mounted_partitions.get_mut(&partition_id).unwrap();
    fs.write(inode.index, offset, size, content).await;
}

pub async fn read_file(path: ResolvedPathBorrowed<'_>, buffer: &[PhysAddr], offset: u64, size: u64) {
    let inode_num = fs_tree::get_inode_index(path).await.unwrap();
    let inode = fs_tree::get_inode(inode_num).unwrap();
    let mut vfs = VFS.lock();
    let device_details = vfs.devices.get(&inode.device).unwrap();
    let partition_id = device_details.partition;
    let fs = vfs.mounted_partitions.get_mut(&partition_id).unwrap();
    fs.read(inode.index, offset, size, buffer).await;
}
