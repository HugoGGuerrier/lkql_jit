/*
This module contains all elements to produce LuaJIT bytecode

A LuaJIT bytecode is formed of
- A header containing information about the Lua context
- Multiple function prototypes to define execution units

LuaJIT bytecode always have a header field
Its composition is :
[
    MAGIC (3 bytes) |
    VERSION (1 byte) |
    FLAGS (1 byte or 1 uleb128)
]

A LuaJIT prototype is composed as :
[
    SIZE (1 uleb128) |
    FLAGS (1 byte) |
    ARG_COUNT (1 byte) |
    FRAME_SIZE (1 byte) |
    UPVAL_COUNT (1 byte) |
    COMPLEX_CONST_COUNT (1 uleb128) |
    NUM_CONST_COUNT (1 uleb128) |
    INST_COUNT (1 uleb128) |
    DEBUG_INFO_SIZE (1 uleb128 if FLAG_H_IS_STRIPPED else absent) |
    FIRST_LINE_NB (1 uleb128 if FLAG_H_IS_STRIPPED else absent) |
    LINE_COUNT (1 uleb128 if FLAG_H_IS_STRIPPED else absent) |
    INSTRUCTIONS (4 bytes[]) |
    CONSTANT_TABLE
]

The LuaJIT constant table is a bytecode section at the end of every prototype that contains
constants of the functions (Ex: all used strings, all used integers)
Its structure is :
[
    UPVALUE_CONST (each on 2 bytes as unsigned int that represent the index of the UV, if the highest bit is 1, the UV is outer local) |
    COMPLEX_CONST (can be STR, TAB, CHILD, COMPLEX, NUMBER) |
    NUMERIC_CONST (each is encoded as lo|hi with lo = uleb128 from 33 bits and hi = uleb128)
]

A constant in LuaJIT bytecode is composed of two elements :
[ CONSTANT_KIND (uleb128) (IF STR THIS IS ALSO THE SIZE OF THE STRING + 5) | CONSTANT_DATA (depending of the constant kind) ]
Table constants ar structure as prototype constants

About numeric constants !
Lua integers ars 64 bit signed integers
Every numeric constant follow the uleb128 on 33 bits rule on its first part to determine
if the constant is an integer or a number
Example : 0x00000011 IS A NUMBER WITH HI PART, LO IS 1
          0x00000010 IS AN INTEGER EQUALS TO 1, NO HI PART TO READ

About table constant !
A table constant is represented as this in the constant pool :
[
    ARRAY_ITEM_COUNT (uleb128) |
    HASH_ITEM_COUNT (uleb128) |
    ARRAY (TABLE_ITEM[]) |
    MAP ((TABLE_ITEM, TABLE_ITEM)[])
]
*/

use std::collections::HashMap;
use std::fmt;
use nano_leb128::ULEB128;


// --- Defining the header macros

pub const MAGIC: [u8; 3] = [0x1B, 0x4C, 0x4A];
pub const CUR_VERSION: u8 = 0x02;
pub const MAX_VERSION: u8 = 0x80;

pub const FLAG_H_IS_BIG_ENDIAN: u8 = 0b00000001; // If the bytecode is in big endian
pub const FLAG_H_IS_STRIPPED: u8 = 0b00000010; // If the bytecode is stripped (without debug info)
pub const FLAG_H_HAS_FFI: u8 = 0b00000100; // If the bytecode hase FFI access
pub const FLAG_H_FR2: u8 = 0b00001000; // TODO


// --- Defining the prototype macros

pub const FLAG_P_HAS_CHILD: u8 = 0b00000001;
pub const FLAG_P_IS_VARIADIC: u8 = 0b00000010;
pub const FLAG_P_HAS_FFI: u8 = 0b00000100;
pub const FLAG_P_JIT_DISABLED: u8 = 0b00001000;
pub const FLAG_P_HAS_ILOOP: u8 = 0b00010000;

pub const JUMP_BIASING: u16 = 0x8000;


// --- Defining the operation codes

// -- Comparison ops
pub const ISLT: u8 = 0x00;
pub const ISGE: u8 = 0x01;
pub const ISLE: u8 = 0x02;
pub const ISGT: u8 = 0x03;

