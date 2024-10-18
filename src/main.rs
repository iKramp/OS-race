fn main() {
    //demangle the kernel.map file
    //let _ = std::process::Command::new("rustfilt")
    //    .arg("-i")
    //    .arg("/home/nejc/programming/OS-race/kernel.map")
    //    .arg("-o")
    //    .arg("/home/nejc/programming/Os-race/kernel.map");



    //chose whether to debug with GDB
    let debug = true;
    let uefi = false;

    let mut cmd = std::process::Command::new("qemu-system-x86_64");
    cmd.arg("-d").arg("int").arg("-D").arg("./log.txt").arg("-no-reboot");
    if debug {
        cmd.arg("-s");
        cmd.arg("-S");
    }
    //cmd.arg("-cpu").arg("EPYC");
    cmd.arg("-smp").arg("1");

    #[cfg(test)]
    {
        cmd.arg("-device").arg("isa-debug-exit,iobase=0xf4,iosize=0x04");
    }


    if uefi {
        cmd.arg("-bios").arg(ovmf_prebuilt::ovmf_pure_efi());
        cmd.arg("-drive").arg("format=raw,file=kernel_build_files/image.iso");
    } else {
        cmd.arg("-drive").arg("format=raw,file=kernel_build_files/image.iso");
    }
    //cmd.arg("-cdrom").arg("kernel_build_files/kernel.iso");
    let mut child = cmd.spawn().unwrap();

    if debug {
        let _ = std::process::Command::new("kitty")
            .arg("gdb")
            .arg("-x").arg("~/programming/OS-race/gdb_commands.txt")
            .spawn().unwrap();
            
    }

    child.wait().unwrap();
}

#[test]
fn test_run() {
    main();
}
