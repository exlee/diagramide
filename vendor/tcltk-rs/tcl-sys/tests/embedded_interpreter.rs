use std::ffi::CString;
use tcl_sys::*;

mod common;
use common::Wrapper;

const PROGRAM: &'static str = "\
set A 6
set B 6
expr { $A * $B}";

#[test]
fn use_embedded_tcl_engine() {
    let tcl_interp = Wrapper::new();

    let script = CString::new(PROGRAM).expect("Unable to create cstring");
    assert_eq!(
        unsafe { Tcl_Eval(tcl_interp.0, script.as_ptr()) },
        TCL_OK as i32,
        "Failed to run tcl script"
    );

    let obj_result = unsafe { Tcl_GetObjResult(tcl_interp.0) };
    assert!(!obj_result.is_null());

    let mut result = 0;
    assert_eq!(
        unsafe { Tcl_GetIntFromObj(tcl_interp.0, obj_result, &mut result) },
        TCL_OK as i32,
        "Contents of tcl result was not an integer"
    );

    assert_eq!(result, 36);
}
