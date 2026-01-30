import lldb

def find_thread(debugger, target_file, result, internal_dict):
    process = debugger.GetSelectedTarget().GetProcess()
    for thread in process:
        for frame in thread:
            file_spec = frame.GetLineEntry().GetFileSpec()
            if file_spec.IsValid() and target_file in file_spec.GetFilename():
                print(f"Thread {thread.GetIndexID()}: {thread.GetName()} @ {frame.GetFrameID()}")
                break

def __lldb_init_module(debugger, internal_dict):
    debugger.HandleCommand('command script add -f filters.find_thread find_thread')
