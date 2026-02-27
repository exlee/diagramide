use tcl_sys::*;
use tokio::task;
use std::{ffi::{CStr, CString}, panic, sync::OnceLock};

pub async fn safe_eval_tcl(script: String) -> Result<String, String> {
    task::spawn_blocking(move || {
        panic::catch_unwind(|| {
            eval_tcl(&script)
        }).map_err(|_| "Tcl interpreter panicked or crashed".to_string())?
    })
    .await
    .map_err(|e| e.to_string())?
}
pub fn eval_tcl(script: &str) -> Result<String, String> {
    unsafe {
        let interp = Tcl_CreateInterp();
        
        let c_script = CString::new(script).unwrap();
        
        // Tcl_EvalEx takes: (interp, script, numBytes, flags)
        // -1 for numBytes tells Tcl to read until the null terminator.
        let code = Tcl_EvalEx(interp, c_script.as_ptr(), -1, 0);
        // 1. Get the result as a Tcl_Obj
        let obj_result = Tcl_GetObjResult(interp);
        
        // 2. Extract the C string from the Tcl_Obj
        // Passing null_mut() ignores the length output
        let result_ptr = Tcl_GetStringFromObj(obj_result, std::ptr::null_mut());
        
        let result = CStr::from_ptr(result_ptr).to_string_lossy().into_owned();

        Tcl_DeleteInterp(interp);

        if code == TCL_OK as i32 {
            Ok(result)
        } else {
            Err(result)
        }
    }
}
static TCL_AVAILABLE: OnceLock<bool> = OnceLock::new();
pub fn is_tcl_loadable() -> bool {
    *TCL_AVAILABLE.get_or_init(|| {
        let candidates = if cfg!(target_os = "windows") {
            vec!["tcl86.dll", "tcl86t.dll", "tcl85.dll"]
        } else if cfg!(target_os = "macos") {
            vec![
                "libtcl8.6.dylib", 
                "libtcl.dylib",
                "/opt/homebrew/opt/tcl-tk/lib/libtcl8.6.dylib",
                "/usr/local/opt/tcl-tk/lib/libtcl8.6.dylib"
            ]
        } else {
            // Linux: Check versioned first, then generic
            vec!["libtcl8.6.so", "libtcl.so"]
        };

        unsafe {
            candidates.into_iter().any(|name| libloading::Library::new(name).is_ok())
        }
    })
}
