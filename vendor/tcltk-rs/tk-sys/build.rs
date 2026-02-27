use pkg_config::Config;
use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    let tcl_local = env::var("DEP_TCL_LOCAL_INCLUDE_DIR")
        .unwrap_or("DEP_TCL_LOCAL_INCLUDE_DIR env var not found".to_string());
    println!("cargo:warning={}", &tcl_local);
    if std::env::var("DOCS_RS").is_ok() {
        do_bindgen(&[&tcl_local, "tk8.6/generic"].map(PathBuf::from));
        return Ok(());
    }

    let include_paths = Config::new()
        .atleast_version("8.6")
        .probe("tk")?
        .include_paths;

    do_bindgen(&include_paths);

    Ok(())
}

fn do_bindgen(includes: &[PathBuf]) {
    let bindings = bindgen::Builder::default()
        .header("src/tk.h")
        .clang_args(
            includes
                .iter()
                .flat_map(|dir| vec!["-I", dir.to_str().unwrap()]),
        )
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Unable to write bindings");
}