pub const ISEQV: u8 = 0x04;
pub const ISNEV: u8 = 0x05;

pub const ISEQS: u8 = 0x06;
pub const ISNES: u8 = 0x07;

pub const ISEQN: u8 = 0x08;
pub const ISNEN: u8 = 0x09;

pub const ISEQP: u8 = 0x0A;
pub const ISNEP: u8 = 0x0B;

// -- Unary test and copy ops
pub const ISTC: u8 = 0x0C;
pub const ISFC: u8 = 0x0D;

pub const IST: u8 = 0x0E;
pub const ISF: u8 = 0x0F;

pub const ISTYPE: u8 = 0x10;
pub const ISNUM: u8 = 0x11;

// -- Unary ops
pub const MOV: u8 = 0x12;
pub const NOT: u8 = 0x13;
pub const UNM: u8 = 0x14;
pub const LEN: u8 = 0x15;

// -- Binary ops
pub const ADDVN: u8 = 0x16;
pub const SUBVN: u8 = 0x17;
pub const MULVN: u8 = 0x18;
pub const DIVVN: u8 = 0x19;
pub const MODVN: u8 = 0x1A;

pub const ADDNV: u8 = 0x1B;
pub const SUBNV: u8 = 0x1C;
pub const MULNV: u8 = 0x1D;
pub const DIVNV: u8 = 0x1E;
pub const MODNV: u8 = 0x1F;

pub const ADDVV: u8 = 0x20;
pub const SUBVV: u8 = 0x21;
pub const MULVV: u8 = 0x22;
pub const DIVVV: u8 = 0x23;
pub const MODVV: u8 = 0x24;

pub const POW: u8 = 0x25;
pub const CAT: u8 = 0x26;

// -- Constant ops
pub const KSTR: u8 = 0x27;
pub const KCDATA: u8 = 0x28;
pub const KSHORT: u8 = 0x29;
pub const KNUM: u8 = 0x2A;
pub const KPRI: u8 = 0x2B;

pub const KNIL: u8 = 0x2C;

// -- Upvalue and function ops
pub const UGET: u8 = 0x2D;

pub const USETV: u8 = 0x2E;
pub const USETS: u8 = 0x2F;
pub const USETN: u8 = 0x30;
pub const USETP: u8 = 0x31;

pub const UCLO: u8 = 0x32;

pub const FNEW: u8 = 0x33;

// -- Table ops
pub const TNEW: u8 = 0x34;

pub const TDUP: u8 = 0x35;

pub const GGET: u8 = 0x36;
pub const GSET: u8 = 0x37;

pub const TGETV: u8 = 0x38;
pub const TGETS: u8 = 0x39;
pub const TGETB: u8 = 0x3A;
pub const TGETR: u8 = 0x3B;

pub const TSETV: u8 = 0x3C;
pub const TSETS: u8 = 0x3D;
pub const TSETB: u8 = 0x3E;
pub const TSETM: u8 = 0x3F;
pub const TSETR: u8 = 0x40;

// -- Calls and vararg handling
pub const CALLM: u8 = 0x41;
pub const CALL: u8 = 0x42;
pub const CALLMT: u8 = 0x43;
pub const CALLT: u8 = 0x44;

pub const ITERC: u8 = 0x45;
pub const ITERN: u8 = 0x46;

pub const VARG: u8 = 0x47;

pub const ISNEXT: u8 = 0x48;

// -- Returns
pub const RETM: u8 = 0x49;
pub const RET: u8 = 0x4A;
pub const RET0: u8 = 0x4B;
pub const RET1: u8 = 0x4C;

// -- Loops and branches
pub const FORI: u8 = 0x4D;
pub const JFORI: u8 = 0x4E;

pub const FORL: u8 = 0x4F;
pub const IFORL: u8 = 0x50;
pub const JFORL: u8 = 0x51;

pub const ITERL: u8 = 0x52;
pub const IITERL: u8 = 0x53;
pub const JITERL: u8 = 0x54;

pub const LOOP: u8 = 0x55;
pub const ILOOP: u8 = 0x56;
pub const JLOOP: u8 = 0x57;

