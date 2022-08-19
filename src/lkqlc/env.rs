/*
This module define the lexical environment for the LKQL analysis and compilation
*/

use std::collections::{HashMap, HashSet};
use std::thread::sleep;
use crate::lkql_wrapper::{__syscall_slong_t, lkql_source_location};
use crate::lkqlc::bc::{FLAG_P_HAS_CHILD, BCInstABC, BCInstAD, BCInstruction, JUMP_BIASING, Program, Prototype, RET0, RET1, UCLO, ComplexConstant, KStr};
use crate::lkqlc::builtins::add_builtins;
use crate::lkqlc::ir::{IRArg, IRInstAD, IRInstruction, process_ir, Slot};


// --- Define the environment structures

#[derive(Debug)]
pub struct CompilationEnv {
    bytecode: Program,

    global_var: HashSet<String>,
    local_env_stack: Vec<LocalEnv>,

    module_name: String,
}

impl CompilationEnv {
    /// Create a new compilation environment
    pub fn new() -> CompilationEnv {
        let mut res = CompilationEnv {
            bytecode: Program::new(),

            global_var: HashSet::new(),
            local_env_stack: vec![LocalEnv::new(0, 0)],

            module_name: String::from("")
        };
        add_builtins(&mut res);

        res
    }

    /// Get the bytecode, result of the compilation
    pub fn get_bytecode(&self) -> Vec<u8> {
        self.bytecode.encode()
    }

    // --- Env management

    /// Open a new local environment and place it at the top of the stack
    pub fn open_env(&mut self, arg_count: u8) {
        let new_env = LocalEnv::new(self.local_env_stack.first().unwrap().depth, arg_count);
        self.local_env_stack.push(new_env);
    }

    /// Close the currently open local environment
    pub fn close_env(&mut self) {
        // Close the current local env and put it into the program
        let mut to_close = self.local_env_stack.pop().unwrap();
        to_close.finalize();
        self.bytecode.prototypes.push(to_close.prototype);

        // Tell the upper env that it has a child
        if !self.local_env_stack.is_empty() {
            self.local_env_stack.first_mut().unwrap().has_child = true;
        }
    }

    /// Open a new pseudo local environment
    pub fn open_pseudo_env(&mut self) {
        let mut local_env = self.local_env_stack.first_mut().unwrap();
        local_env.open_pseudo_env();
    }

    /// Close the current pseudo local environment
    pub fn close_pseudo_env(&mut self) {
        let mut local_env = self.local_env_stack.first_mut().unwrap();
        local_env.close_pseudo_env();
    }

    /// Just add the global symbol to the context
    pub fn add_global(&mut self, name: String) {
        self.global_var.insert(name);
    }

    /// Get if the global variable exists
    pub fn get_global(&self, name: &str) -> bool {
        self.global_var.contains(name)
    }

    /// Add the symbol to the local ones and return the associated slot or name
    pub fn add_local(&mut self, name: String) -> LocalResult {
        let mut local_env = self.local_env_stack.first_mut().unwrap();
        local_env.add_local(name)
    }

    /// Get the slot or local name for the original name, this only looks in the current local env
    pub fn get_local(&self, name: &str) -> LocalResult {
        let local_env = self.local_env_stack.first().unwrap();
        local_env.get_local(name)
    }

    /// Try to get the up value index for the given symbol, if none, try to get it in the upper env
    pub fn get_upvalue(&mut self, name: &str) -> UpvalueResult {
        self.lookup_uv(name, 0)
    }


    /// Internal function to lookup for an upvalue
    fn lookup_uv(&mut self, name: &str, depth: usize) -> UpvalueResult {
        // Get the local and upper environment and prepare the working variables
        let local_env = self.local_env_stack.get(depth).unwrap();
        let upper_env_opt = self.local_env_stack.get(depth + 1);

        let mut new_uv_reference: u16 = 0x0000;

        // Search in the already upvalues cache
        match local_env.get_upvalue(name) {
            UpvalueResult::Slot(slot) => { return UpvalueResult::Slot(slot); }
            _ => ()
        }

        // Verify if the upper environment exists
        if upper_env_opt.is_none() { return UpvalueResult::NotFound; }
        let upper_env = upper_env_opt.unwrap();

        // Search in the locals of the upper env
        let upper_local_res = upper_env.get_local(name);
        match upper_local_res {
            // If the name is an upper local
            LocalResult::Slot(slot) => {
                new_uv_reference = 0xC000 | (slot as u16);
            }

            // If the name is an upper pseudo-local
            LocalResult::Name(name) => {
                return UpvalueResult::Name(name);
            }

            // If the name is not in the upper env recurse the lookup in the upper env
            LocalResult::NotFound => {
                let upper_res = self.lookup_uv(name, depth + 1);
                match upper_res {
                    UpvalueResult::Slot(slot) => {
                        new_uv_reference = slot as u16;
                    }
                    _ => { return upper_res; }
                }
            }
        }

        // Create the upvalue in the local environment and return the newly created slot
        let local_env_mut = self.local_env_stack.get_mut(depth).unwrap();
        let new_index = local_env_mut.add_upvalue(new_uv_reference, String::from(name));
        return UpvalueResult::Slot(new_index);
    }

