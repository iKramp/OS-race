use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let link_script_file = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("linker_script.ld");

    if !(Command::new("nasm")
        .args([
            "-f",
            "elf64",
            "src/acpi/smp/trampoline.asm",
            "-o",
            &(out_dir.clone() + "/trampoline.o"),
        ])
        .status()
        .unwrap()
        .success()
        //&& Command::new("rm")
        //    .arg(&(out_dir.clone() + "/libap_startup.a"))
        //    .status()
        //    .unwrap()
        //    .success()
        && Command::new("ar")
            .arg("rcs")
            .arg(&(out_dir.clone() + "/libtrampoline.a"))
            .arg(&(out_dir.clone() + "/trampoline.o"))
            .status()
            .unwrap()
            .success())
    {
        panic!("Failed to assemble trampoline.s");
    }

    // Set the flag to generate the linker map file
    println!("cargo:rustc-link-arg=-T{}", link_script_file.display());
    //println!("cargo:rustc-link-arg=Map=/home/nejc/programming/OS-race/kernel.map");

    // Re-run the build script if the build configuration changes
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=kernel/linker_script.ld");
    println!("cargo:rustc-link-search={}", out_dir);
    println!("cargo:rustc-link-lib=static=trampoline");
}