pub const JMP: u8 = 0x58;

// -- Function headers
pub const FUNCF: u8 = 0x59;
pub const IFUNCF: u8 = 0x5A;
pub const JFUNCF: u8 = 0x5B;

pub const FUNCV: u8 = 0x5C;
pub const IFUNCV: u8 = 0x5D;
pub const JFUNCV: u8 = 0x5E;

pub const FUNCC: u8 = 0x5F;
pub const FUNCCW: u8 = 0x60;


// --- Defining the constant table macros

pub const BCDUMP_KGC_CHILD: u32 = 0;
pub const BCDUMP_KGC_TAB: u32 = 1;
pub const BCDUMP_KGC_I64: u32 = 2;
pub const BCDUMP_KGC_U64: u32 = 3;
pub const BCDUMP_KGC_COMPLEX: u32 = 4;
pub const BCDUMP_KGC_STR: u32 = 5;

pub const BCDUMP_KTAB_NIL: u32 = 0;
pub const BCDUMP_KTAB_FALSE: u32 = 1;
pub const BCDUMP_KTAB_TRUE: u32 = 2;
pub const BCDUMP_KTAB_INT: u32 = 3;
pub const BCDUMP_KTAB_NUM: u32 = 4;
pub const BCDUMP_KTAB_STR: u32 = 5;


// --- Defining the bytecode fundamentals structures

// Structure of a bytecode program
#[derive(Debug)]
pub struct Program {
    pub header: Header,
    pub prototypes: Vec<Prototype>,
}

impl Program {
    /// Create a new bytecode program
    pub fn new() -> Program {
        Program {
            header: Header::new(),
            prototypes: Vec::new()
        }
    }

    /// Encode the program into real bytecode
    pub fn encode(&self) -> Vec<u8> {
        // Create the result
        let mut res = Vec::new();

        // Add the encoded header at the top of the bytecode
        let mut header_bc = self.header.encode();
        res.append(&mut header_bc);

        // Add the prototype to the bytecode
        for proto in &self.prototypes {
            let mut proto_bc = proto.encode();
            res.append(&mut proto_bc);
        }

        // The tail
        res.push(0);

        // Return the result
        res
    }
}

// Structure for the bytecode file header
#[derive(Debug)]
pub struct Header {
    pub magic: [u8; 3],
    pub version: u8,
    pub flags: u8,
}

impl Header {
    /// Create a new header for the bytecode
    pub fn new() -> Header {
        Header {
            magic: MAGIC,
            version: CUR_VERSION,
            flags: 0x0 | FLAG_H_IS_STRIPPED | FLAG_H_HAS_FFI
        }
    }

    /// Encode the header and return the real bytecode
    pub fn encode(&self) -> Vec<u8> {
        // Create the result from the magic
        let mut res = Vec::from(self.magic);

        // Add the version and the flags
        res.push(self.version);
        res.push(self.flags);

        // Return the result
        res
    }
}

// Structure for a function prototype
#[derive(Debug)]
pub struct Prototype {
    pub flags: u8,
    pub arg_count: u8,
    pub frame_size: u8,
    pub instructions: Vec<BCInstruction>,
    pub upval_references: Vec<u16>,
    pub complex_constants: Vec<ComplexConstant>,
    pub numeric_constants: Vec<NumericConstant>,
}

impl Prototype {
    /// Create a new prototype with the default value
    pub fn new(arg_count: u8) -> Prototype {
        Prototype {
            flags: 0,
            arg_count,
            frame_size: 0,
            instructions: Vec::new(),
            upval_references: Vec::new(),
            complex_constants: Vec::new(),
            numeric_constants: Vec::new()
        }
    }

