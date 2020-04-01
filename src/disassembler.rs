use super::DecompilerError;
use std::io::{Cursor, Error as IoError, Read};

impl From<IoError> for DecompilerError {
    fn from(_: IoError) -> Self {
        DecompilerError::Read
    }
}

fn read_u8<T: Read>(data: &mut T) -> Result<u8, DecompilerError> {
    let mut buf = [0_u8; 1];
    let amt = data.read(&mut buf)?;
    if amt < 1 {
        return Err(DecompilerError::EndOfCode);
    }
    Ok(buf[0])
}

fn read_u16<T: Read>(data: &mut T) -> Result<u16, DecompilerError> {
    let mut buf = [0_u8; 2];
    let amt = data.read(&mut buf)?;
    if amt < 2 {
        return Err(DecompilerError::EndOfCode);
    }
    let r: u16 = unsafe { std::mem::transmute(buf) };
    Ok(r.to_be())
}

fn read_u32<T: Read>(data: &mut T) -> Result<u32, DecompilerError> {
    let mut buf = [0_u8; 4];
    let amt = data.read(&mut buf)?;
    if amt < 4 {
        return Err(DecompilerError::EndOfCode);
    }
    let r: u32 = unsafe { std::mem::transmute(buf) };
    Ok(r.to_be())
}

#[derive(Debug, Clone)]
pub enum ArrayType {
    Boolean,
    Char,
    Float,
    Double,
    Byte,
    Short,
    Int,
    Long,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    SALoad,
    TableSwitch {
        default: u32,
        low: u32,
        high: u32,
        offsets: Vec<u32>,
    },
    Swap,
    SAStore,
    SIPush {
        value: i16,
    },
    NewArray {
        array_type: ArrayType,
    },
    Pop2,
    IConst {
        value: i32,
    },
    FConst {
        value: f32,
    },
    DConst {
        value: f64,
    },
    LConst {
        value: i64,
    },
    IAdd,
    FAdd,
    InvokeSpecial {
        index: u16,
    },
    InvokeStatic {
        index: u16,
    },
    InvokeVirtual {
        index: u16,
    },
    InvokeInterface {
        index: u16,
    },
    PutField {
        index: u16,
    },
    GetField {
        index: u16,
    },
    PutStatic {
        index: u16,
    },
    Return,
    Dup,
    DupX1,
    DupX2,
    Dup2,
    Dup2X1,
    Dup2X2,
    Pop,
    DAdd,
    DDiv,
    D2i,
    D2f,
    D2l,
    AReturn,
    CheckCast {
        index: u16,
    },
    F2i,
    AConstNull,
    BIPush {
        value: i8,
    },
    LoadConst {
        index: u16,
    },
    DCmpL,
    DCmpG,
    ArrayLength,
    AThrow,
    DALoad,
    CALoad,
    BALoad,
    AALoad,
    FALoad,
    DAStore,
    CAStore,
    BAStore,
    AAStore,
    FAStore,
    ANewArray {
        index: u16,
    },
    DMul,
    DNeg,
    DRem,
    DReturn,
    FSub,
    FMul,
    FNeg,
    FRem,
    FReturn,
    FCmpL,
    FCmpG,
    DSub,
    FDiv,
    F2l,
    F2d,
    GetStatic {
        index: u16,
    },
    I2l,
    I2d,
    I2s,
    I2c,
    I2b,
    I2f,
    IALoad,
    IAStore,
    IMul,
    IDiv,
    IAnd,
    INeg,
    InstanceOf {
        index: u16,
    },
    InvokeDynamic {
        index: u16,
    },
    L2i,
    L2d,
    L2f,
    LALoad,
    LAStore,
    LAdd,
    LAnd,
    LOr,
    LXOr,
    LSub,
    LMul,
    LDiv,
    ISub,
    IRem,
    LNeg,
    IShL,
    IShR,
    IUShR,
    IOr,
    IXOr,
    LCmp,
    IReturn,
    LReturn,
    LRem,
    LShL,
    LShR,
    LUShR,
    LookupSwitch {
        default: u32,
        pairs: Vec<(i32, u32)>,
    },
    Nop,
    MonitorEnter,
    MonitorExit,
    MultiANewArray {
        index: u16,
        dimensions: u8,
    },
    New {
        index: u16,
    },
    Ret {
        index: u16,
    },
    AStore {
        index: u16,
    },
    LStore {
        index: u16,
    },
    IStore {
        index: u16,
    },
    DStore {
        index: u16,
    },
    FStore {
        index: u16,
    },
    FLoad {
        index: u16,
    },
    ILoad {
        index: u16,
    },
    ALoad {
        index: u16,
    },
    DLoad {
        index: u16,
    },
    LLoad {
        index: u16,
    },
    IInc {
        index: u16,
        value: i16,
    },
    IfACmpEq {
        branch: u16,
    },
    IfACmpNe {
        branch: u16,
    },
    IfICmpEq {
        branch: u16,
    },
    IfICmpNe {
        branch: u16,
    },
    IfICmpLt {
        branch: u16,
    },
    IfICmpGe {
        branch: u16,
    },
    IfICmpGt {
        branch: u16,
    },
    IfICmpLe {
        branch: u16,
    },
    IfNull {
        branch: u16,
    },
    IfNonNull {
        branch: u16,
    },
    IfEq {
        branch: u16,
    },
    IfNe {
        branch: u16,
    },
    IfLt {
        branch: u16,
    },
    IfGe {
        branch: u16,
    },
    IfGt {
        branch: u16,
    },
    IfLe {
        branch: u16,
    },
    Goto {
        branch: u16,
    },
    JSr {
        branch: u16,
    },
}

