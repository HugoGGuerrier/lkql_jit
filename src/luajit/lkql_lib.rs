/*
This module contains all functions and utils to register the LKQL standard library
in the lua context
*/

use std::ffi::{c_void, CString};
use std::os::raw::{c_char, c_int};


// --- Define the c function signatures

extern "C" {
    fn lua_pushcclosure(l: *mut c_void, c_fn: unsafe extern "C" fn(*mut c_void) -> c_int, n: c_int);
    fn lua_setfield(l: *mut c_void, index: c_int, key: *const c_char);
}

// --- Global functions for lkql

/// The LKQL printing function
#[no_mangle]
pub unsafe extern "C" fn lkql_print(l: *mut c_void) -> c_int {
    println!("This is my LKQL printing funtions");
    0
}


// --- List for the library definition

const FUNC_NAMES: [&str; 1] = [
    "print"
];
const FUNC_REF: [unsafe extern "C" fn(*mut c_void) -> c_int; 1] = [
    lkql_print
];


// --- Util functions

/// Load the LKQL library in the lua context
pub unsafe fn lkql_openlib(l: *mut c_void) {
    // Put the global functions to the lua context
    for i in 0..FUNC_NAMES.len() {
        let name = CString::new(FUNC_NAMES[i]).unwrap();
        lua_pushcclosure(l, FUNC_REF[i], 0);
        lua_setfield(l, -10002, name.as_ptr());
    }
}