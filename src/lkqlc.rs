/*
This module contains all functions to compile LKQL sources to luajit bytecode
*/

pub mod env;
pub mod bc;
pub mod builtins;
pub mod ir;
pub mod nodes;

use std::ffi::CString;
use std::os::raw::c_uint;
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use widestring::U32String;
use crate::Cli;
use crate::errors::LKQLError;
use crate::lkql_wrapper::*;
use crate::lkqlc::bc::{GGET, KPRI, KSTR, MOV, UGET};
use crate::lkqlc::env::{CompilationEnv, LocalResult, UpvalueResult};
use crate::lkqlc::ir::{IRArg, IRInstAD, IRInstruction, Primitive};


// --- Entry points of the compiler

/// Compile the given buffer in the appropriate bytecode
pub fn compile_lkql_buffer(buffer: &str, name: &str) -> Vec<u8> {
    let env = CompilationEnv::new();
    // TODO : Add the LKQL buffer compilation
    env.get_bytecode()
}

/// Open and compile the given file to LuaJIT bytecode
pub fn compile_lkql_file(file: &PathBuf, charset: &Option<String>) -> Result<Vec<u8>, LKQLError> {
    unsafe {
        // Create the lkql context
        let ctx = lkql_create_analysis_context(
            null(),
            null_mut(),
            null_mut(),
            null_mut(),
            1,
            8
        );

        // Get the LKQL script and the charset
        let file_path_c = CString::new(
            file
                .canonicalize()
                .unwrap()
                .to_str()
                .unwrap()
        ).unwrap();

        let charset_c = CString::new(
            charset.as_ref().unwrap_or(&String::from("NULL")).as_str()
        ).unwrap();

        // Create the analysis unit from the LKQL file
        let unit = lkql_get_analysis_unit_from_file(
            ctx,
            file_path_c.as_ptr(),
            (if charset.is_none() {null()} else {charset_c.as_ptr()}),
            0,
            lkql_grammar_rule_LKQL_GRAMMAR_RULE_MAIN_RULE_RULE
        );

        // Get the unit root node
        let mut root = new_node();
        lkql_unit_root(unit, &mut root);

        // Compile the LKQL AST
        let mut env = CompilationEnv::new();
        match compile_node(&mut root, &mut env) {
            Err(e) => { return Err(e); }
            Ok(_) => {}
        }
        env.close_env();

        // Return the bytecode for the LKQL file
        Ok(env.get_bytecode())
    }
}


// --- The BIG dispatching function

/// Dispatch the node compilation
unsafe fn compile_node(node: &mut lkql_base_entity, env: &mut CompilationEnv) -> Result<(), LKQLError> {
    let kind = lkql_node_kind(node);
    match kind {
        // -- Top level node
        lkql_node_kind_enum_lkql_top_level_list => nodes::top_level_list::compile(node, env),

        // -- Expressions
        lkql_node_kind_enum_lkql_fun_call => nodes::fun_call::compile(node, env),

        // -- Literals
        lkql_node_kind_enum_lkql_bool_literal_true => nodes::bool_literal::compile_true(node, env),
        lkql_node_kind_enum_lkql_bool_literal_false => nodes::bool_literal::compile_false(node, env),
        lkql_node_kind_enum_lkql_integer_literal => nodes::integer_literal::compile(node, env),
        lkql_node_kind_enum_lkql_string_literal => nodes::string_literal::compile(node, env),

        // -- Default result is an error
        _ => panic!("Unknown node kind ({}), cannot proceed to compilation", kind)
    }
}


// --- Utils functions

/// Create a new entity structure
fn new_node() -> lkql_base_entity {
    lkql_base_entity {
        node: null_mut(),
        info: lkql_internal_entity_info {
            md: lkql_internal_metadata {},
            rebindings: null_mut(),
            from_rebound: 0
        }
    }
}

/// Get the text of a node
unsafe fn node_text(node: &mut lkql_base_entity) -> String {
    let mut text = new_text();
    lkql_node_text(node, &mut text);
    text_to_string(&text)
}

/// Test if the given node is a constant
unsafe fn node_is_literal(node: &mut lkql_base_entity) -> bool {
    let kind = lkql_node_kind(node);
    match kind {
        lkql_node_kind_enum_lkql_null_literal |
        lkql_node_kind_enum_lkql_unit_literal |
        lkql_node_kind_enum_lkql_bool_literal_true |
        lkql_node_kind_enum_lkql_bool_literal_false |
        lkql_node_kind_enum_lkql_integer_literal |
        lkql_node_kind_enum_lkql_string_literal |
        lkql_node_kind_enum_lkql_list_literal |
        lkql_node_kind_enum_lkql_object_literal |
        lkql_node_kind_enum_lkql_block_string_literal => true,
        _ => false
    }
}

/// Create a new text structure
fn new_text() -> lkql_text {
    lkql_text {
        chars: null_mut(),
        length: 0,
        is_allocated: 0
    }
}

