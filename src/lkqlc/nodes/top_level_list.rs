/*
Functions for the top level list node
*/

use std::os::raw::c_uint;
use crate::errors::LKQLError;
use crate::lkql_wrapper::{lkql_base_entity, lkql_node_child, lkql_node_children_count};
use crate::lkqlc::env::CompilationEnv;
use crate::lkqlc::{compile_node, new_node};


/// Compile a top level list node
pub unsafe fn compile(node: &mut lkql_base_entity, env: &mut CompilationEnv) -> Result<(), LKQLError> {
    // Compile all children
    let children_count = lkql_node_children_count(node);
    let mut i: c_uint = 0;
    while i < children_count {
        let mut child = new_node();
        lkql_node_child(node, i, &mut child);
        match compile_node(&mut child, env) {
            Err(e) => { return Err(e); }
            Ok(_) => {}
        }
        i += 1;
    }

    Ok(())
}