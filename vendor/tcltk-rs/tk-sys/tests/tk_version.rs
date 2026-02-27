use std::ffi::{CStr, CString};
use tcl_sys::*;
use tk_sys::Tk_Init;

mod common;
use common::Wrapper;

fn return_code(code: u32) -> &'static str {
    match code {
        TCL_OK => stringify!(TCL_OK),
        TCL_ERROR => stringify!(TCL_ERROR),
        TCL_RETURN => stringify!(TCL_RETURN),
        TCL_BREAK => stringify!(TCL_BREAK),
        TCL_CONTINUE => stringify!(TCL_CONTINUE),
        _ => unreachable!(),
    }
}

fn get_result_str<'a>(interp: &'a mut Wrapper) -> &'a CStr {
    unsafe { CStr::from_ptr(Tcl_GetString(Tcl_GetObjResult(interp.0))) }
}

#[test]
fn tk_is_expected_version() {
    let mut interpreter = Wrapper::new();

    assert_eq!(
        unsafe { Tk_Init(interpreter.0 as *mut tk_sys::Tcl_Interp) },
        TCL_OK as i32,
        "{:?}",
        get_result_str(&mut interpreter)
    );

    let script = CString::new("package require Tk").expect("Unable to convert string to cstring");
    let interp_result = unsafe { Tcl_Eval(interpreter.0, script.as_ptr()) };
    let return_code = return_code(interp_result.try_into().unwrap());
    assert_eq!(
        interp_result,
        TCL_OK as i32,
        "Failed to run script, return error: {return_code}, reason: {:?}",
        get_result_str(&mut interpreter)
    );
}