fn read_instruction(
    data: &mut Cursor<Vec<u8>>,
    pos: i32,
    wide: bool,
) -> Result<Instruction, DecompilerError> {
    let code = read_u8(data)?;
    Ok(match code {
        0x0 => Instruction::Nop,
        0x1 => Instruction::AConstNull,
        0x2 => Instruction::IConst { value: -1 },
        0x3 => Instruction::IConst { value: 0 },
        0x4 => Instruction::IConst { value: 1 },
        0x5 => Instruction::IConst { value: 2 },
        0x6 => Instruction::IConst { value: 3 },
        0x7 => Instruction::IConst { value: 4 },
        0x8 => Instruction::IConst { value: 5 },
        0x9 => Instruction::LConst { value: 0 },
        0xa => Instruction::LConst { value: 1 },
        0xb => Instruction::FConst { value: 0.0 },
        0xc => Instruction::FConst { value: 1.0 },
        0xd => Instruction::FConst { value: 2.0 },
        0xe => Instruction::DConst { value: 0.0 },
        0xf => Instruction::DConst { value: 1.0 },
        0x10 => Instruction::BIPush {
            value: read_u8(data)? as i8,
        },
        0x11 => Instruction::SIPush {
            value: read_u16(data)? as i16,
        },
        0x12 => Instruction::LoadConst {
            index: read_u8(data)? as u16,
        },
        0x13 | 0x14 => Instruction::LoadConst {
            index: read_u16(data)?,
        },
        0x15 => Instruction::ILoad {
            index: if wide {
                read_u16(data)?
            } else {
                read_u8(data)? as u16
            },
        },
        0x16 => Instruction::LLoad {
            index: if wide {
                read_u16(data)?
            } else {
                read_u8(data)? as u16
            },
        },
        0x17 => Instruction::FLoad {
            index: if wide {
                read_u16(data)?
            } else {
                read_u8(data)? as u16
            },
        },
        0x18 => Instruction::DLoad {
            index: if wide {
                read_u16(data)?
            } else {
                read_u8(data)? as u16
            },
        },
        0x19 => Instruction::ALoad {
            index: if wide {
                read_u16(data)?
            } else {
                read_u8(data)? as u16
            },
        },
        0x1a => Instruction::ILoad { index: 0 },
        0x1b => Instruction::ILoad { index: 1 },
        0x1c => Instruction::ILoad { index: 2 },
        0x1d => Instruction::ILoad { index: 3 },
        0x1e => Instruction::LLoad { index: 0 },
        0x1f => Instruction::LLoad { index: 1 },
        0x20 => Instruction::LLoad { index: 2 },
        0x21 => Instruction::LLoad { index: 3 },
        0x22 => Instruction::FLoad { index: 0 },
        0x23 => Instruction::FLoad { index: 1 },
        0x24 => Instruction::FLoad { index: 2 },
        0x25 => Instruction::FLoad { index: 3 },
        0x26 => Instruction::DLoad { index: 0 },
        0x27 => Instruction::DLoad { index: 1 },
        0x28 => Instruction::DLoad { index: 2 },
        0x29 => Instruction::DLoad { index: 3 },
        0x2a => Instruction::ALoad { index: 0 },
        0x2b => Instruction::ALoad { index: 1 },
        0x2c => Instruction::ALoad { index: 2 },
        0x2d => Instruction::ALoad { index: 3 },
        0x2e => Instruction::IALoad,
        0x2f => Instruction::LALoad,
        0x30 => Instruction::FALoad,
        0x31 => Instruction::DALoad,
        0x32 => Instruction::AALoad,
        0x33 => Instruction::BALoad,
        0x34 => Instruction::CALoad,
        0x35 => Instruction::SALoad,
        0x36 => Instruction::IStore {
            index: if wide {
                read_u16(data)?
            } else {
                read_u8(data)? as u16
            },
        },
        0x37 => Instruction::LStore {
            index: if wide {
                read_u16(data)?
            } else {
                read_u8(data)? as u16
            },
        },
        0x38 => Instruction::FStore {
            index: if wide {
                read_u16(data)?
            } else {
                read_u8(data)? as u16
            },
        },
        0x39 => Instruction::DStore {
            index: if wide {
                read_u16(data)?
            } else {
                read_u8(data)? as u16
            },
        },
        0x3a => Instruction::AStore {
            index: if wide {
                read_u16(data)?
            } else {
                read_u8(data)? as u16
            },
        },
        0x3b => Instruction::IStore { index: 0 },
        0x3c => Instruction::IStore { index: 1 },
        0x3d => Instruction::IStore { index: 2 },
        0x3e => Instruction::IStore { index: 3 },
        0x3f => Instruction::LStore { index: 0 },
        0x40 => Instruction::LStore { index: 1 },
        0x41 => Instruction::LStore { index: 2 },
        0x42 => Instruction::LStore { index: 3 },
        0x43 => Instruction::FStore { index: 0 },
        0x44 => Instruction::FStore { index: 1 },
        0x45 => Instruction::FStore { index: 2 },
        0x46 => Instruction::FStore { index: 3 },
        0x47 => Instruction::DStore { index: 0 },
        0x48 => Instruction::DStore { index: 1 },
        0x49 => Instruction::DStore { index: 2 },
        0x4a => Instruction::DStore { index: 3 },
        0x4b => Instruction::AStore { index: 0 },
        0x4c => Instruction::AStore { index: 1 },
        0x4d => Instruction::AStore { index: 2 },
        0x4e => Instruction::AStore { index: 3 },
        0x4f => Instruction::IAStore,
        0x50 => Instruction::LAStore,
        0x51 => Instruction::FAStore,
        0x52 => Instruction::DAStore,
        0x53 => Instruction::AAStore,
        0x54 => Instruction::BAStore,
        0x55 => Instruction::CAStore,
        0x56 => Instruction::SAStore,
        0x57 => Instruction::Pop,
        0x58 => Instruction::Pop2,
        0x59 => Instruction::Dup,
        0x5a => Instruction::DupX1,
        0x5b => Instruction::DupX2,
        0x5c => Instruction::Dup2,
        0x5d => Instruction::Dup2X1,
        0x5e => Instruction::Dup2X2,
        0x5f => Instruction::Swap,
        0x60 => Instruction::IAdd,
        0x61 => Instruction::LAdd,
        0x62 => Instruction::FAdd,
        0x63 => Instruction::DAdd,
        0x64 => Instruction::ISub,
        0x65 => Instruction::LSub,
        0x66 => Instruction::FSub,
        0x67 => Instruction::DSub,
        0x68 => Instruction::IMul,
        0x69 => Instruction::LMul,
        0x6a => Instruction::FMul,
        0x6b => Instruction::DMul,
        0x6c => Instruction::IDiv,
        0x6d => Instruction::LDiv,
        0x6e => Instruction::FDiv,
        0x6f => Instruction::DDiv,
        0x70 => Instruction::IRem,
        0x71 => Instruction::LRem,
        0x72 => Instruction::FRem,
        0x73 => Instruction::DRem,
        0x74 => Instruction::INeg,
        0x75 => Instruction::LNeg,
        0x76 => Instruction::FNeg,
        0x77 => Instruction::DNeg,
        0x78 => Instruction::IShL,
        0x79 => Instruction::LShL,
        0x7a => Instruction::IShR,
        0x7b => Instruction::LShR,
        0x7c => Instruction::IUShR,
        0x7d => Instruction::LUShR,
        0x7e => Instruction::IAnd,
        0x7f => Instruction::LAnd,
        0x80 => Instruction::IOr,
        0x81 => Instruction::LOr,
        0x82 => Instruction::IXOr,
        0x83 => Instruction::LXOr,
        0x84 => Instruction::IInc {
            index: if wide {
                read_u16(data)?
            } else {
                read_u8(data)? as u16
            },
            value: if wide {
                read_u16(data)? as i16
            } else {
                (read_u8(data)? as i8) as i16
            },
        },
        0x85 => Instruction::I2l,
        0x86 => Instruction::I2f,
        0x87 => Instruction::I2d,
        0x88 => Instruction::L2i,
        0x89 => Instruction::L2f,
        0x8a => Instruction::L2d,
        0x8b => Instruction::F2i,
        0x8c => Instruction::F2l,
        0x8d => Instruction::F2d,
        0x8e => Instruction::D2i,
        0x8f => Instruction::D2l,
        0x90 => Instruction::D2f,
        0x91 => Instruction::I2b,
        0x92 => Instruction::I2c,
        0x93 => Instruction::I2s,
        0x94 => Instruction::LCmp,
        0x95 => Instruction::FCmpL,
        0x96 => Instruction::FCmpG,
        0x97 => Instruction::DCmpL,
        0x98 => Instruction::DCmpG,
        0x99 => Instruction::IfEq {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0x9a => Instruction::IfNe {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0x9b => Instruction::IfLt {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0x9c => Instruction::IfGe {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0x9d => Instruction::IfGt {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0x9e => Instruction::IfLe {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0x9f => Instruction::IfICmpEq {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0xa0 => Instruction::IfICmpNe {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0xa1 => Instruction::IfICmpLt {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0xa2 => Instruction::IfICmpGe {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0xa3 => Instruction::IfICmpGt {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0xa4 => Instruction::IfICmpLe {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0xa5 => Instruction::IfACmpEq {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0xa6 => Instruction::IfACmpNe {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0xa7 => Instruction::Goto {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0xa8 => Instruction::JSr {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0xa9 => Instruction::Ret {
            index: if wide {
                read_u16(data)?
            } else {
                read_u8(data)? as u16
            },
        },
        0xaa => {
            let pad = (1 + ((data.position() - 1) / 4)) * 4 - data.position();
            for _ in 0..pad {
                read_u8(data)?;
            }
            let default = (pos + (read_u32(data)? as i32)) as u32;
            let low = read_u32(data)?;
            let high = read_u32(data)?;

            let mut offsets = Vec::new();
            for _ in low..=high {
                offsets.push((pos + (read_u32(data)? as i32)) as u32);
            }
            Instruction::TableSwitch {
                default,
                low,
                high,
                offsets,
            }
        }
        0xab => {
            let pad = (1 + ((data.position() - 1) / 4)) * 4 - data.position();
            for _ in 0..pad {
                read_u8(data)?;
            }
            let default = (pos + (read_u32(data)? as i32)) as u32;
            let count = read_u32(data)?;

            let mut pairs = Vec::new();
            for _ in 0..count {
                pairs.push((
                    read_u32(data)? as i32,
                    (pos + (read_u32(data)? as i32)) as u32,
                ));
            }
            Instruction::LookupSwitch { default, pairs }
        }
        0xac => Instruction::IReturn,
        0xad => Instruction::LReturn,
        0xae => Instruction::FReturn,
        0xaf => Instruction::DReturn,
        0xb0 => Instruction::AReturn,
        0xb1 => Instruction::Return,
        0xb2 => Instruction::GetStatic {
            index: read_u16(data)?,
        },
        0xb3 => Instruction::PutStatic {
            index: read_u16(data)?,
        },
        0xb4 => Instruction::GetField {
            index: read_u16(data)?,
        },
        0xb5 => Instruction::PutField {
            index: read_u16(data)?,
        },
        0xb6 => Instruction::InvokeVirtual {
            index: read_u16(data)?,
        },
        0xb7 => Instruction::InvokeSpecial {
            index: read_u16(data)?,
        },
        0xb8 => Instruction::InvokeStatic {
            index: read_u16(data)?,
        },
        0xb9 => {
            let index = read_u16(data)?;
            read_u16(data)?;
            Instruction::InvokeInterface { index }
        }
        0xba => {
            let index = read_u16(data)?;
            read_u16(data)?;
            Instruction::InvokeDynamic { index }
        }
        0xbb => Instruction::New {
            index: read_u16(data)?,
        },
        0xbc => {
            let type_id = read_u8(data)?;
            let array_type = match type_id {
                4 => ArrayType::Boolean,
                5 => ArrayType::Char,
                6 => ArrayType::Float,
                7 => ArrayType::Double,
                8 => ArrayType::Byte,
                9 => ArrayType::Short,
                10 => ArrayType::Int,
                11 => ArrayType::Long,
                _ => return Err(DecompilerError::UnknownArrayType { type_id }),
            };
            Instruction::NewArray { array_type }
        }
        0xbd => Instruction::ANewArray {
            index: read_u16(data)?,
        },
        0xbe => Instruction::ArrayLength,
        0xbf => Instruction::AThrow,
        0xc0 => Instruction::CheckCast {
            index: read_u16(data)?,
        },
        0xc1 => Instruction::InstanceOf {
            index: read_u16(data)?,
        },
        0xc2 => Instruction::MonitorEnter,
        0xc3 => Instruction::MonitorExit,
        0xc4 => read_instruction(data, pos, true)?,
        0xc5 => Instruction::MultiANewArray {
            index: read_u16(data)?,
            dimensions: read_u8(data)?,
        },
        0xc6 => Instruction::IfNull {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0xc7 => Instruction::IfNonNull {
            branch: (pos + ((read_u16(data)? as i16) as i32)) as u16,
        },
        0xc8 => Instruction::Goto {
            branch: (pos + (read_u32(data)? as i32)) as u16,
        },
        0xc9 => Instruction::JSr {
            branch: (pos + (read_u32(data)? as i32)) as u16,
        },
        _ => return Err(DecompilerError::UnknownInstr { instruction: code }),
    })
}

pub fn disassemble(codes_vec: Vec<u8>) -> Result<Vec<(u64, Instruction)>, DecompilerError> {
    let length = codes_vec.len() as u64;
    let mut codes = Cursor::new(codes_vec);

    let mut instructions = Vec::new();
    loop {
        let pos = codes.position();
        let instr = read_instruction(&mut codes, pos as i32, false)?;
        instructions.push((pos, instr));
        if codes.position() == length {
            break;
        }
    }
    Ok(instructions)
}
