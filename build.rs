use std::{path::PathBuf, process::Command};

fn main() {
    // set by cargo, build scripts should use this directory for output files
    let _out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let kernel = PathBuf::from(std::env::var_os("CARGO_BIN_FILE_KERNEL_kernel").unwrap());
    std::fs::copy(
        kernel,
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("kernel_build_files/kernel.elf"),
    )
    .unwrap();
    let status = Command::new("objcopy")
        .arg("kernel_build_files/kernel.elf")
        .arg("kernel_build_files/kernel.bin")
        .current_dir(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new(std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/make_disk.sh")
        .status()
        .unwrap();
    assert!(status.success());

    println!("cargo:rerun-if-canged=build.rs");
}
