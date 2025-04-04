use std::{
    mem_utils::{VirtAddr, get_at_physical_addr},
    println,
    vec::Vec,
};

use crate::vfs::{self, InodeType};


//TODO: move this to vfs operations when vfs works

const FILE_OPERATIONS: [FileOperation; 4] = [
    // FileOperation::CreateFolder(CreateFolderOperation::new("/test")),
    // FileOperation::ReadDir(ReadDirOperation::new("/")),
    FileOperation::CreateFile(CreateFileOperation::new("/test.txt")),
    // FileOperation::ReadDir(ReadDirOperation::new("/")),
    FileOperation::Write(WriteFileOperation::new("/test.txt", "Hello, world!", 0)),
    FileOperation::ReadFile(ReadFileOperation::new("/test.txt", 0, 5)),
    FileOperation::ReadFile(ReadFileOperation::new("/test.txt", 7, 6)),
];

pub fn do_file_operations() {
    for opeartion in FILE_OPERATIONS {
        opeartion.execute();
    }
}

enum FileOperation {
    CreateFile(CreateFileOperation),
    ReadDir(ReadDirOperation),
    Write(WriteFileOperation),
    Delete(DeleteFileOperation),
    CreateFolder(CreateFolderOperation),
    ReadFile(ReadFileOperation),
}

impl FileOperation {
    fn execute(&self) {
        match self {
            Self::ReadFile(op) => op.execute(),
            Self::CreateFile(op) => op.execute(),
            Self::Write(op) => op.execute(),
            Self::Delete(op) => op.execute(),
            Self::CreateFolder(op) => op.execute(),
            Self::ReadDir(op) => op.execute(),
        }
    }
}

struct CreateFileOperation {
    file_name: &'static str,
}

struct ReadDirOperation {
    folder_name: &'static str,
}

impl ReadDirOperation {
    const fn new(folder_name: &'static str) -> Self {
        Self { folder_name }
    }

    fn execute(&self) {
        let path = vfs::resolve_path(self.folder_name, "/");
        let entries = vfs::get_dir_entries(path);
        println!("Read dir: {}", self.folder_name);
        println!("Dir entries: {:?}", entries);
    }
}

impl CreateFileOperation {
    const fn new(file_name: &'static str) -> Self {
        Self { file_name }
    }

    fn execute(&self) {
        let split_index = self.file_name.rfind('/').unwrap();
        let (parent_path, file_name) = self.file_name.split_at(split_index + 1); //slash is in the
        println!("Creating file: {}", self.file_name);
        vfs::create_file(vfs::resolve_path(parent_path, "/"), file_name, InodeType::new_file(0));
        println!("Created file: {:?}", self.file_name);
    }
}

struct WriteFileOperation {
    file_name: &'static str,
    content: &'static str,
    offset: u64,
}

impl WriteFileOperation {
    const fn new(file_name: &'static str, content: &'static str, offset: u64) -> Self {
        Self {
            file_name,
            content,
            offset,
        }
    }

    fn execute(&self) {
        let path = vfs::resolve_path(self.file_name, "/");

        let mut phys_addresses = Vec::new();
        let content_address = VirtAddr((&raw const self.content) as u64);
        for i in 0..(self.content.len().div_ceil(4096)) {
            let phys_addr = std::mem_utils::translate_virt_phys_addr(content_address + i as u64 * 4096).unwrap();
            phys_addresses.push(phys_addr);
        }

        vfs::write_file(path, &phys_addresses, self.offset, self.content.len() as u64);
    }
}

struct CreateFolderOperation {
    folder_name: &'static str,
}

impl CreateFolderOperation {
    const fn new(folder_name: &'static str) -> Self {
        Self { folder_name }
    }

    fn execute(&self) {
        let split_index = self.folder_name.rfind('/').unwrap();
        let (parent_path, file_name) = self.folder_name.split_at(split_index + 1); //slash is in the
        println!("Creating folder: {}", self.folder_name);
        vfs::create_file(vfs::resolve_path(parent_path, "/"), file_name, InodeType::new_file(0))
    }
}

struct DeleteFileOperation {
    file_name: &'static str,
}

impl DeleteFileOperation {
    const fn new(file_name: &'static str) -> Self {
        Self { file_name }
    }

    fn execute(&self) {
        todo!("add vfs delete op");
    }
}

struct ReadFileOperation {
    file_name: &'static str,
    offset: u64,
    length: u64,
}

impl ReadFileOperation {
    const fn new(file_name: &'static str, offset: u64, length: u64) -> Self {
        Self {
            file_name,
            offset,
            length,
        }
    }

    fn execute(&self) {
        let path = vfs::resolve_path(self.file_name, "/");
        let mut buffer = Vec::with_capacity(self.length.div_ceil(4096) as usize);
        for _ in 0..(self.length.div_ceil(4096)) {
            let frame = crate::memory::physical_allocator::allocate_frame();
            buffer.push(frame);
        }
        vfs::read_file(path, &buffer, self.offset, self.length);
        for frame in buffer {
            let data = unsafe { get_at_physical_addr::<[u8; 4096]>(frame) };
            println!("{:?}", data);
            unsafe { crate::memory::physical_allocator::deallocate_frame(frame) };
        }
    }
}
