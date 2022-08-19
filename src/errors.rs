/*
This module contains all error types for the LKQL compilation and execution
*/

use crate::lkql_wrapper::lkql_base_entity;


// --- The structure to represents an error in LKQL

pub struct LKQLError {
    pub message: String
}

impl LKQLError {
    // --- Creation methods ---

    /// Create a new exception just with its message
    pub fn new(message: String) -> LKQLError {
        LKQLError {
            message
        }
    }
}