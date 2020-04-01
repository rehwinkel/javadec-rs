mod disassembler;

use disassembler::Instruction;
use javaclass::{AttributeInfo, ClassFile, ClassFileError, ConstantPool, ConstantPoolInfo};
use javaclass::{ConstClassData, ConstFieldData, ConstMethodData};
use std::collections::HashMap;
use std::convert::From;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

mod descriptors {
    use std::error::Error;
    use std::fmt::{Display, Formatter, Result as FmtResult};
    use std::iter::Peekable;

    #[derive(Debug)]
    pub enum DescriptorParseError {
        EOF,
        Expect { expected: String, got: char },
    }

    impl Error for DescriptorParseError {}

    impl Display for DescriptorParseError {
        fn fmt(&self, f: &mut Formatter) -> FmtResult {
            write!(
                f,
                "{}",
                match self {
                    DescriptorParseError::EOF => format!("end of iterator"),
                    DescriptorParseError::Expect { expected, got } =>
                        format!("unexpected char: expected '{}' got '{}'", expected, got),
                }
            )
        }
    }

    fn expect<T: Iterator<Item = char>>(
        iter: &mut Peekable<T>,
        expected: char,
    ) -> Result<(), DescriptorParseError> {
        let ch: char = peek(iter)?;
        if ch != expected {
            Err(DescriptorParseError::Expect {
                expected: expected.to_string(),
                got: ch,
            })
        } else {
            Ok(())
        }
    }

    fn accept<T: Iterator<Item = char>>(
        iter: &mut Peekable<T>,
        expected: char,
    ) -> Result<bool, DescriptorParseError> {
        let next_ch: char = peek(iter)?;
        Ok(if next_ch != expected { false } else { true })
    }

    fn peek<T: Iterator<Item = char>>(
        iter: &mut Peekable<T>,
    ) -> Result<char, DescriptorParseError> {
        Ok(*iter.peek().ok_or(DescriptorParseError::EOF)?)
    }

    fn consume<T: Iterator<Item = char>>(
        iter: &mut Peekable<T>,
    ) -> Result<(), DescriptorParseError> {
        iter.next().ok_or(DescriptorParseError::EOF)?;
        Ok(())
    }

    #[derive(Debug, PartialEq)]
    pub enum FieldType {
        Void,
        Byte,
        Char,
        Double,
        Float,
        Int,
        Long,
        Short,
        Boolean,
        Reference { name: String },
        Array { inner: Box<FieldType> },
    }

    fn parse_field_type<T: Iterator<Item = char>>(
        iter: &mut Peekable<T>,
    ) -> Result<FieldType, DescriptorParseError> {
        let ch = peek(iter)?;
        let field_type = match ch {
            'B' => FieldType::Byte,
            'C' => FieldType::Char,
            'S' => FieldType::Short,
            'Z' => FieldType::Boolean,
            'J' => FieldType::Long,
            'I' => FieldType::Int,
            'D' => FieldType::Double,
            'F' => FieldType::Float,
            'L' => {
                let mut name = String::new();
                consume(iter)?;
                while peek(iter)? != ';' {
                    name.push(peek(iter)?);
                    consume(iter)?;
                }
                FieldType::Reference { name }
            }
            '[' => FieldType::Array {
                inner: Box::new(parse_field_type(iter)?),
            },
            _ => {
                return Err(DescriptorParseError::Expect {
                    expected: String::from("field type"),
                    got: ch,
                })
            }
        };
        consume(iter)?;
        Ok(field_type)
    }

    fn parse_return_desc<T: Iterator<Item = char>>(
        iter: &mut Peekable<T>,
    ) -> Result<FieldType, DescriptorParseError> {
        if accept(iter, 'V')? {
            consume(iter)?;
            Ok(FieldType::Void)
        } else {
            parse_field_type(iter)
        }
    }

