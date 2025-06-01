use crate::println;
pub use core::panic::*;

//function that prints the stack trace (function addresses)

pub fn print_stack_trace() {
    let mut rbp: usize;
    unsafe {
        core::arch::asm!("mov {}, rbp", out(reg) rbp); // Get the base pointer
    }

    if rbp == 0 {
        println!("Enable rustflags -C force-frame-pointers to get a stack trace you idiot`");
        println!("Enable rustflags -C force-frame-pointers to get a stack trace you idiot`");
        println!("Enable rustflags -C force-frame-pointers to get a stack trace you idiot`");
        println!("Enable rustflags -C force-frame-pointers to get a stack trace you idiot`");
        return;
    }
    println!("rbp is {:#x}", rbp);

    // Walk the stack frame

    loop {
        let old_rbp: usize = unsafe { *(rbp as *const usize) };
        if old_rbp == 0 {
            println!("main.rs");
            break;
        }
        let return_address = unsafe { *((rbp + 8) as *const usize) };
        let call_instruction_address = return_address.wrapping_sub(5);

        let mut offset_bytes: [u8; 4] = [0; 4];

        let instruction_ptr = call_instruction_address as *const u8;

        let opcode: u8 = unsafe { *instruction_ptr };

        if opcode == 0xE8 {
            // Read the offset (next 4 bytes)
            offset_bytes.copy_from_slice(unsafe { core::slice::from_raw_parts(instruction_ptr.add(1), 4) });

            // Convert offset to a signed integer
            let offset = i32::from_le_bytes(offset_bytes);

            // Calculate the target address
            let target_address = call_instruction_address.wrapping_add(5).wrapping_add(offset as usize);

            // Print the return address and the target address
            println!(
                "Return address: {:#x}, Target function address: {:#x}",
                return_address, target_address
            );
        } else {
            // If it's not a near call, handle other types accordingly
            println!(
                "Return address: {return_address:#x}, Call instruction {opcode:#x} not recognized at: {call_instruction_address:#x}"
            );
        }

        rbp = old_rbp;
    }
    println!("End of stack trace");
}

fn test_fn_1() {
    test_fn_2();
}

fn test_fn_2() {
    test_fn_3();
}

fn test_fn_3() {
    print_stack_trace();
}
