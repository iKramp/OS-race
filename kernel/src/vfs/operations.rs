use std::{
    sync::arc::Arc, boxed::Box, format, lock_w_info, mem_utils::PhysAddr, string::{String, ToString}, sync::lock_info::LockLocationInfo, vec::Vec
};

use uuid::Uuid;

use crate::drivers::{
    disk::{BlockDevice, DirEntry, MountedPartition, PartitionSchemeDriver},
    gpt::GPTDriver,
};

use super::{
    DeviceDetails, InodeIdentifierChain, InodeType, ROOT_INODE_INDEX, ResolvedPath, ResolvedPathBorrowed, VFS,
    file::{FileFlags, FileHandle},
    filesystem_trait::FileSystem,
    fs_tree::{self},
};

pub async fn add_disk(mut disk: Box<dyn BlockDevice + Send>) {
    //for now only GPT
    let gpt_driver = GPTDriver {};
    let guid = gpt_driver.guid(&mut *disk).await;
    let partitions = gpt_driver.partitions(&mut *disk).await;
    let partition_guids: Vec<Uuid> = partitions.iter().map(|(guid, _)| *guid).collect();

    let mut vfs = lock_w_info!(VFS);

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
    let mut vfs = lock_w_info!(VFS);
    for partition in vfs.disks.remove(&uuid).unwrap().1.iter() {
        vfs.available_partitions.remove(partition);
    }
}

pub async fn mount_blkdev_partition(part_id: Uuid, mountpoint: ResolvedPath) -> Result<(), String> {
    let mut vfs = lock_w_info!(VFS);
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
    let fs = fs_factory.mount(mounted_partition).await;
    if let Err(e) = mount_filesystem(mountpoint, &fs).await {
        fs.unmount().await;
        Err(e)
    } else {
        let fs: Arc<dyn FileSystem + Send> = fs;
        vfs.mounted_filesystems.insert(part_id, fs);
        Ok(())
    }
}

async fn mount_filesystem(mountpoint: ResolvedPath, fs: &Arc<dyn FileSystem + Send>) -> Result<(), String> {
    let root = mountpoint.inner().is_empty();
    if root {
        //mounting root
        mount_new_root(fs).await;
        //anything else?
    } else {
        let fs_root_inode = fs.stat(ROOT_INODE_INDEX).await;
        //we disallow the mounting of root failing so no checks :3
        let (inode, _parent_inode_chain) = fs_tree::get_inode_chain((&mountpoint).into(), None)
            .await
            .ok_or("mountpoint not found")?;
        fs_tree::mount_inode(inode, fs_root_inode);
    }

    Ok(())
}

async fn mount_new_root(fs: &Arc<dyn FileSystem + Send>) {
    let inode = fs.stat(ROOT_INODE_INDEX).await;
    let inode_index = inode.index;
    fs_tree::init(inode);

    //root checks
    let root_dirs = fs.read_dir(inode_index).await;
    let required_dirs = ["dev", "proc"];
    for required_dir in required_dirs.iter() {
        if !root_dirs.iter().any(|entry| entry.name.as_ref() == *required_dir) {
            //create the required directory
            fs.create(required_dir, ROOT_INODE_INDEX, InodeType::new_dir(0o755), 0, 0)
                .await;
        }
    }
}

pub async fn unmount(path: ResolvedPathBorrowed<'_>) -> Result<(), String> {
    let inodes = fs_tree::get_unmount_inodes(path, None).await.ok_or("Not a mount point")?;
    let last_part_mount = fs_tree::unmount_inode(inodes.0);
    if last_part_mount {
        let mut vfs = lock_w_info!(VFS);
        let Some(device) = vfs.devices.get(&inodes.1.device_id) else {
            return Ok(());
        };
        let partition_id = device.partition;
        let Some(partition) = vfs.mounted_filesystems.remove(&partition_id) else {
            return Ok(());
        };
        partition.unmount().await;
    }
    Ok(())
}

