use uuid::Uuid;

use crate::{
    drivers::block_device::disk::DirEntry,
    memory::addresses::PhysAddr,
    vfs::{DeviceId as VfsDeviceId, FileReadResult, Inode, adapters::VfsAdapterTrait},
};
use std::{alloc::boxed::Box, error::KernelError};

#[derive(Debug)]
struct InputVfsAdapter {
    vfs_device_id: VfsDeviceId,
    device_details: crate::vfs::DeviceDetails,
}

#[async_trait::async_trait]
impl VfsAdapterTrait for InputVfsAdapter {
    fn device_id(&self) -> VfsDeviceId {
        self.vfs_device_id
    }

    fn partition_id(&self) -> Uuid {
        self.device_details.partition
    }

    async fn read(
        &self,
        _inode: crate::vfs::InodeIndex,
        _offset_bytes: u64,
        size_bytes: u64,
        buffer: &[PhysAddr],
        blocking: bool,
    ) -> Result<(u64, FileReadResult), KernelError> {
        todo!()
    }

    async fn read_dir(&self, _inode: crate::vfs::InodeIndex) -> Result<Box<[DirEntry]>, KernelError> {
        todo!()
    }

    async fn write(
        &self,
        inode: crate::vfs::InodeIndex,
        _offset: u64,
        size: u64,
        buffer: &[PhysAddr],
    ) -> Result<(Inode, u64), KernelError> {
        todo!()
    }

    async fn stat(&self, inode: crate::vfs::InodeIndex) -> Result<Inode, KernelError> {
        todo!()
    }
}