/// Translate an LKQL text to a string
unsafe fn text_to_string(text: &lkql_text) -> String {
    let decoded = U32String::from_ptr(text.chars, text.length as usize);
    decoded.to_string().expect("Cannot decode the UTF-32 string")
}

/// Get the node kind as a string
unsafe fn node_kind(node: &mut lkql_base_entity) -> String {
    let mut text = new_text();
    lkql_kind_name(lkql_node_kind(node), &mut text);
    text_to_string(&text)
}

/// Load the needed variable in the expression slot for a read purpose
/// If the var is already in a slot just set the expr return slot to this one
fn load_var(name: &str, env: &mut CompilationEnv) -> bool {
    // Try to get the local variable
    match env.get_local(name) {
        LocalResult::Slot(slot) => {
            env.set_expr_slot(Some(slot))
        }
        LocalResult::Name(name) => {
            // Add the name in the constant table
            let name_index = env.add_string_constant(String::from(name));

            // Add the global getting
            env.add_instruction(IRInstruction::AD(IRInstAD::new(
                GGET,
                IRArg::Slot(env.get_expr_slot().unwrap()),
                IRArg::Str(name_index)
            )));
        }
        LocalResult::NotFound => {
            // Try to get the variable in the upvalues
            match env.get_upvalue(name) {
                UpvalueResult::Slot(uv_slot) => {
                    // Add the up value getting
                    env.add_instruction(IRInstruction::AD(IRInstAD::new(
                        UGET,
                        IRArg::Slot(env.get_expr_slot().unwrap()),
                        IRArg::Upvalue(uv_slot)
                    )));
                }
                UpvalueResult::Name(name) => {
                    // Add the name in the constant table
                    let name_index = env.add_string_constant(String::from(name));

                    // Add the global getting
                    env.add_instruction(IRInstruction::AD(IRInstAD::new(
                        GGET,
                        IRArg::Slot(env.get_expr_slot().unwrap()),
                        IRArg::Str(name_index)
                    )));
                }
                UpvalueResult::NotFound => {
                    // Try to get the variable in the global scope
                    if env.get_global(name) {
                        // Add the name in the constant table
                        let name_index = env.add_string_constant(String::from(name));

                        // Add the global getting
                        env.add_instruction(IRInstruction::AD(IRInstAD::new(
                            GGET,
                            IRArg::Slot(env.get_expr_slot().unwrap()),
                            IRArg::Str(name_index)
                        )));
                    } else {
                        // Return the failure, cannot load the variable
                        return false;
                    }
                }
            }
        }
    }

    // Return the success
    true
}

/// Load the needed variable in the expression slot for write purpose (always copy)
fn load_var_copy(name: &str, env: &mut CompilationEnv) -> bool {
    // Try to get the local variable
    match env.get_local(name) {
        LocalResult::Slot(slot) => {
            // Copy the value from the local variable
            env.add_instruction(IRInstruction::AD(IRInstAD::new(
                MOV,
                IRArg::Slot(env.get_expr_slot().unwrap()),
                IRArg::Slot(slot)
            )));
        }
        LocalResult::Name(name) => {
            // Add the name in the constant table
            let name_index = env.add_string_constant(String::from(name));

            // Add the global getting
            env.add_instruction(IRInstruction::AD(IRInstAD::new(
                GGET,
                IRArg::Slot(env.get_expr_slot().unwrap()),
                IRArg::Str(name_index)
            )));
        }
        LocalResult::NotFound => {
            // Try to get the variable in the upvalues
            match env.get_upvalue(name) {
                UpvalueResult::Slot(uv_slot) => {
                    // Add the up value getting
                    env.add_instruction(IRInstruction::AD(IRInstAD::new(
                        UGET,
                        IRArg::Slot(env.get_expr_slot().unwrap()),
                        IRArg::Upvalue(uv_slot)
                    )));
                }
                UpvalueResult::Name(name) => {
                    // Add the name in the constant table
                    let name_index = env.add_string_constant(String::from(name));

                    // Add the global getting
                    env.add_instruction(IRInstruction::AD(IRInstAD::new(
                        GGET,
                        IRArg::Slot(env.get_expr_slot().unwrap()),
                        IRArg::Str(name_index)
                    )));
                }
                UpvalueResult::NotFound => {
                    // Try to get the variable in the global scope
                    if env.get_global(name) {
                        // Add the name in the constant table
                        let name_index = env.add_string_constant(String::from(name));

                        // Add the global getting
                        env.add_instruction(IRInstruction::AD(IRInstAD::new(
                            GGET,
                            IRArg::Slot(env.get_expr_slot().unwrap()),
                            IRArg::Str(name_index)
                        )));
                    } else {
                        // Return the failure, cannot load the variable
                        return false;
                    }
                }
            }
        }
    }

    // Return the success
    true
}