    /// Get the slot for the expression result
    pub fn get_expr_slot(&self) -> Option<u8> {
        self.local_env_stack.first().unwrap().expr_result_slot
    }

    /// Set the expression result slot
    pub fn set_expr_slot(&mut self, slot: Option<u8>) {
        self.local_env_stack.first_mut().unwrap().expr_result_slot = slot;
    }

    /// Get the slot to return
    pub fn get_return_slot(&self) -> Option<u8> {
        self.local_env_stack.first().unwrap().return_slot
    }

    /// Set the return slot
    pub fn set_return_slot(&mut self, slot: Option<u8>) {
        self.local_env_stack.first_mut().unwrap().return_slot = slot;
    }

    /// Get a temporary slot
    pub fn new_tmp(&mut self) -> u8 {
        let mut local_env = self.local_env_stack.first_mut().unwrap();
        local_env.new_tmp()
    }

    /// Get n temporary contiguous slots
    pub fn new_tmps(&mut self, n: u8) -> Vec<u8> {
        let mut local_env = self.local_env_stack.first_mut().unwrap();
        local_env.new_tmps(n)
    }

    /// Free an temporary used slot
    pub fn free_tmp(&mut self, slot: u8) {
        let mut local_env = self.local_env_stack.first_mut().unwrap();
        local_env.free_tmp(slot);
    }

    /// Free the temporary slots
    pub fn free_tmps(&mut self, slots: Vec<u8>) {
        let mut local_env = self.local_env_stack.first_mut().unwrap();
        for slot in slots {
            local_env.free_tmp(slot);
        }
    }

    // --- Instruction generation

    /// Get a new label
    pub fn new_label(&mut self) -> u64 {
        let mut local_env = self.local_env_stack.first_mut().unwrap();
        local_env.new_label()
    }

    /// Add an instruction to the current prototype
    pub fn add_instruction(&mut self, inst: IRInstruction) {
        let mut local_env = self.local_env_stack.first_mut().unwrap();
        local_env.add_instruction(inst);
    }

    // --- Constants

    /// Add a string constant to the current prototype and return its index
    pub fn add_string_constant(&mut self, string: String) -> u16 {
        let mut local_env = self.local_env_stack.first_mut().unwrap();
        local_env.add_string_constant(string)
    }
}


#[derive(Debug)]
pub struct LocalEnv {
    depth: usize, // The depth of the local environment

    // Those values are stack to create pseudo local environment
    occupied_slot: [bool; 256], // The array that indicates the currently occupied slots
    local_var_stack: Vec<HashMap<String, u8>>, // This map goes from the var name to the register index
    local_var_overflow_stack: Vec<HashSet<String>>, // This is a cheat to avoid the Lua local var limitation (200)

    string_constant_cache: HashMap<String, u16>, // The cache that goes from the String to the constant index

    expr_result_slot: Option<u8>, // The slot to put the result of the current expression in
    return_slot: Option<u8>, // The slot to return at the end of the fun (if -1 return nothing)

    upvalues: HashMap<String, u8>, // This map goes from the var name to the upvalue index

    frame_size: u8, // The frame size for the prototype
    label_counter: u64, // The counter for the jump labels
    has_child: bool, // If the local environment comport one or more child env

    ir: Vec<IRInstruction>, // The intermediary representation of the code
    prototype: Prototype, // The bytecode of the local environment
}

impl LocalEnv {
    /// Create a new local environment
    fn new(depth: usize, arg_count: u8) -> LocalEnv {
        LocalEnv {
            depth,

            occupied_slot: [false; 256],
            local_var_stack: vec![HashMap::new()],
            local_var_overflow_stack: vec![HashSet::new()],

            string_constant_cache: HashMap::new(),

            expr_result_slot: None,
            return_slot: None,

            upvalues: HashMap::new(),

            frame_size: 0,
            label_counter: 0,
            has_child: false,

            ir: Vec::new(),
            prototype: Prototype::new(arg_count)
        }
    }

    /// Finalize the local environment just before pushing it in the program
    fn finalize(&mut self) {
        // Return the result of the function TODO push it into the IR
        if self.return_slot.is_none() {
            self.ir.push(IRInstruction::AD(IRInstAD::new(
                RET0,
                IRArg::Slot(0),
                IRArg::Literal(1)
            )));
        } else {
            self.ir.push(IRInstruction::AD(IRInstAD::new(
                RET1,
                IRArg::Slot(self.return_slot.unwrap()),
                IRArg::Literal(2)
            )));
        }

        let mut code = process_ir(&mut self.ir);
        self.prototype.frame_size = self.frame_size;
        self.prototype.instructions.append(&mut code);

        // Set the prototype flags
        if self.has_child { self.prototype.flags |= FLAG_P_HAS_CHILD }
    }

    /// Open a pseudo local environment
    fn open_pseudo_env(&mut self) {
        // Push the new environment
        self.local_var_stack.push(HashMap::new());
        self.local_var_overflow_stack.push(HashSet::new());
    }

