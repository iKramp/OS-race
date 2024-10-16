use std::path::PathBuf;

fn main() {
    let target_file = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("rust_os.json");
    let link_script_file = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("linker_script.ld");
    println!("cargo:rustc-env=TARGET_FILE={}", target_file.display());

    // Set the flag to generate the linker map file
    println!("cargo:rustc-env=RUSTFLAGS=-C force-frame-pointers=yes");
    println!("cargo:rustc-env=RUSTFLAGS=-C force-unwind-tables=yes");
    println!("cargo:rustc-link-arg=-T{}", link_script_file.display());
    //println!("cargo:rustc-link-arg=Map=/home/nejc/programming/OS-race/kernel.map");


    // Re-run the build script if the build configuration changes
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=kernel/linker_script.ld");
    
}
