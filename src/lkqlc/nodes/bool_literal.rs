/*
Functions for the boolean literals
*/

use crate::lkql_wrapper::lkql_base_entity;
use crate::lkqlc::bc::KPRI;
use crate::lkqlc::env::CompilationEnv;
use crate::lkqlc::ir::{IRArg, IRInstAD, IRInstruction, Primitive};


/// Compile a true literal
pub unsafe fn compile_true(node: &mut lkql_base_entity, env: &mut CompilationEnv) {
    let inst = IRInstruction::AD(IRInstAD::new(
        KPRI,
        IRArg::Slot(env.get_expr_result()),
        IRArg::Primitive(Primitive::True)
    ));
    env.add_instruction(inst);
}

/// Compile a false literal
pub unsafe fn compile_false(node: &mut lkql_base_entity, env: &mut CompilationEnv) {
    let inst = IRInstruction::AD(IRInstAD::new(
        KPRI,
        IRArg::Slot(env.get_expr_result()),
        IRArg::Primitive(Primitive::False)
    ));
    env.add_instruction(inst);
}