/*
Rust module that holds the interface with luajit library
All luajit calls should be done here
*/

mod lkql_lib;

use std::ffi::{c_void, CString};
use std::os::raw::{c_char, c_int};
use crate::luajit::lkql_lib::lkql_openlib;


// --- Define the c function signatures

extern "C" {
    fn luaL_newstate() -> *mut c_void;
    fn luaL_openlibs(state: *mut c_void);
    fn luaL_loadfile(state: *mut c_void, file: *const c_char) -> c_int;
    fn luaL_loadbuffer(state: *mut c_void, buffer: *const c_char, size: usize, name: *const c_char) -> c_int;
    fn lua_call(state: *mut c_void, nargs: c_int, nresults: c_int) -> c_int;
    fn lua_close(state: *mut c_void);
}


// --- Defining the module structures

pub struct LuaState {
    state: *mut c_void,
}


// --- Defining the functions to control the lkql JIT

/// Function to initialize the lua interpreter
pub fn init_env() -> LuaState {
    unsafe {
        // Initialize the lua state and load the libraries
        let state = luaL_newstate();
        luaL_openlibs(state);
        lkql_openlib(state);
        LuaState {
            state,
        }
    }
}

/// Close the lua environment
pub fn close_env(l: &LuaState) {
    unsafe {
        lua_close(l.state);
    }
}

/// Function to run a lua script (DEBUG)
pub fn run_lua_script(l: &LuaState, file: &str) {
    let file_c = CString::new(file).unwrap();
    unsafe {
        let load_res = luaL_loadfile(l.state, file_c.as_ptr());
        if load_res != 0 {
            panic!("Cannot load the Lua script");
        }


        let result = lua_call(l.state, 0, -1);
        if result != 0 {
            panic!("Failed to run the lua script");
        }
    }
}

/// Function to run a lua buffer (DEBUG)
pub fn run_lua_buffer(l: &LuaState, buffer: &str, name: &str) {
    let buffer_c = CString::new(buffer).unwrap();
    let name_c = CString::new(name).unwrap();
    unsafe {
        let load_res = luaL_loadbuffer(l.state, buffer_c.as_ptr(), buffer.len(), name_c.as_ptr());
        if load_res != 0 {
            panic!("Cannot load the buffer");
        }

        let result = lua_call(l.state, 0, -1);
        if result != 0 {
            panic!("Failed to run the lua script");
        }
    }
}

/// Function to run a lua bytecode buffer
pub fn run_lua_bytecode(l: &LuaState, bytecode: &Vec<u8>, name: &str) {
    let buffer_c = bytecode.as_ptr() as *const c_char;
    let name_c = CString::new(name).unwrap();
    unsafe {
        let load_res = luaL_loadbuffer(l.state, buffer_c, bytecode.len(), name_c.as_ptr());
        if load_res != 0 {
            panic!("Cannot load the buffer");
        }

        let result = lua_call(l.state, 0, -1);
        if result != 0 {
            panic!("Failed to run the lua script");
        }
    }
}
