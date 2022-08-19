/*
This module contains all needed functions and structure to create the intermediary
representation between LKQL and LuaJIT bytecode
*/


// --- Enum that contains the IR instruction

use std::mem::replace;
use crate::lkqlc::bc::{BCInstABC, BCInstAD, BCInstruction, JUMP_BIASING};

#[derive(Debug)]
pub enum IRInstruction {
    ABC(IRInstABC),
    AD(IRInstAD)
}

impl IRInstruction {
    pub fn to_bc_instruction(&self) -> BCInstruction {
        match self {
            IRInstruction::ABC(inst) => {
                BCInstABC::emit(inst.op_code, inst.a.as_8(), inst.b.as_8(), inst.c.as_8())
            }
            IRInstruction::AD(inst) => {
                BCInstAD::emit(inst.op_code, inst.a.as_8(), inst.d.as_16())
            }
        }
    }
}

// --- Structure that holds the instruction

#[derive(Debug)]
pub struct IRInstABC {
    label: u64,
    op_code: u8,
    a: IRArg,
    b: IRArg,
    c: IRArg
}

impl IRInstABC {
    pub fn new(op_code: u8, a: IRArg, b: IRArg, c: IRArg) -> IRInstABC {
        IRInstABC {
            label: 0,
            op_code,
            a,
            b,
            c
        }
    }
}

#[derive(Debug)]
pub struct IRInstAD {
    label: u64,
    op_code: u8,
    a: IRArg,
    d: IRArg
}

impl IRInstAD {
    pub fn new(op_code: u8, a: IRArg, d: IRArg) -> IRInstAD {
        IRInstAD {
            label: 0,
            op_code,
            a,
            d
        }
    }
}


// --- Enum for the instruction args

#[derive(Debug, Copy, Clone)]
pub enum IRArg {
    None,
    Slot(u8),
    Upvalue(u8),
    Literal(u16),
    SignedLiteral(i16),
    Primitive(Primitive),
    TNewLiteral(u8, u16),
    Num(u16),
    Str(u16),
    Tab(u16),
    Func(u16),
    CData(u16),
    Jump(u64),
    JumpLiteral(u16),
}

impl IRArg {
    pub fn as_8(&self) -> u8 {
        match self {
            IRArg::None => 0,
            IRArg::Slot(slot) => *slot,
            IRArg::Upvalue(uv) => *uv,
            IRArg::Literal(lit) => {
                if *lit > 0xFF { panic!("Cannot encode the literal {} in a 8 bit operand", *lit) }
                else { *lit as u8 }
            }
            IRArg::SignedLiteral(_) => panic!("Cannot encode a signed literal in a 8 bit operand"),
            IRArg::Primitive(prim) => {
                match prim {
                    Primitive::Nil => 0,
                    Primitive::False => 1,
                    Primitive::True => 2
                }
            }
            IRArg::TNewLiteral(_, _) => panic!("Should not reach here"),
            IRArg::Num(num) => {
                if *num > 0xFF { panic!("Cannot encode num constant with index {} in a 8 bit operand", *num) }
                else { *num as u8 }
            }
            IRArg::Str(str) => {
                if *str > 0xFF { panic!("Cannot encode str constant with index {} in a 8 bit operand", *str) }
                else { *str as u8 }
            }
            IRArg::Tab(tab) => {
                if *tab > 0xFF { panic!("Cannot encode tab constant with index {} in a 8 bit operand", *tab) }
                else { *tab as u8 }
            }
            IRArg::Func(func) => {
                if *func > 0xFF { panic!("Cannot encode func constant with index {} in a 8 bit operand", *func) }
                else { *func as u8 }
            }
            IRArg::CData(cdata) => {
                if *cdata > 0xFF { panic!("Cannot encode cdata constant with index {} in a 8 bit operand", *cdata) }
                else { *cdata as u8 }
            }
            IRArg::Jump(_) => panic!("Not handled IR jump"),
            IRArg::JumpLiteral(_) => panic!("Cannot encode jump in a 8 bit operand")
        }
    }

    pub fn as_16(&self) -> u16 {
        match self {
            IRArg::None => 0,
            IRArg::Slot(slot) => *slot as u16,
            IRArg::Upvalue(uv) => *uv as u16,
            IRArg::Literal(lit) => *lit,
            IRArg::SignedLiteral(slit) => *slit as u16,
            IRArg::Primitive(prim) => {
                match prim {
                    Primitive::Nil => 0,
                    Primitive::False => 1,
                    Primitive::True => 2
                }
            }
            IRArg::TNewLiteral(hash, tab) => {
                ((*hash as u16) << 11) | (*tab & 0x7FF) as u16
            }
            IRArg::Num(num) => *num,
            IRArg::Str(str) => *str,
            IRArg::Tab(tab) => *tab,
            IRArg::Func(func) => *func,
            IRArg::CData(cdata) => *cdata,
            IRArg::Jump(_) => panic!("Not handled IR jump"),
            IRArg::JumpLiteral(offset) => *offset
        }
    }
}


// --- Enum to represents a slot

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Slot {
    Local(u8),
    Tmp(u8)
}


// --- Enum to represents a primitive

#[derive(Debug, Copy, Clone)]
pub enum Primitive {
    Nil,
    False,
    True
}


// --- Functions

/// Process the intermediary representation and return the instructions and the frame size
pub fn process_ir(ir: &mut Vec<IRInstruction>) -> Vec<BCInstruction> {
    // Process the slots and the jumps
    process_jumps(ir);

    // Translate the IR instruction to BC instructions
    let mut res = Vec::new();
    for ir_inst in ir {
        res.push(ir_inst.to_bc_instruction());
    }

    res
}

/// Function to process the jump instruction with the labelled instructions
fn process_jumps(ir: &mut Vec<IRInstruction>) {
    // Iterate over all IR instructions
    for i in 0..ir.len() {
        // Get the current instruction
        let ir_inst = ir.get(i).unwrap();

        // If the instruction contains an unresolved jump, resolve it
        match ir_inst {
            IRInstruction::AD(ad_inst) => {
                match ad_inst.d {
                    IRArg::Jump(label) => {

                        // Get the current position and the target label position
                        let current_pos = i + 1;
                        let target_pos = get_label_position(ir, label)
                            .expect("Cannot process IR : label not found");

                        // Compute the offset
                        let mut offset: isize = (target_pos as isize) - (target_pos as isize);
                        offset += (JUMP_BIASING as isize);

                        // Updating the current instruction operand
                        match ir.get_mut(i).unwrap() {
                            IRInstruction::AD(to_change) => {
                                to_change.d =
                                    IRArg::JumpLiteral(u16::try_from(offset).expect("Jump is too long and cannot be handled by LuaJIT"));
                            }
                            _ => ()
                        }

                    }
                    _ => ()
                }
            }
            _ => ()
        }
    }
}

/// Get the position of the given label in the instruction vector
fn get_label_position(ir: &Vec<IRInstruction>, label: u64) -> Option<usize> {
    for i in 0..ir.len() {
        let ir_inst = ir.get(i).unwrap();

        match ir_inst {
            IRInstruction::ABC(abc_inst) => {
                if abc_inst.label == label { return Some(i); }
            }
            IRInstruction::AD(ad_inst) => {
                if ad_inst.label == label { return Some(i); }
            }
        }
    }

    None
}