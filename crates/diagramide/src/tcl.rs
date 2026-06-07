use std::{
    ffi::{CStr, CString},
    panic,
    sync::{
        Arc, Condvar, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
use tcl_sys::*;
use tokio::task;

static TCL_LIB: OnceLock<Option<libloading::Library>> = OnceLock::new();
pub const DEFAULT_TCL_TIMEOUT: Duration = Duration::from_secs(2);
pub const DEFAULT_TCL_COMMAND_LIMIT: u32 = 1_000_000;

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
                    "libtcl9.0.dylib",
                    "/opt/homebrew/opt/tcl-tk/lib/libtcl9.0.dylib",
                    "/usr/local/opt/tcl-tk/lib/libtcl9.0.dylib",
                    "libtcl8.6.dylib",
                    "/opt/homebrew/opt/tcl-tk/lib/libtcl8.6.dylib",
                    "/usr/local/opt/tcl-tk/lib/libtcl8.6.dylib",
                    "libtcl.dylib",
                ]
            } else {
                vec!["libtcl8.6.so", "libtcl.so"]
            };

            candidates.into_iter().find_map(|name| unsafe {
                let library = libloading::Library::new(name).ok()?;
                library
                    .get::<unsafe extern "C" fn()>(b"Tcl_CancelEval\0")
                    .ok()?;
                Some(library)
            })
        })
        .is_some()
}

pub async fn safe_eval_tcl(script: String) -> Result<String, String> {
    safe_eval_tcl_with_timeout(script, DEFAULT_TCL_TIMEOUT).await
}

pub async fn safe_eval_tcl_with_timeout(
    script: String,
    timeout: Duration,
) -> Result<String, String> {
    safe_eval_tcl_with_limits(script, timeout, DEFAULT_TCL_COMMAND_LIMIT).await
}

pub async fn safe_eval_tcl_with_limits(
    script: String,
    timeout: Duration,
    command_limit: u32,
) -> Result<String, String> {
    if !is_tcl_loadable() {
        return Err("Compatible Tcl 8.6 shared library not found".to_string());
    }

    task::spawn_blocking(move || {
        panic::catch_unwind(|| eval_tcl_with_limits(&script, timeout, command_limit))
            .map_err(|_| "Tcl interpreter panicked or crashed".to_string())?
    })
    .await
    .map_err(|e| e.to_string())?
}

#[allow(unused)]
pub fn eval_tcl(script: &str) -> Result<String, String> {
    eval_tcl_with_timeout(script, DEFAULT_TCL_TIMEOUT)
}

#[allow(unused)]
pub fn eval_tcl_with_timeout(script: &str, timeout: Duration) -> Result<String, String> {
    eval_tcl_with_limits(script, timeout, DEFAULT_TCL_COMMAND_LIMIT)
}

