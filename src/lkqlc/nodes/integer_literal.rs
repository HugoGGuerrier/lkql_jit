/*
Functions for the integer literals in LKQL
*/

use crate::errors::LKQLError;
use crate::lkql_wrapper::lkql_base_entity;
use crate::lkqlc::env::CompilationEnv;


/// Compile a integer literal
pub unsafe fn compile(node: &mut lkql_base_entity, env: &mut CompilationEnv) -> Result<(), LKQLError>  {
    println!("TODO : Compile integer literal");

    Ok(())
}