    /// Encode the prototype and return the real bytecode
    pub fn encode(&self) -> Vec<u8> {
        // Create the result vector
        let mut res = Vec::new();

        // Prepare the working vars
        let mut uleb: ULEB128;

        // Put the flags, arg count and frame size
        res.push(self.flags);
        res.push(self.arg_count);
        res.push(self.frame_size);

        // Put the upval count
        res.push(self.upval_references.len() as u8);

        // Put the complex constant count
        uleb = ULEB128::from(self.complex_constants.len() as u64);
        encode_uleb128(&uleb, &mut res);

        // Put the numeric constant count
        uleb = ULEB128::from(self.numeric_constants.len() as u64);
        encode_uleb128(&uleb, &mut res);

        // Put the instruction count
        uleb = ULEB128::from(self.instructions.len() as u64);
        encode_uleb128(&uleb, &mut res);

        // Put the instructions in the result
        for inst in &self.instructions {
            let mut inst_bc = inst.encode();
            res.append(&mut inst_bc);
        }

        // Put the constant table in the result

        // The upvalue constants
        for upval in &self.upval_references {
            res.push(((upval >> 8) & 0xFF) as u8);
            res.push((upval & 0xFF) as u8);
        }

        // The complex constants
        for complex in &self.complex_constants {
            let mut complex_bc = complex.encode();
            res.append(&mut complex_bc);
        }

        // The numeric constants
        for numeric in &self.numeric_constants {
            let mut numeric_bc = numeric.encode();
            res.append(&mut numeric_bc);
        }

        // Add the size at the very start of the bytecode
        let mut buff = [0u8; 11];
        let buff_len = ULEB128::from(res.len() as u64).write_into(&mut buff).unwrap();
        let mut i = buff_len - 1;
        loop {
            res.insert(0, buff[i]);
            if i == 0 { break; }
            else { i -= 1 }
        }

        // Return the result
        res
    }
}

// The instruction enum, to unify instruction types
#[derive(Debug)]
pub enum BCInstruction {
    Abc(BCInstABC),
    Ad(BCInstAD)
}

impl BCInstruction {
    /// Encode a function into the bytecode
    pub fn encode(&self) -> Vec<u8> {
        match self {
            BCInstruction::Abc(abc) => abc.encode(),
            BCInstruction::Ad(ad) => ad.encode()
        }
    }
}

// Structure for a OP A B C instruction
#[derive(Debug)]
pub struct BCInstABC {
    pub op_code: u8,
    pub a: u8,
    pub b: u8,
    pub c: u8,
}

impl BCInstABC {
    /// Create a new instruction ABC type with the parameters
    pub fn new(op_code: u8, a: u8, b: u8, c: u8) -> BCInstABC {
        BCInstABC {
            op_code,
            a,
            b,
            c
        }
    }

    /// Create a new instruction ABC type wrapped in the instruction enum
    pub fn emit(op_code: u8, a: u8, b: u8, c: u8) -> BCInstruction {
        BCInstruction::Abc(BCInstABC::new(op_code, a, b, c))
    }

    /// Encode the instruction as bytecode
    pub fn encode(&self) -> Vec<u8> {
        // Create the result
        let mut res = Vec::with_capacity(4);
        let mut inst_int: u32 = 0;

        // Put the instruction construction in the integer
        inst_int |= (self.b as u32) << 24;
        inst_int |= (self.c as u32) << 16;
        inst_int |= (self.a as u32) << 8;
        inst_int |= (self.op_code as u32);

        // Put the instruction integer in the result
        let inst_bytes = inst_int.to_ne_bytes();
        for inst_byte in inst_bytes {
            res.push(inst_byte);
        }

        // Return the bytecode
        res
    }
}

// Structure for a OP A D instruction
#[derive(Debug)]
pub struct BCInstAD {
    pub op_code: u8,
    pub a: u8,
    pub d: u16,
}

impl BCInstAD {
    /// Create a new instruction AD type with the parameters
    pub fn new(op_code: u8, a: u8, d: u16) -> BCInstAD {
        BCInstAD {
            op_code,
            a,
            d
        }
    }

    /// Create an AD instruction wrapped in the instruction enum
    pub fn emit(op_code: u8, a: u8, d: u16) -> BCInstruction {
        BCInstruction::Ad(BCInstAD::new(op_code, a, d))
    }

