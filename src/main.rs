/*
This module is the root module of the project.
It contains the entry point of LKQL JIT ("fn main()") and the argument parsing logic
*/

extern crate core;

pub mod luajit;
pub mod lkql_wrapper;
pub mod lkqlc;
pub mod errors;

use std::path::PathBuf;
use clap::{CommandFactory, ErrorKind, Parser};


// --- Defining the arguments structure

#[derive(Parser)]
#[clap(name = "LKQL JIT")]
#[clap(version)]
#[clap(author)]
#[clap(about = "The JIT compiled implementation of LKQL", long_about = "The LKQL implementation that use the LuaJIT project to perform compilation")]
pub struct Cli {
    /// Charset to use for source decoding
    #[clap(short = 'C', long = "charset", value_parser, value_name = "CHARSET")]
    charset: Option<String>,

    /// Project file to use
    #[clap(short = 'P', long = "project", value_parser, value_name = "FILE")]
    project_file: Option<PathBuf>,

    /// Path of the LKQL script to evaluate
    #[clap(short = 'S', long = "script-path", value_parser, value_name = "FILE")]
    script_file: PathBuf,

    /// Files to analyze
    #[clap(value_parser)]
    files: Vec<PathBuf>,

    /// If the bytecode is showed just before the interpretation
    #[clap(short = 'b', long = "bytecode")]
    show_bc: bool,
}


// --- Defining the entry point of the application

use lkqlc::bc::NumericConstant;
use crate::errors::LKQLError;
use crate::lkqlc::bc::KNum;

// The main entry point !
fn main() {
    // Parse the arguments
    let args: Cli = Cli::parse();
    let mut cmd = Cli::command();

    // Verify that there is at least a project file or one file to analyse
    if args.files.len() == 0 && (args.project_file.as_ref().is_none()) {
        cmd.error(
            ErrorKind::MissingRequiredArgument,
            "Provide at least a file to analyse or a project file",
        ).exit();
    }

    // Verify that the project file is a valid file
    if !args.project_file.is_none() && !args.project_file.as_ref().unwrap().is_file() {
        cmd.error(
            ErrorKind::Io,
            "Provided project file not found",
        ).exit();
    }

    // Verify that the script file is a valid file
    if !args.script_file.is_file() {
        cmd.error(
            ErrorKind::Io,
            "Provided script file not found",
        ).exit();
    }

    // Get the LuaJIT bytecode for the lkql script
    match lkqlc::compile_lkql_file(&args.script_file, &args.charset) {
        Err(e) => {
            eprintln!("{}", e.message);
        }
        Ok(bytecode) => {
            if args.show_bc {
                println!("GENERATED BYTECODE : \n{:X?}", bytecode)
            }
            // TODO : Start the LuaJIT with the generated bytecode
        }
    }
}
