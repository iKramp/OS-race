use std::mem_utils::{PhysAddr, VirtAddr};


static LIMINE_BASE_REVISION: [u64; 3] = [0xf9562b2d5c95a6c8, 0x6a7b384944536bdc, 2];

pub static mut LIMINE_BOOTLOADER_REQUESTS: BootloaderRequests = BootloaderRequests {
    _request_start_marker: [0xf6b8f4b39de7d1ae, 0xfab91a6940fcb9cf, 0x785c6ed015d3e316, 0x181e920a7852b9d9],
    bootloader_info_request: BootloaderInfoRequest {
        magic: [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b, 0xf55038d8e2a1202f, 0x279426fcf5f59740],
        revision: 0,
        info: core::ptr::null(),
    },
    firmware_type_request: FirmwareTypeRequest {
        magic: [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b, 0x8c2f75d90bef28a8, 0x7045a4688eac00],
        revision: 0,
        info: core::ptr::null(),
    },
    //higher_half_direct_map_request: todo!(),
    frame_buffer_request: FrameBufferRequest {
        magic: [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b, 0x9d5827dcd881dd75, 0xa3148604f6fab11b],
        revision: 0,
        info: core::ptr::null(),
    },
    //paging_mode_request: todo!(),
    //smp_request: todo!(),
    //memory_map_request: todo!(),
    //rsdp_request: todo!(),
    //kernel_address_request: todo!(),
    _request_end_marker: [0xadc0e0531bb10d03, 0x9572709f31764c62],
};

#[repr(C)]
pub struct BootloaderRequests {
    _request_start_marker: [u64; 4],

    pub bootloader_info_request: BootloaderInfoRequest,
    pub firmware_type_request: FirmwareTypeRequest,
    //higher_half_direct_map_request: HigherHalfDirectMapRequest,
    pub frame_buffer_request: FrameBufferRequest,
    //paging_mode_request: PagingModeRequest,
    //smp_request: SMPRequest,
    //memory_map_request: MemoryMapRequest,
    //rsdp_request: RsdpRequest,
    //kernel_address_request: KernelAddressRequest,
    
    _request_end_marker: [u64; 2],
}

#[repr(C)]
pub struct BootloaderInfoRequest {
    magic: [u64; 4],
    revision: u64,
    pub info: *const BootloaderInfo,
}

#[repr(C)]
pub struct BootloaderInfo {
    revision: u64,
    name: &'static [u8],
    version: &'static [u8],
}

#[repr(C)]
struct FirmwareTypeRequest {
    magic: [u64; 4],
    revision: u64,
    info: *const FirmwareType,
}

#[repr(C)]
struct FirmwareType {
    revision: u64,
    firmware_type: u64, //0 = bios, 1 = uefi32, 2 = uefi64
}

#[repr(C)]
struct HigherHalfDirectMapRequest {
    magic: [u64; 4],
    revision: u64,
    info: *const HigherHalfDirectMap,
}

#[repr(C)]
struct HigherHalfDirectMap {
    revision: u64,
    offset: u64,
}

#[repr(C)]
pub struct FrameBufferRequest {
    magic: [u64; 4],
    pub revision: u64,
    pub info: *const FrameBuffer,
}

#[repr(C)]
pub struct FrameBuffer {
    pub revision: u64,
    pub framebuffer_count: u64,
    pub framebuffers: *const [*const FramebufferInfo],
}

#[repr(C)]
pub struct FramebufferInfo {
    pub address: *const (),
    pub width: u64,
    pub height: u64,
    pub pitch: u64,
    pub bpp: u16,
    pub memory_model: u8,
    pub red_mask_size: u8,
    pub red_mask_shift: u8,
    pub green_mask_size: u8,
    pub green_mask_shift: u8,
    pub blue_mask_size: u8,
    pub blue_mask_shift: u8,
    pub unused: [u8; 7],
    pub edid_size: u64,
    pub edid: *const (),

    //v >= 1
    pub mode_count: u64,
    pub modes: *const [&'static FramebufferMode],
}

#[repr(C)]
pub struct FramebufferMode {
    pub pitch: u64,
    pub width: u64,
    pub height: u64,
    pub bpp: u16,
    pub memory_model: u8,
    pub red_mask_size: u8,
    pub red_mask_shift: u8,
    pub green_mask_size: u8,
    pub green_mask_shift: u8,
    pub blue_mask_size: u8,
    pub blue_mask_shift: u8,
}



#[repr(C)]
struct PagingModeRequest {
    magic: [u64; 4],
    revision: u64, //1
    info: *const u8,
    mode: u64,
    min_mode: u64, //set to 0
    max_mode: u64,
}

#[repr(C)]
struct SMPRequest { //processors will be bootstrapped 
    magic: [u64; 4],
    revision: u64,
    info: *const SMPInfo,
    flags: u64, //set to 0 to not enable x2apic, dealing with that is a pain
}

#[repr(C)]
struct SMPInfo {
    revision: u64,
    flags: u32,
    bsp_lapic_id: u32,
    cpu_count: u64,
    cpu_info: *const [&'static CPUInfo],
}

#[repr(C)]
struct CPUInfo {
    processor_id: u32,
    lapic_id: u32,
    reserved: u64,
    goto_address: PhysAddr,
    extra_argument: u64, //free to use
}


#[repr(C)]
struct MemoryMapRequest {
    magic: [u64; 4],
    revision: u64,
    info: *const MemoryMap,
}

#[repr(C)]
struct MemoryMap {
    revision: u64,
    memory_map_count: u64,
    memory_map: *const [&'static MemoryMapEntry],
}

#[repr(C)]
struct MemoryMapEntry {
    base: u64,
    length: u64,
    entry_type: u64,
}
/*
#define LIMINE_MEMMAP_USABLE                 0
#define LIMINE_MEMMAP_RESERVED               1
#define LIMINE_MEMMAP_ACPI_RECLAIMABLE       2
#define LIMINE_MEMMAP_ACPI_NVS               3
#define LIMINE_MEMMAP_BAD_MEMORY             4
#define LIMINE_MEMMAP_BOOTLOADER_RECLAIMABLE 5
#define LIMINE_MEMMAP_KERNEL_AND_MODULES     6
#define LIMINE_MEMMAP_FRAMEBUFFER            7
*/

#[repr(C)]
struct RsdpRequest {
    magic: [u64; 4],
    revision: u64,
    info: *const Rsdp,
}

#[repr(C)]
struct Rsdp {
    revision: u64,
    rsdp: *const (),
}

#[repr(C)]
struct KernelAddressRequest {
    magic: [u64; 4],
    revision: u64,
    info: *const KernelAddress,
}

#[repr(C)]
struct KernelAddress {
    revision: u64,
    phys_addr: PhysAddr,
    virt_addr: VirtAddr,
}






