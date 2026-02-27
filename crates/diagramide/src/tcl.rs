use std::{
    ffi::{CStr, CString},
    panic,
    sync::OnceLock,
};
use tcl_sys::*;
use tokio::task;

static TCL_LIB: OnceLock<Option<libloading::Library>> = OnceLock::new();

macro_rules! get_lib {
    () => {
        TCL_LIB
            .get()
            .and_then(|opt| opt.as_ref())
            .expect("ERROR: TCL function called but no TCL library was loaded.")
    };
}

macro_rules! get_sym {
    ($name:ident, $t:ty) => {
        unsafe {
            let lib = get_lib!();
            let sym: libloading::Symbol<$t> = lib
                .get(concat!(stringify!($name), "\0").as_bytes())
                .expect(concat!("ERROR: Could not find symbol ", stringify!($name)));
            sym
        }
    };
}

pub fn is_tcl_loadable() -> bool {
    TCL_LIB
        .get_or_init(|| {
            let candidates = if cfg!(target_os = "windows") {
                vec!["tcl86.dll", "tcl86t.dll", "tcl85.dll"]
            } else if cfg!(target_os = "macos") {
                vec![
                    "libtcl8.6.dylib",
                    "libtcl.dylib",
                    "/opt/homebrew/opt/tcl-tk/lib/libtcl8.6.dylib",
                    "/usr/local/opt/tcl-tk/lib/libtcl8.6.dylib",
                ]
            } else {
                vec!["libtcl8.6.so", "libtcl.so"]
            };

            candidates
                .into_iter()
                .find_map(|name| unsafe { libloading::Library::new(name).ok() })
        })
        .is_some()
}

pub async fn safe_eval_tcl(script: String) -> Result<String, String> {
    if !is_tcl_loadable() {
        return Err("Tcl shared library not found".to_string());
    }

    task::spawn_blocking(move || {
        panic::catch_unwind(|| eval_tcl(&script))
            .map_err(|_| "Tcl interpreter panicked or crashed".to_string())?
    })
    .await
    .map_err(|e| e.to_string())?
}

pub fn eval_tcl(script: &str) -> Result<String, String> {
    type TclFindExecFn = unsafe extern "C" fn(*const std::os::raw::c_char);
    type TclCreateInterpFn = unsafe extern "C" fn() -> *mut Tcl_Interp;
    type TclDeleteInterpFn = unsafe extern "C" fn(*mut Tcl_Interp);
    type TclEvalExFn =
        unsafe extern "C" fn(*mut Tcl_Interp, *const std::os::raw::c_char, i32, i32) -> i32;
    type TclGetObjResFn = unsafe extern "C" fn(*mut Tcl_Interp) -> *mut Tcl_Obj;
    type TclGetStrFromObjFn =
        unsafe extern "C" fn(*mut Tcl_Obj, *mut i32) -> *mut std::os::raw::c_char;

    let tcl_find_executable = get_sym!(Tcl_FindExecutable, TclFindExecFn);
    let tcl_create_interp = get_sym!(Tcl_CreateInterp, TclCreateInterpFn);
    let tcl_delete_interp = get_sym!(Tcl_DeleteInterp, TclDeleteInterpFn);
    let tcl_eval_ex = get_sym!(Tcl_EvalEx, TclEvalExFn);
    let tcl_get_obj_result = get_sym!(Tcl_GetObjResult, TclGetObjResFn);
    let tcl_get_string_from_obj = get_sym!(Tcl_GetStringFromObj, TclGetStrFromObjFn);
    unsafe {
        tcl_find_executable(std::ptr::null());

        let interp = tcl_create_interp();
        if interp.is_null() {
            return Err("Failed to create Tcl interpreter".to_string());
        }

        let c_script = CString::new(script).unwrap();

        let code = tcl_eval_ex(interp, c_script.as_ptr() as *const _, -1, 0);

        let obj_result = tcl_get_obj_result(interp);
        let result_ptr = tcl_get_string_from_obj(obj_result, std::ptr::null_mut());

        let result = CStr::from_ptr(result_ptr as *const _)
            .to_string_lossy()
            .into_owned();

        tcl_delete_interp(interp);

        if code == TCL_OK as i32 {
            Ok(result)
        } else {
            Err(result)
        }
    }
}
