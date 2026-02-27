use std::error::Error;
use std::ffi::CString;
use std::io;
use std::io::prelude::*;
use tcl_sys::{Tcl_Eval, TCL_OK};

mod tcl {
    include!("../tests/common/mod.rs");
}

fn main() -> Result<(), Box<dyn Error>> {
    let interpreter = tcl::Wrapper::new();
    let stdin = io::stdin();
    let handle = stdin.lock();

    for line in handle.lines() {
        let command_str = line?;
        let script = CString::new(command_str)?;
        assert_eq!(
            unsafe { Tcl_Eval(interpreter.0, script.as_ptr()) },
            TCL_OK as i32,
        );
    }

    Ok(())
}
