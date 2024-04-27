fn main() {
    // read env variables that were set in build script
    let uefi_path = env!("UEFI_PATH");
    let bios_path = env!("BIOS_PATH");

    println!("{}\n{}", uefi_path, bios_path);

    // choose whether to start the UEFI or BIOS image
    let uefi = false;

    //chose whether to debug with GDB
    let debug = false;

    let mut cmd = std::process::Command::new("qemu-system-x86_64");
    cmd.arg("-d")
        .arg("int")
        .arg("-D")
        .arg("./log.txt")
        .arg("-no-reboot");
    if debug {
        cmd.arg("-s");
        cmd.arg("-S");
    }

    if uefi {
        cmd.arg("-bios").arg(ovmf_prebuilt::ovmf_pure_efi());
        cmd.arg("-drive")
            .arg(format!("format=raw,file={uefi_path}"));
    } else {
        cmd.arg("-drive")
            .arg(format!("format=raw,file={bios_path}"));
    }
    let mut child = cmd.spawn().unwrap();
    child.wait().unwrap();
}