    pub fn parse_method<T: IntoIterator<Item = char>>(
        into: T,
    ) -> Result<(Vec<FieldType>, FieldType), DescriptorParseError> {
        let mut iter = into.into_iter().peekable();
        expect(&mut iter, '(')?;
        consume(&mut iter)?;

        let mut params = Vec::new();
        while peek(&mut iter)? != ')' {
            params.push(parse_field_type(&mut iter)?);
        }
        expect(&mut iter, ')')?;
        consume(&mut iter)?;
        let return_desc = parse_return_desc(&mut iter)?;
        Ok((params, return_desc))
    }
}

#[derive(Debug)]
pub enum DecompilerError {
    UnknownInstr {
        instruction: u8,
    },
    EndOfCode,
    Read,
    UnknownArrayType {
        type_id: u8,
    },
    StackSize {
        size: usize,
    },
    ClassFileError {
        error: ClassFileError,
    },
    DescriptorParsing {
        error: descriptors::DescriptorParseError,
    },
    EmptyStack,
}

impl Error for DecompilerError {}

impl Display for DecompilerError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "{}",
            match self {
                DecompilerError::Read => String::from("error reading input"),
                DecompilerError::EndOfCode => String::from("unexpected end of code"),
                DecompilerError::UnknownInstr { instruction } =>
                    format!("unknown instruction: 0x{:x}", instruction),
                DecompilerError::UnknownArrayType { type_id } =>
                    format!("unknown array type: {}", type_id),
                DecompilerError::ClassFileError { error } => format!("{}", error),
                DecompilerError::StackSize { size } => format!(
                    "unexpected amount of elements on stack after decompiling block: {}",
                    size
                ),
                DecompilerError::EmptyStack => format!("expected element but stack was empty"),
                DecompilerError::DescriptorParsing { error } => format!("{}", error),
            }
        )
    }
}

impl From<ClassFileError> for DecompilerError {
    fn from(err: ClassFileError) -> Self {
        DecompilerError::ClassFileError { error: err }
    }
}

impl From<descriptors::DescriptorParseError> for DecompilerError {
    fn from(err: descriptors::DescriptorParseError) -> Self {
        DecompilerError::DescriptorParsing { error: err }
    }
}

fn get_index_for_pos(instructions: &Vec<(u64, Instruction)>, pos: u16) -> Option<usize> {
    for (i, (i_pos, _)) in instructions.iter().enumerate() {
        if i_pos == &(pos as u64) {
            return Some(i);
        }
    }
    None
}

#[derive(Debug, Clone)]
struct Block {
    instructions: Vec<(u64, Instruction)>,
    branches: Vec<u64>,
}

fn split_at_multiple<T: Clone>(vec: Vec<T>, split_indices: Vec<usize>) -> Vec<Vec<T>> {
    let mut split_indices = split_indices;
    split_indices.sort();
    split_indices.dedup();
    let mut output = Vec::with_capacity(split_indices.len() + 1);

    if split_indices.len() == 0 {
        return vec![vec];
    } else {
        if split_indices[0] == 0 {
            split_indices.remove(0);
        }
        if *split_indices.last().unwrap() == vec.len() {
            split_indices.remove(vec.len() - 1);
        }
        let mut split_vector = vec;
        for i in 0..split_indices.len() {
            let index = split_indices[i] - if i == 0 { 0 } else { split_indices[i - 1] };
            let (first, second) = split_vector.split_at(index);

            output.push(first.to_vec());
            if i + 1 == split_indices.len() {
                output.push(second.to_vec());
            }
            split_vector = second.to_vec();
        }
    }
    output
}

