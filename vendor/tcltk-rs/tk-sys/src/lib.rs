//! Rust bindings for the Tk Graphics library
//!
//! Function specific documentation can be found at the [Tcl website]
//!
//! [Tcl website]: https://www.tcl-lang.org/man/tcl8.6/TkLib/contents.htm
//!
//! ## Note
//! This library only seems to work if it's ran in an enviroment that has a
//! graphics shell.
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
