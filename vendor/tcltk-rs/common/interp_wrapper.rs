use tcl_sys::*;

pub struct Wrapper(pub *mut Tcl_Interp);

impl Wrapper {
    pub fn new() -> Wrapper {
        unsafe {
            let tcl_interp = Tcl_CreateInterp();
            Tcl_Init(tcl_interp);
            assert!(!tcl_interp.is_null());
            Wrapper(tcl_interp)
        }
    }
}

impl Drop for Wrapper {
    fn drop(&mut self) {
        unsafe { Tcl_DeleteInterp(self.0) }
    }
}