    /// Close the current pseudo local environment
    fn close_pseudo_env(&mut self) {
        // Pop all stack
        let local_var = self.local_var_stack.pop().unwrap();
        self.local_var_overflow_stack.pop();

        // Free the occupied local slots
        for (_, slot) in local_var {
            self.free_slot(slot);
        }
    }

    /// Get the next free slot and set it to occupied
    fn get_new_slot(&mut self) -> Option<u8> {
        for i in 0..self.occupied_slot.len() {
            if !self.occupied_slot[i] {
                self.occupied_slot[i] = true;
                let slot = i as u8;
                if slot >= self.frame_size { self.frame_size = slot + 1; }
                return Some(slot);
            }
        }

        None
    }

    /// Free the given slot
    fn free_slot(&mut self, slot: u8) {
        self.occupied_slot[slot as usize] = false;
    }

    /// Add a local variable to the local environment
    fn add_local(&mut self, name: String) -> LocalResult {
        // Get the next available slot
        let slot = self.get_new_slot().unwrap_or(255);

        // If the slot is over 220 then free it and get to the cheat local vars
        if slot >= 220 {
            // Free the slot
            self.free_slot(slot);

            // Get the local name and put it in the overflow set
            let local_var_overflow = self.local_var_overflow_stack.first_mut().unwrap();
            let depth_name = name_with_depth(&*name, self.depth);

            local_var_overflow.insert(depth_name.clone());
            LocalResult::Name(depth_name)
        } else {
            // Just add the slot in the local variable env
            let local_var = self.local_var_stack.first_mut().unwrap();
            local_var.insert(name, slot);
            LocalResult::Slot(slot)
        }
    }

    /// Get the local symbol associated slot or name
    fn get_local(&self, name: &str) -> LocalResult {
        // Prepare the depth name
        let depth_name = name_with_depth(name, self.depth);

        // Iterate on all pseudo local environment
        for i in 0..self.local_var_stack.len() {
            let local_var = self.local_var_stack.get(i).unwrap();
            if local_var.contains_key(name) {
                return LocalResult::Slot(*local_var.get(name).unwrap());
            } else {
                let local_var_overflow = self.local_var_overflow_stack.get(i).unwrap();
                if local_var_overflow.contains(name) { return LocalResult::Name(depth_name); }
            }
        }

        // Return the not found
        LocalResult::NotFound
    }

    /// Add a upvalue to the current local environment and return its index
    fn add_upvalue(&mut self, reference: u16, name: String) -> u8 {
        self.prototype.upval_references.push(reference);
        let index = (self.prototype.upval_references.len() - 1) as u8;
        self.upvalues.insert(name, index);
        index
    }

    /// Get the upvalue index in the local environment, or special value if not found
    fn get_upvalue(&self, name: &str) -> UpvalueResult {
        if self.upvalues.contains_key(name) {
            UpvalueResult::Slot(*(self.upvalues.get(name).unwrap()))
        } else {
            UpvalueResult::NotFound
        }
    }

    /// Create and return a new temporary slot
    fn new_tmp(&mut self) -> u8 {
        self.get_new_slot().expect("Cannot get new tmp, all slots are busy")
    }

    /// Get n contiguous slots
    fn new_tmps(&mut self, n: u8) -> Vec<u8> {
        // Prepare the working variables
        let mut start: Option<u8> = None;

        // Look for n contiguous slots
        for i in 0..self.occupied_slot.len() {
            if !self.occupied_slot[i] {
                if start.is_none() {
                    start = Some(i as u8);
                }

                if (i as u8) - start.unwrap() == n - 1 {
                    return (start.unwrap()..((i as u8) + 1)).collect()
                }
            } else {
                start = None;
            }
        }

        // Panic when there is not more contiguous slots
        panic!("Cannot get {} contiguous slots", n);
    }

    /// Free a temporary used slot
    fn free_tmp(&mut self, slot: u8) {
        self.free_slot(slot);
    }

    /// Get a new label for the labelled jumps
    fn new_label(&mut self) -> u64 {
        self.label_counter += 1;
        self.label_counter
    }

    /// Add an instruction to the list
    fn add_instruction(&mut self, inst: IRInstruction) {
        self.ir.push(inst);
    }

    /// Add the string constant and return its position
    fn add_string_constant(&mut self, string: String) -> u16 {
        // If the cache already contains the string just return its index
        if self.string_constant_cache.contains_key(&*string) {
            *self.string_constant_cache.get(&*string).unwrap()
        }

        // Else, create a new constant and put it in the cache
        else {
            let constant = ComplexConstant::String(KStr::new(
                string.clone()
            ));
            self.prototype.complex_constants.insert(0, constant);
            let res = (self.prototype.complex_constants.len() - 1) as u16;
            self.string_constant_cache.insert(string, res);
            res
        }
    }
}


// --- Return enums

pub enum LocalResult {
    Slot(u8),
    Name(String),
    NotFound
}

pub enum UpvalueResult {
    Slot(u8),
    Name(String),
    NotFound
}


// --- Util functions

/// Get the name of the variable with the wanted lexical depth
fn name_with_depth(name: &str, depth: usize) -> String {
    String::from("_").repeat(depth) + name
}