/*
Functions for the function call nodes
*/

use crate::errors::LKQLError;
use crate::lkql_wrapper::{lkql_base_entity, lkql_fun_call_f_arguments, lkql_fun_call_f_name};
use crate::lkqlc::env::{CompilationEnv, LocalResult};
use crate::lkqlc::{compile_node, load_var_copy, new_node, node_text};
use crate::lkqlc::bc::{CALL, MOV};
use crate::lkqlc::ir::{IRArg, IRInstABC, IRInstAD, IRInstruction};


/// Compile a function call node to LuaJIT bytecode
pub unsafe fn compile(node: &mut lkql_base_entity, env: &mut CompilationEnv) -> Result<(), LKQLError>  {
    // Get the two slots for the function and the argument table
    let slots = env.new_tmps(2);
    let res_slot = env.get_expr_slot();
    let fun_slot = slots[0];
    let arg_slot = slots[1];

    // Get the function name
    let mut fun_id = new_node();
    lkql_fun_call_f_name(node, &mut fun_id);
    let fun_name = node_text(&mut fun_id);

    // Load the function variable in the slot
    env.set_expr_slot(Some(fun_slot));
    load_var_copy(&*fun_name, env);

    // Get the function argument list
    let mut arg_list = new_node();
    lkql_fun_call_f_arguments(node, &mut arg_list);

    // Process the arguments to create the calling table
    env.set_expr_slot(Some(arg_slot));
    match compile_node(&mut arg_list, env) {
        Err(e) => { return Err(e); }
        Ok(_) => {}
    }

    // Call the function
    env.add_instruction(IRInstruction::ABC(IRInstABC::new(
        CALL,
        IRArg::Slot(fun_slot),
        IRArg::Literal(1),
        IRArg::Literal(2)
    )));

    // Set the expression result
    if res_slot.is_some() {
        env.add_instruction(IRInstruction::AD(IRInstAD::new(
            MOV,
            IRArg::Slot(res_slot.unwrap()),
            IRArg::Slot(fun_slot)
        )));
    }

    // Free the temporary slots
    env.free_tmps(slots);

    // Reset the expr slot
    env.set_expr_slot(res_slot);

    Ok(())
}