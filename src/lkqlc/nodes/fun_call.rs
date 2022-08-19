/*
Functions for the function call nodes
*/

use crate::lkql_wrapper::{lkql_base_entity, lkql_fun_call_f_name};
use crate::lkqlc::env::{CompilationEnv, LocalResult};
use crate::lkqlc::{new_node, node_text};


/// Compile a function call node to LuaJIT bytecode
pub unsafe fn compile(node: &mut lkql_base_entity, env: &mut CompilationEnv) {
    // Get the two slots for the function and the argument table
    let slots = env.new_tmps(2);
    let fun_slot = slots[0];
    let arg_slot = slots[1];

    // Get the function name
    let mut fun_id = new_node();
    lkql_fun_call_f_name(node, &mut fun_id);
    let fun_name = node_text(&mut fun_id);

    match env.get_local(&*fun_name) {
        LocalResult::Slot(_) => {}
        LocalResult::Name(_) => {}
        LocalResult::NotFound => {}
    }

    // Add the function name to the env
    let fun_name_id = env.add_string_constant(fun_name);

    // Free the temporary slots

    println!("Function call");
}