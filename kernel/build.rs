fn main() {
    // Set the flag to generate the linker map file
    println!("cargo:rustc-env=RUSTFLAGS=-C force-frame-pointers=yes");
    println!("cargo:rustc-env=RUSTFLAGS=-C force-unwind-tables=yes");
    println!("cargo:rustc-link-arg=-Map=/home/nejc/programming/OS-race/kernel.map");

    // Re-run the build script if the build configuration changes
    println!("cargo:rerun-if-changed=build.rs");
}
