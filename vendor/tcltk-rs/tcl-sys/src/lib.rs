#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
mod bindings { include!("bindings/aarch64-apple-darwin.rs"); }

#[cfg(all(target_arch = "aarch64", target_os = "windows"))]
mod bindings { include!("bindings/aarch64-pc-windows-gnullvm.rs"); }

#[cfg(all(target_arch = "aarch64", target_os = "linux", target_env = "musl"))]
mod bindings { include!("bindings/aarch64-unknown-linux-musl.rs"); }

#[cfg(all(target_arch = "x86_64", target_os = "macos"))]
mod bindings { include!("bindings/x86_64-apple-darwin.rs"); }

#[cfg(all(target_arch = "x86_64", target_os = "windows"))]
mod bindings { include!("bindings/x86_64-pc-windows-gnu.rs"); }

#[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "musl"))]
mod bindings { include!("bindings/x86_64-unknown-linux-musl.rs"); }

pub use bindings::*;
