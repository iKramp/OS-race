use std::{
    mem_utils::{get_at_physical_addr, set_at_physical_addr, set_at_virtual_addr, VirtAddr}, println, string::String, vec::Vec, PageAllocator
};

use crate::{
    memory::PAGE_TREE_ALLOCATOR,
    vfs::{self, InodeType},
};

const FILE_OPERATIONS: [FileOperation; 8] = [
    FileOperation::CreateFolder(CreateFolderOperation::new("/test")),
    // FileOperation::ReadDir(ReadDirOperation::new("/")),
    FileOperation::CreateFile(CreateFileOperation::new("/test.txt")),
    FileOperation::CreateFile(CreateFileOperation::new("/test/test.txt")),
    // FileOperation::ReadDir(ReadDirOperation::new("/")),
    FileOperation::Write(WriteFileOperation::new("/test.txt", "Hello, world!", 0)),
    FileOperation::Write(WriteFileOperation::new("/test/test.txt", "This is a second string", 0)),
    FileOperation::ReadFile(ReadFileOperation::new("/test.txt", 0, 5)),
    FileOperation::ReadFile(ReadFileOperation::new("/test.txt", 7, 6)),
    FileOperation::ReadFile(ReadFileOperation::new("/test/test.txt", 0, 23)),
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
        let content = self.content.as_bytes();
        for i in 0..(content.len().div_ceil(4096)) {
            let frame = crate::memory::physical_allocator::allocate_frame();
            let frame_binding = unsafe { PAGE_TREE_ALLOCATOR.allocate(Some(frame)) };
            unsafe {
                PAGE_TREE_ALLOCATOR
                    .get_page_table_entry_mut(frame_binding)
                    .set_pat(crate::memory::paging::LiminePat::UC);
            }
            let ptr = frame_binding.0 as *mut u8;
            for j in 0..4096 {
                if (i * 4096 + j) < content.len() {
                    unsafe { *ptr.add(j) = content[i * 4096 + j] };
                } else {
                    unsafe { *ptr.add(j) = 0 };
                }
            }
            phys_addresses.push(frame);
            unsafe { PAGE_TREE_ALLOCATOR.unmap(frame_binding) };

        }


        println!("Writing file: {} with content: {}", self.file_name, self.content);
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
        let real_offset = self.offset & !4095;
        let real_length = self.length - real_offset + self.offset;
        let mut buffer = Vec::with_capacity(real_length.div_ceil(4096) as usize);
        for _ in 0..(real_length.div_ceil(4096)) {
            let frame = crate::memory::physical_allocator::allocate_frame();
            buffer.push(frame);
        }
        vfs::read_file(path, &buffer, real_offset, real_length);
        let mut final_data = Vec::with_capacity(self.length as usize);
        let mut frame_ptr = (self.offset as usize) & 0xFFF;
        for (index, frame) in buffer.iter().enumerate() {
            let frame_ptr_start = frame_ptr;
            let limit = if index == buffer.len() - 1 {
                (self.length as usize) & 0xFFF
            } else {
                4096
            };
            let data = unsafe { get_at_physical_addr::<[u8; 4096]>(*frame) };
            while frame_ptr < limit + frame_ptr_start {
                final_data.push(data[frame_ptr]);
                frame_ptr += 1;
            }
            unsafe { crate::memory::physical_allocator::deallocate_frame(*frame) };
        }
        println!("Read file: {} at offset {} and size of read {}", self.file_name, self.offset, self.length);
        println!("File content: {:?}", final_data);
        //transofm into string
        let string = String::from_utf8(final_data).unwrap();
        println!("File content as string: {}", string);
    }
}