    /// Encode the instruction as bytecode
    pub fn encode(&self) -> Vec<u8> {
        // Create the result
        let mut res = Vec::with_capacity(4);
        let mut inst_int: u32 = 0;

        // Put the instruction construction in the instruction integer
        inst_int |= (self.d as u32) << 16;
        inst_int |= (self.a as u32) << 8;
        inst_int |= (self.op_code as u32);

        // Put the instruction in the result
        let inst_bytes = inst_int.to_ne_bytes();
        for inst_byte in inst_bytes {
            res.push(inst_byte);
        }

        // Return the bytecode
        res
    }
}

// The enum for the complex constants
#[derive(Debug)]
pub enum ComplexConstant {
    String(KStr),
    Table(KTable),
    Complex(KComplex),
    I64(i64),
    U64(u64),
    Child
}

impl ComplexConstant {
    /// Encode the complex constant into bytecode
    pub fn encode(&self) -> Vec<u8> {
        // Prepare the result
        let mut res = Vec::new();

        match self {
            // If string constant
            ComplexConstant::String(kstr) => {
                let uleb = ULEB128::from((kstr.content.len() + (BCDUMP_KGC_STR as usize)) as u64);
                encode_uleb128(&uleb, &mut res);

                let mut str_bc = kstr.encode();
                res.append(&mut str_bc);
            },

            // If table constant
            ComplexConstant::Table(ktable) => {
                res.push(BCDUMP_KGC_TAB as u8);

                let mut table_bc = ktable.encode();
                res.append(&mut table_bc);
            },

            // If complex number
            ComplexConstant::Complex(kcomplex) => {
                res.push(BCDUMP_KGC_COMPLEX as u8);

                let mut complex_bc = kcomplex.encode();
                res.append(&mut complex_bc);
            },

            // If signed int constant
            ComplexConstant::I64(int) => {
                res.push(BCDUMP_KGC_I64 as u8);

                // PLACEHOLDER | TODO : WHAT IS I64 AND HOW TO USE IT
                res.push(0u8);
            }

            // If unsigned int constant
            ComplexConstant::U64(int) => {
                res.push(BCDUMP_KGC_U64 as u8);

                // PLACEHOLDER | TODO : WHAT IS U64 AND HOW TO USE IT
                res.push(0u8);
            }

            // If child
            ComplexConstant::Child => {
                res.push(BCDUMP_KGC_CHILD as u8)
            }
        };

        // Return the bytecode
        res
    }
}

// The structure for the string constants
#[derive(Debug)]
pub struct KStr {
    pub content: Vec<u8>,
}

impl KStr {
    /// Create a new string constant for the LuaJIT bytecode
    pub fn new(value: String) -> KStr {
        KStr {
            content: Vec::from(value)
        }
    }

    /// Decode the string constant into a real string
    pub fn decode(&self) -> String {
        String::from_utf8(self.content.clone()).unwrap_or(String::from("INVALID UTF_8 STRING"))
    }

    /// Encode the string constant into LuaJIT bytecode
    pub fn encode(&self) -> Vec<u8> {
        self.content.clone()
    }
}

// The structure for the table constants
#[derive(Debug)]
pub struct KTable {
    pub array: Vec<TableItem>,
    pub map: HashMap<TableItem, TableItem>,
}

impl KTable {
    /// Create a new constant table
    pub fn new() -> KTable {
        KTable {
            array: Vec::new(),
            map: HashMap::new()
        }
    }

    /// Encode the table into bytecode
    pub fn encode(&self) -> Vec<u8> {
        // Create the result
        let mut res = Vec::new();

        // Put the counts into the result
        let array_count = ULEB128::from(self.array.len() as u64);
        let map_count = ULEB128::from(self.map.len() as u64);
        encode_uleb128(&array_count, &mut res);
        encode_uleb128(&map_count, &mut res);

        // Put the array elements
        for elem in &self.array {
            let mut elem_bc = elem.encode();
            res.append(&mut elem_bc);
        }

        // Put the map elements
        for (key, val) in &self.map {
            let mut key_bc = key.encode();
            let mut val_bc = val.encode();
            res.append(&mut key_bc);
            res.append(&mut val_bc);
        }

        // Return the bytecode
        res
    }
}

// The enum for the table item types
#[derive(Debug)]
pub enum TableItem {
    String(KStr),
    Int(i32),
    Num(KNum),
    True,
    False,
    Nil
}