pub async fn open_file(
    path: ResolvedPathBorrowed<'_>,
    from: Option<InodeIdentifierChain>,
    mut open_mode: FileFlags,
) -> Result<FileHandle, String> {
    let (inode_index, inode_chain) = fs_tree::get_inode_chain(path, from).await.ok_or("Path not found")?;
    let inode = fs_tree::get_inode(inode_index).ok_or("Inode not found")?;
    open_mode.set_dir(inode.type_mode.is_dir());
    //TODO: check permissions
    Ok(FileHandle {
        inode: inode_index,
        parent_chain: inode_chain,
        position: 0,
        file_flags: open_mode,
    })
}

pub async fn get_dir_entries(file_handle: &FileHandle) -> Result<Box<[DirEntry]>, String> {
    let inode = fs_tree::get_inode(file_handle.inode).ok_or("Inode not found")?;
    let mut vfs = lock_w_info!(VFS);
    let device_details = vfs.devices.get(&inode.device).ok_or("Device not found")?;
    let partition_id = device_details.partition;
    let fs = vfs.mounted_filesystems.get_mut(&partition_id).ok_or("FS not found")?;
    let fs = fs.clone();
    drop(vfs);
    Ok(fs.read_dir(file_handle.inode.index).await)
}

pub async fn create_file(parent_dir: &mut FileHandle, name: &str, inode_type: InodeType) -> Result<(), String> {
    if !parent_dir.file_flags.dir() || !parent_dir.file_flags.write() {
        return Err("Parent directory not opened in write mode".to_string());
    }

    let parent_inode = fs_tree::get_inode(parent_dir.inode).ok_or("Inode not found")?;
    let mut vfs = lock_w_info!(VFS);
    let device_details = vfs.devices.get(&parent_inode.device).ok_or("Device not found")?;
    let partition_id = device_details.partition;
    let fs = vfs.mounted_filesystems.get_mut(&partition_id).ok_or("FS not found")?;
    let fs = fs.clone();
    drop(vfs);
    let (file_inode, parent_inode) = fs.create(name, parent_inode.index, inode_type, 0, 0).await;
    fs_tree::update_inode(parent_dir.inode, parent_inode);
    fs_tree::insert_inode(parent_dir.inode, name.to_string().into_boxed_str(), file_inode);
    Ok(())
}

pub async fn write_file(file_handle: &mut FileHandle, content: &[PhysAddr], offset: u64, size: u64) -> Result<(), String> {
    if !file_handle.file_flags.write() {
        return Err("File opened in read-only mode".to_string());
    }

    let inode = fs_tree::get_inode(file_handle.inode).ok_or("Inode not found")?;
    let mut vfs = lock_w_info!(VFS);
    let device_details = vfs.devices.get(&inode.device).ok_or("Device not found")?;
    let partition_id = device_details.partition;
    let fs = vfs.mounted_filesystems.get_mut(&partition_id).ok_or("FS not found")?;
    let fs = fs.clone();
    drop(vfs);

    let offset = if file_handle.file_flags.append() {
        inode.size
    } else {
        file_handle.position + offset
    };

    fs.write(inode.index, offset, size, content).await;

    if file_handle.file_flags.append() {
        return Ok(());
    }

    file_handle.position += size;
    Ok(())
}

pub async fn read_file(file_handle: &mut FileHandle, buffer: &[PhysAddr], size: u64) -> Result<(), String> {
    if !file_handle.file_flags.read() {
        return Err("File opened in write-only mode".to_string());
    }

    let inode = fs_tree::get_inode(file_handle.inode).ok_or("Inode not found")?;
    let mut vfs = lock_w_info!(VFS);
    let device_details = vfs.devices.get(&inode.device).ok_or("Device not found")?;
    let partition_id = device_details.partition;
    let fs = vfs.mounted_filesystems.get_mut(&partition_id).ok_or("FS not found")?;
    let fs = fs.clone();
    drop(vfs);

    let offset = file_handle.position;

    fs.read(inode.index, offset, size, buffer).await;

    file_handle.position += size;
    Ok(())
}