fn gen_control_flow_graph(instructions: &Vec<(u64, Instruction)>) -> HashMap<u64, Block> {
    //get jump indices
    let mut jump_indices = Vec::new();
    for (i, (_, instr)) in instructions.iter().enumerate() {
        match &instr {
            Instruction::IfNe { branch }
            | Instruction::IfEq { branch }
            | Instruction::IfLe { branch }
            | Instruction::IfGe { branch }
            | Instruction::IfGt { branch }
            | Instruction::IfLt { branch }
            | Instruction::IfICmpEq { branch }
            | Instruction::IfICmpNe { branch }
            | Instruction::IfICmpGt { branch }
            | Instruction::IfICmpGe { branch }
            | Instruction::IfICmpLt { branch }
            | Instruction::IfICmpLe { branch } => {
                let true_pos = get_index_for_pos(&instructions, *branch).unwrap();
                jump_indices.push(true_pos);
                let false_pos = i + 1;
                jump_indices.push(false_pos);
            }
            Instruction::Goto { branch } => {
                let jump_pos = get_index_for_pos(&instructions, *branch).unwrap();
                jump_indices.push(jump_pos);
            }
            _ => {}
        }
    }

    let raw_blocks = split_at_multiple(instructions.clone(), jump_indices);
    let mut blocks: HashMap<u64, Block> = raw_blocks
        .iter()
        .map(|el| {
            (
                el[0].0,
                Block {
                    instructions: el.clone(),
                    branches: Vec::new(),
                },
            )
        })
        .collect();

    //store jumps
    for (_, block) in &mut blocks {
        let (last_pos, last_instr) = block.instructions.last().unwrap();
        let next = instructions
            .iter()
            .skip_while(|el| el.0 <= *last_pos)
            .next();

        match last_instr {
            Instruction::IfNe { branch }
            | Instruction::IfEq { branch }
            | Instruction::IfLe { branch }
            | Instruction::IfGe { branch }
            | Instruction::IfGt { branch }
            | Instruction::IfLt { branch }
            | Instruction::IfICmpEq { branch }
            | Instruction::IfICmpNe { branch }
            | Instruction::IfICmpGt { branch }
            | Instruction::IfICmpGe { branch }
            | Instruction::IfICmpLt { branch }
            | Instruction::IfICmpLe { branch } => {
                let next_pos = next.unwrap().0;
                block.branches.push(*branch as u64);
                block.branches.push(next_pos);
            }
            Instruction::Goto { branch } => {
                block.branches.push(*branch as u64);
            }
            Instruction::Return
            | Instruction::AReturn
            | Instruction::IReturn
            | Instruction::LReturn
            | Instruction::DReturn
            | Instruction::FReturn => {}
            _ => {
                let next_pos = next.unwrap().0;
                block.branches.push(next_pos);
            }
        }
    }
    blocks
}

fn find_paths(blocks: &HashMap<u64, Block>, node: u64, path_in: Vec<u64>) -> Vec<Vec<u64>> {
    let block: &Block = blocks.get(&node).unwrap();
    let start_vector = vec![node];
    let mut path = path_in;
    path.push(node);

    let mut paths = Vec::new();
    if block.branches.len() == 0 {
        paths.push(start_vector);
    } else {
        for b in &block.branches {
            if !path.contains(b) {
                for p in find_paths(blocks, *b, path.clone()) {
                    let mut v = start_vector.clone();
                    v.extend(p.iter());
                    paths.push(v);
                }
            } else {
                let mut v = start_vector.clone();
                v.push(*b);
                paths.push(v);
            }
        }
    }
    paths
}

#[derive(Debug, Clone)]
enum VarType {
    Reference,
    Int,
    Float,
    Long,
    Double,
    Byte,
}

impl Display for VarType {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "{}",
            match self {
                VarType::Reference => panic!("can't understand reference cast"),
                VarType::Int => "int",
                VarType::Float => "float",
                VarType::Double => "double",
                VarType::Long => "long",
                VarType::Byte => "byte",
            }
        )
    }
}