pub fn eval_tcl_with_limits(
    script: &str,
    timeout: Duration,
    command_limit: u32,
) -> Result<String, String> {
    type TclFindExecFn = unsafe extern "C" fn(*const std::os::raw::c_char);
    type TclCreateInterpFn = unsafe extern "C" fn() -> *mut Tcl_Interp;
    type TclDeleteInterpFn = unsafe extern "C" fn(*mut Tcl_Interp);
    type TclEvalExFn =
        unsafe extern "C" fn(*mut Tcl_Interp, *const std::os::raw::c_char, i32, i32) -> i32;
    type TclGetObjResFn = unsafe extern "C" fn(*mut Tcl_Interp) -> *mut Tcl_Obj;
    type TclGetStrFromObjFn =
        unsafe extern "C" fn(*mut Tcl_Obj, *mut i32) -> *mut std::os::raw::c_char;
    type TclGetTimeFn = unsafe extern "C" fn(*mut Tcl_Time);
    type TclLimitSetTimeFn = unsafe extern "C" fn(*mut Tcl_Interp, *mut Tcl_Time);
    type TclLimitSetCommandsFn = unsafe extern "C" fn(*mut Tcl_Interp, std::os::raw::c_int);
    type TclCancelEvalFn = unsafe extern "C" fn(
        *mut Tcl_Interp,
        *mut Tcl_Obj,
        ClientData,
        std::os::raw::c_int,
    ) -> std::os::raw::c_int;

    let tcl_find_executable = get_sym!(Tcl_FindExecutable, TclFindExecFn);
    let tcl_create_interp = get_sym!(Tcl_CreateInterp, TclCreateInterpFn);
    let tcl_delete_interp = get_sym!(Tcl_DeleteInterp, TclDeleteInterpFn);
    let tcl_eval_ex = get_sym!(Tcl_EvalEx, TclEvalExFn);
    let tcl_get_obj_result = get_sym!(Tcl_GetObjResult, TclGetObjResFn);
    let tcl_get_string_from_obj = get_sym!(Tcl_GetStringFromObj, TclGetStrFromObjFn);
    let tcl_get_time = get_sym!(Tcl_GetTime, TclGetTimeFn);
    let tcl_limit_set_time = get_sym!(Tcl_LimitSetTime, TclLimitSetTimeFn);
    let tcl_limit_set_commands = get_sym!(Tcl_LimitSetCommands, TclLimitSetCommandsFn);
    let tcl_cancel_eval = *get_sym!(Tcl_CancelEval, TclCancelEvalFn);
    unsafe {
        tcl_find_executable(std::ptr::null());

        let interp = tcl_create_interp();
        if interp.is_null() {
            return Err("Failed to create Tcl interpreter".to_string());
        }

        let c_script = CString::new(script).unwrap();

        let mut deadline = Tcl_Time { sec: 0, usec: 0 };
        tcl_get_time(&mut deadline);
        add_duration(&mut deadline, timeout);
        tcl_limit_set_time(interp, &mut deadline);
        tcl_limit_set_commands(interp, command_limit.min(i32::MAX as u32) as i32);

        // Native limits are checked at evaluator checkpoints; cancellation also stops tight bytecode loops.
        let completed = Arc::new((Mutex::new(false), Condvar::new()));
        let timed_out = Arc::new(AtomicBool::new(false));
        let watchdog_completed = completed.clone();
        let watchdog_timed_out = timed_out.clone();
        let interp_address = interp as usize;
        let watchdog = thread::spawn(move || {
            let (lock, condvar) = &*watchdog_completed;
            let completed = lock.lock().expect("Tcl watchdog mutex poisoned");
            let wait = condvar
                .wait_timeout_while(completed, timeout, |completed| !*completed)
                .expect("Tcl watchdog mutex poisoned");

            if wait.1.timed_out() {
                watchdog_timed_out.store(true, Ordering::Release);
                tcl_cancel_eval(
                    interp_address as *mut Tcl_Interp,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    TCL_CANCEL_UNWIND as i32,
                );
            }
        });
        let code = tcl_eval_ex(interp, c_script.as_ptr() as *const _, -1, 0);
        {
            let (lock, condvar) = &*completed;
            *lock.lock().expect("Tcl watchdog mutex poisoned") = true;
            condvar.notify_one();
        }
        watchdog.join().expect("Tcl watchdog thread panicked");

        let obj_result = tcl_get_obj_result(interp);
        let result_ptr = tcl_get_string_from_obj(obj_result, std::ptr::null_mut());

        let result = CStr::from_ptr(result_ptr as *const _)
            .to_string_lossy()
            .into_owned();

        tcl_delete_interp(interp);

        if timed_out.load(Ordering::Acquire) {
            Err("Tcl execution timed out".to_string())
        } else if code == TCL_OK as i32 {
            Ok(result)
        } else {
            Err(result)
        }
    }
}

fn add_duration(time: &mut Tcl_Time, duration: Duration) {
    let seconds = duration.as_secs().min(std::os::raw::c_long::MAX as u64) as std::os::raw::c_long;
    let microseconds = duration.subsec_micros() as std::os::raw::c_long;

    time.sec = time.sec.saturating_add(seconds);
    time.usec += microseconds;
    if time.usec >= 1_000_000 {
        time.sec = time.sec.saturating_add(1);
        time.usec -= 1_000_000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_allows_terminating_tcl() {
        if !is_tcl_loadable() {
            return;
        }

        assert_eq!(
            eval_tcl_with_timeout("expr {20 + 22}", Duration::from_secs(1)).unwrap(),
            "42"
        );
    }

    #[test]
    fn timeout_stops_non_terminating_tcl() {
        if !is_tcl_loadable() {
            return;
        }

        let error = eval_tcl_with_timeout("while {1} {}", Duration::from_millis(10))
            .expect_err("infinite Tcl loop should exceed its time limit");

        assert_eq!(error, "Tcl execution timed out");
    }

    #[test]
    fn command_limit_stops_runaway_tcl() {
        if !is_tcl_loadable() {
            return;
        }

        let error = eval_tcl_with_limits("while {1} {set value 1}", Duration::from_secs(1), 100)
            .expect_err("runaway Tcl loop should exhaust its command limit");

        assert!(error.contains("command count limit exceeded"), "{error}");
    }
}
