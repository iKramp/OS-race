from gdb.unwinder import Unwinder, FrameId
import gdb

TARGET_FUNC = "general_interrupt_handler"

class InterruptUnwinder(Unwinder):
    def __init__(self):
        super().__init__("interrupt_unwinder")

    def __call__(self, pending_frame):
        print("InterruptUnwinder called for frame:", pending_frame)
        # Only activate in the target function
        try:
            name = pending_frame.name()
            if not name or not name.endswith(TARGET_FUNC):
                print(f"Skipping unwinder for function: {name}")
                return None
        except:
            print("Could not get function name.")
            return None
        
        print(f"Unwinding frame in function: {name}")

        # Get argument: proc_data
        try:
            proc_data = pending_frame.read_var("proc_data")
        except:
            print("Could not read proc_data variable.")
            return None

        # load the saved return rip and stack pointer from interrupt_frame
        try:
            saved_rip = proc_data["interrupt_frame"]["rip"]
            saved_rsp = proc_data["interrupt_frame"]["rsp"]
        except:
            print("Could not extract saved RIP and RSP from proc_data.")
            return None

        print(f"Unwinding interrupt frame at RIP={saved_rip:#x}, RSP={saved_rsp:#x}")

        # Create the unwind info for the caller frame
        fid = FrameId(saved_rip, saved_rsp)
        unwind_info = pending_frame.create_unwind_info()
        unwind_info.add_cfa_register("rsp", saved_rsp)
        unwind_info.add_saved_register("rip", saved_rip)
        return unwind_info


# gdb.unwinder.register_unwinder(None, InterruptUnwinder(), replace=True)
print("Interrupt unwinder registered for:", TARGET_FUNC)