#[derive(Debug, Clone)]
enum AST {
    BasicCast {
        value: Box<AST>,
        cast_type: VarType,
    },
    ClassCast {
        value: Box<AST>,
        cast_type: ConstClassData,
    },
    Static {
        field_data: ConstFieldData,
    },
    Variable {
        index: u16,
        vartype: VarType,
    },
    Call {
        method_data: ConstMethodData,
        reference: Box<AST>,
        args: Vec<AST>,
    },
    ArrayLength {
        reference: Box<AST>,
    },
    ConstInt {
        value: i64,
    },
    ConstFloat {
        value: f64,
    },
    ConstString {
        value: String,
    },
    VoidReturn,
    Set {
        index: u16,
        value: Box<AST>,
    },
    Mul {
        lhs: Box<AST>,
        rhs: Box<AST>,
    },
}

impl AST {
    fn to_java(&self, is_static: bool, get_class_name: fn(&str) -> String) -> String {
        match self {
            AST::Set { index, value } => {
                let var_name = if *index == 0 && !is_static {
                    format!("this")
                } else {
                    format!("var{}", index)
                };
                format!(
                    "{} = {};",
                    var_name,
                    value.to_java(is_static, get_class_name)
                )
            }
            AST::Variable { index, vartype: _ } => {
                if *index == 0 && !is_static {
                    format!("this")
                } else {
                    format!("var{}", index)
                }
            }
            AST::Call {
                method_data,
                reference,
                args,
            } => {
                let reference = reference.to_java(is_static, get_class_name);
                let name = &method_data.name_and_type.name;
                let args = args
                    .iter()
                    .map(|e| e.to_java(is_static, get_class_name))
                    .collect::<Vec<String>>()
                    .join(", ");
                format!("{}.{}({});", reference, name, args)
            }
            AST::Mul { lhs, rhs } => format!(
                "{} * {}",
                lhs.to_java(is_static, get_class_name),
                rhs.to_java(is_static, get_class_name)
            ),
            AST::ConstInt { value } => format!("{}", value),
            AST::ConstFloat { value } => format!("{}", value),
            AST::VoidReturn => String::from("return;"),
            AST::BasicCast { cast_type, value } => format!(
                "(({}) ({}))",
                cast_type,
                value.to_java(is_static, get_class_name)
            ),
            AST::ClassCast { cast_type, value } => format!(
                "(({}) ({}))",
                get_class_name(&cast_type.name),
                value.to_java(is_static, get_class_name)
            ),
            _ => unimplemented!("{:?}", self),
        }
    }
}