impl TableItem {
    /// Encode the table item into bytecode
    pub fn encode(&self) -> Vec<u8> {
        // Prepare the result
        let mut res = Vec::new();

        match self {
            TableItem::String(kstr) => {
                let uleb = ULEB128::from((kstr.content.len() + (BCDUMP_KTAB_STR as usize)) as u64);
                encode_uleb128(&uleb, &mut res);

                let mut str_bc = kstr.encode();
                res.append(&mut str_bc);
            },
            TableItem::Int(int) => {
                res.push(BCDUMP_KTAB_INT as u8);

                let uleb = ULEB128::from(*int as u64);
                encode_uleb128(&uleb, &mut res);
            },
            TableItem::Num(knum) => {
                res.push(BCDUMP_KTAB_NUM as u8);

                let mut num_bc = knum.encode();
                res.append(&mut num_bc);
            },
            TableItem::True => {
                res.push(BCDUMP_KTAB_TRUE as u8);
            },
            TableItem::False => {
                res.push(BCDUMP_KTAB_FALSE as u8);
            },
            TableItem::Nil => {
                res.push(BCDUMP_KTAB_NIL as u8);
            }
        }

        // Return the bytecode
        res
    }
}

// The structure for the numeric constants
#[derive(Debug)]
pub struct KNum {
    pub value: f64
}

impl KNum {
    /// Create a new numeric constant from its value
    pub fn new(value: f64) -> KNum {
        KNum {
            value
        }
    }

    /// Encode the numeric constant to LuaJIT bytecode
    pub fn encode(&self) -> Vec<u8> {
        // Get the hi and lo parts
        let hi: u64 = (self.value.to_bits() >> 32) & 0xFFFFFFFF;
        let lo: u64 = self.value.to_bits() & 0xFFFFFFFF;

        // Cast it into uleb
        let hi_uleb = ULEB128::from(hi);
        let lo_uleb = ULEB128::from(lo);

        // Prepare the result and add values
        let mut res = Vec::new();
        encode_uleb128(&lo_uleb, &mut res);
        encode_uleb128(&hi_uleb, &mut res);
        res
    }

    /// Encode the numeric constant for the numeric array
    pub fn encode_33bits(&self) -> Vec<u8> {
        // Get the hi and lo parts
        let hi: u64 = (self.value.to_bits() >> 32) & 0xFFFFFFFF;
        let lo: u64 = self.value.to_bits() & 0xFFFFFFFF;

        // Cast it into uleb
        let hi_uleb = ULEB128::from(hi);
        let lo_uleb = ULEB128::from((lo << 1) | 0x1);

        // Prepare the result and add values
        let mut res = Vec::new();
        encode_uleb128(&lo_uleb, &mut res);
        encode_uleb128(&hi_uleb, &mut res);
        res
    }
}

// The structure for the complex number constants --  NOT USED IN LKQL
#[derive(Debug)]
pub struct KComplex {
    pub number: ULEB128,
    pub imaginary: ULEB128,
}

impl KComplex {
    /// TODO : Encode the complex for the LuaJIT Bytecode
    pub fn encode(&self) -> Vec<u8> {
        Vec::new()
    }
}

// The enum for the numeric constants
#[derive(Debug)]
pub enum NumericConstant {
    Int(i32),
    Num(KNum)
}

impl NumericConstant {
    /// Encode the numeric constant as LuaJIT bytecode
    pub fn encode(&self) -> Vec<u8> {
        match self {
            NumericConstant::Int(int) => {
                let mut res = Vec::new();
                let uleb = ULEB128::from((*int as u64) << 1);
                encode_uleb128(&uleb, &mut res);
                res
            }
            NumericConstant::Num(knum) => {
                knum.encode_33bits()
            }
        }
    }
}


// --- Utils functions

/// Write the given ULEB128 into the given vector
fn encode_uleb128(uleb: &ULEB128, vec: &mut Vec<u8>) {
    let mut buff = [0u8; 32];
    let buff_len = uleb.write_into(&mut buff).unwrap();
    for i in 0..buff_len {
        vec.push(buff[i]);
    }
}