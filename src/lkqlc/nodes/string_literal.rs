/*
Functions for the string literals
*/

use crate::lkql_wrapper::lkql_base_entity;
use crate::lkqlc::bc::KSTR;
use crate::lkqlc::env::CompilationEnv;
use crate::lkqlc::ir::{IRArg, IRInstAD, IRInstruction};
use crate::lkqlc::node_text;


/// Compile a string literal
pub unsafe fn compile(node: &mut lkql_base_entity, env: &mut CompilationEnv) {
    // Get the string value
    let full_str = node_text(node);
    let real_str = &full_str[1..full_str.len() - 1];

    // Add the string in the constant table
    let str_index = env.add_string_constant(String::from(real_str));

    // Add the instruction to the compilation result
    let inst = IRInstruction::AD(IRInstAD::new(
        KSTR,
        IRArg::Slot(env.get_expr_result()),
        IRArg::Str(str_index)
    ));
    env.add_instruction(inst);
}