use std::env;
use std::path::PathBuf;
use std::process::Command;

// !!! Change this path to be able to compiler LKQL JIT !!!
const PATH_TO_LKQL_LIB_DIR: &str = "/home/guerrier/Documents/AdaCore/langkit-query-language/lkql/build/lib/relocatable/prod";

fn main() {
    // Make the lua jit library
    Command::new("make")
        .arg("-C")
        .arg("./lua_jit")
        .output()
        .expect("Failed to build Lua JIT");

    // Link the static lua jit library
    println!("cargo:rustc-link-search=all=./lua_jit/src");
    println!("cargo:rustc-link-lib=static=luajit");

    // Link the static lkql lang langkit library
    println!("cargo:rustc-link-search=all={}", PATH_TO_LKQL_LIB_DIR);
    println!("cargo:rustc-link-lib=dylib=lkqllang");

    // Generate the lkql bindings
    let bindings = bindgen::Builder::default()
        .header("lkql_wrapper/wrapper.h")
        .layout_tests(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Failed to generate the LKQL bindings");

    let out_path = PathBuf::from("./src");
    bindings
        .write_to_file(out_path.join("lkql_wrapper.rs"))
        .expect("Failed to write the LKQL bindings");
}
