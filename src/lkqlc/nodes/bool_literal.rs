/*
Functions for the boolean literals
*/

use crate::errors::LKQLError;
use crate::lkql_wrapper::lkql_base_entity;
use crate::lkqlc::bc::KPRI;
use crate::lkqlc::env::CompilationEnv;
use crate::lkqlc::ir::{IRArg, IRInstAD, IRInstruction, Primitive};


/// Compile a true literal
pub unsafe fn compile_true(node: &mut lkql_base_entity, env: &mut CompilationEnv) -> Result<(), LKQLError> {
    let expr_slot = env.get_expr_slot();
    if expr_slot.is_some() {
        env.add_instruction(IRInstruction::AD(IRInstAD::new(
            KPRI,
            IRArg::Slot(expr_slot.unwrap()),
            IRArg::Primitive(Primitive::True)
        )));
    }

    Ok(())
}

/// Compile a false literal
pub unsafe fn compile_false(node: &mut lkql_base_entity, env: &mut CompilationEnv) -> Result<(), LKQLError>  {
    let expr_slot = env.get_expr_slot();
    if expr_slot.is_some() {
        env.add_instruction(IRInstruction::AD(IRInstAD::new(
            KPRI,
            IRArg::Slot(expr_slot.unwrap()),
            IRArg::Primitive(Primitive::False)
        )));
    }

    Ok(())
}