from gdb.unwinder import Unwinder, FrameId
import gdb

TARGET_FUNC = "set_entries::wrapper"

class InterruptUnwinder(Unwinder):
    def __init__(self):
        super().__init__("interrupt_macro_unwinder")

    def __call__(self, pending_frame):
        try:
            name = pending_frame.name()
            if not name or not name.endswith(TARGET_FUNC):
                return None
        except:
            return None
        
        rsp = pending_frame.read_register("rsp")
        u64_ptr = gdb.lookup_type("u64").pointer()

        # load the saved return rip and stack pointer from interrupt_frame
        try:
            saved_rsp = (rsp + (21 * 8)).cast(u64_ptr).dereference()
            saved_rip = (rsp + (18 * 8)).cast(u64_ptr).dereference()
        except:
            print("Could not extract saved RIP and RSP from current rsp.")
            return None

        print(f"Unwinding interrupt frame at RIP={int(saved_rip):#x}, RSP={int(saved_rsp):#x}")

        # Create the unwind info for the caller frame
        fid = FrameId(saved_rsp, saved_rip)
        unwind_info = pending_frame.create_unwind_info(fid)
        unwind_info.add_saved_register("rsp", saved_rsp)
        unwind_info.add_saved_register("rip", saved_rip)
        return unwind_info


gdb.unwinder.register_unwinder(None, InterruptUnwinder(), replace=True)
print("Interrupt unwinder registered for:", TARGET_FUNC)