fn decompile_block(
    block: &Block,
    constant_pool: &ConstantPool,
) -> Result<Vec<AST>, DecompilerError> {
    let mut statements = Vec::new();

    let mut stack: Vec<AST> = Vec::new();
    for (pos, code) in &block.instructions {
        println!("{}: {:?}", pos, code);
        match code {
            Instruction::ILoad { index } => {
                stack.push(AST::Variable {
                    index: *index,
                    vartype: VarType::Int,
                });
            }
            Instruction::LLoad { index } => {
                stack.push(AST::Variable {
                    index: *index,
                    vartype: VarType::Long,
                });
            }
            Instruction::FLoad { index } => {
                stack.push(AST::Variable {
                    index: *index,
                    vartype: VarType::Float,
                });
            }
            Instruction::DLoad { index } => {
                stack.push(AST::Variable {
                    index: *index,
                    vartype: VarType::Double,
                });
            }
            Instruction::ALoad { index } => {
                stack.push(AST::Variable {
                    index: *index,
                    vartype: VarType::Reference,
                });
            }
            Instruction::InvokeSpecial { index } | Instruction::InvokeVirtual { index } => {
                let method = constant_pool.get_method_or_interface_entry(*index)?;
                let descriptor =
                    descriptors::parse_method(method.name_and_type.descriptor.chars())?;
                println!("{:?}", descriptor);
                let mut args = Vec::new();
                for _ in 0..descriptor.0.len() {
                    args.push(stack.pop().ok_or(DecompilerError::EmptyStack)?);
                }
                args.reverse();
                let reference = Box::new(stack.pop().ok_or(DecompilerError::EmptyStack)?);
                if descriptor.1 == descriptors::FieldType::Void {
                    statements.push(AST::Call {
                        method_data: method,
                        reference,
                        args,
                    });
                } else {
                    stack.push(AST::Call {
                        method_data: method,
                        reference,
                        args,
                    });
                }
            }
            Instruction::Return => {
                statements.push(AST::VoidReturn);
            }
            Instruction::IStore { index }
            | Instruction::LStore { index }
            | Instruction::FStore { index }
            | Instruction::DStore { index }
            | Instruction::AStore { index } => {
                statements.push(AST::Set {
                    index: *index,
                    value: Box::new(stack.pop().ok_or(DecompilerError::EmptyStack)?),
                });
            }
            Instruction::GetStatic { index } => {
                let field = constant_pool.get_field_entry(*index)?;
                stack.push(AST::Static { field_data: field });
            }
            Instruction::ArrayLength => {
                let reference = Box::new(stack.pop().ok_or(DecompilerError::EmptyStack)?);
                stack.push(AST::ArrayLength { reference });
            }
            Instruction::LoadConst { index } => {
                let value = match constant_pool.get_entry(*index)? {
                    ConstantPoolInfo::String { string_index } => AST::ConstString {
                        value: constant_pool.get_utf8_entry(string_index)?,
                    },
                    ConstantPoolInfo::Long { data } => AST::ConstInt { value: data },
                    ConstantPoolInfo::Integer { data } => AST::ConstInt { value: data as i64 },
                    ConstantPoolInfo::Double { data } => AST::ConstFloat { value: data },
                    ConstantPoolInfo::Float { data } => AST::ConstFloat { value: data as f64 },
                    _ => unimplemented!(),
                };
                stack.push(value);
            }
            Instruction::IConst { value } => stack.push(AST::ConstInt {
                value: *value as i64,
            }),
            Instruction::IMul => {
                let rhs = Box::new(stack.pop().ok_or(DecompilerError::EmptyStack)?);
                let lhs = Box::new(stack.pop().ok_or(DecompilerError::EmptyStack)?);
                stack.push(AST::Mul { lhs, rhs });
            }
            Instruction::I2b => {
                let cast_type = VarType::Byte;
                let value = Box::new(stack.pop().ok_or(DecompilerError::EmptyStack)?);
                stack.push(AST::BasicCast { cast_type, value })
            }
            Instruction::CheckCast { index } => {
                let cast_type = constant_pool.get_class_entry(*index)?;
                let value = Box::new(stack.pop().ok_or(DecompilerError::EmptyStack)?);
                stack.push(AST::ClassCast { cast_type, value })
            }
            _ => unimplemented!(),
        }
    }
    if stack.len() != 0 {
        return Err(DecompilerError::StackSize { size: stack.len() });
    }
    Ok(statements)
}

fn get_class_name(raw_name: &str) -> String {
    String::from(raw_name)
}

pub fn decompile(class: ClassFile) -> Result<(), DecompilerError> {
    for method in class.methods {
        println!(
            "{}",
            class
                .constant_pool
                .get_utf8_entry(method.name_index)
                .unwrap()
        ); //TODO
        for attrib in method.attributes {
            if let AttributeInfo::Code {
                max_stack: _,
                max_locals: _,
                code,
                exception_table: _,
                attributes: _,
            } = attrib
            {
                let instructions: Vec<(u64, Instruction)> = disassembler::disassemble(code)?;
                for (p, i) in &instructions {
                    println!("{}: {:?}", p, i);
                }
                let control_flow_graph = gen_control_flow_graph(&instructions);
                let paths = find_paths(&control_flow_graph, 0, Vec::new());
                println!("{:?}", paths);
                for block in control_flow_graph.values() {
                    for statement in decompile_block(block, &class.constant_pool)? {
                        println!(
                            "{}",
                            statement.to_java(method.access_flags.acc_static, get_class_name)
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
