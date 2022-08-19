/*
This module define all builtin symbols for the lkql environment
It also contains all functions to fill environments
*/


// --- Symbols

use crate::lkqlc::env::CompilationEnv;

const BUILD_IN_FUNC: [&str; 1] = [
    "print"
];

// --- Util functions

/// Fill a compilation environment with the global symbols
pub fn add_builtins(env: &mut CompilationEnv) {
    for func_name in BUILD_IN_FUNC {
        env.add_global(String::from(func_name));
    }
}
