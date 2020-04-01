use std::collections::HashMap;
use std::convert::From;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::Read;

mod mutf8 {
    pub enum MUtf8Error {
        MissingByte,
        UnknownByte,
        InvalidChar,
    }

    pub fn to_string<T: IntoIterator<Item = u8>>(bytes: T) -> Result<String, MUtf8Error> {
        let mut s = String::new();
        let mut iterator = bytes.into_iter();
        loop {
            if let Some(b) = iterator.next() {
                if b == 0b1110_1101 {
                    let b2 = iterator.next().ok_or(MUtf8Error::MissingByte)?;
                    if !b2 & 0b1111_0000 == 0b1010_0000 {
                        return Err(MUtf8Error::UnknownByte);
                    }
                    let b3 = iterator.next().ok_or(MUtf8Error::MissingByte)?;
                    if !b3 & 0b1100_0000 == 0b1000_0000 {
                        return Err(MUtf8Error::UnknownByte);
                    }
                    if !iterator.next().ok_or(MUtf8Error::MissingByte)? & 0xFF == 0b1110_1101 {
                        return Err(MUtf8Error::UnknownByte);
                    }
                    let b4 = iterator.next().ok_or(MUtf8Error::MissingByte)?;
                    if !b4 & 0b1111_0000 == 0b1011_0000 {
                        return Err(MUtf8Error::UnknownByte);
                    }
                    let b5 = iterator.next().ok_or(MUtf8Error::MissingByte)?;
                    if !b5 & 0b1100_0000 == 0b1000_0000 {
                        return Err(MUtf8Error::UnknownByte);
                    }
                    let codepoint: u32 = 0x10000
                        + ((b2 as u32 & 0x0f) << 16)
                        + ((b3 as u32 & 0x3f) << 10)
                        + ((b4 as u32 & 0x0f) << 6)
                        + (b5 as u32 & 0x3f);
                    s.push(std::char::from_u32(codepoint).ok_or(MUtf8Error::InvalidChar)?);
                } else if b & 0b1111_0000 == 0b1110_0000 {
                    let b2 = iterator.next().ok_or(MUtf8Error::MissingByte)?;
                    if !b2 & 0b1100_0000 == 0b1000_0000 {
                        return Err(MUtf8Error::UnknownByte);
                    }
                    let b3 = iterator.next().ok_or(MUtf8Error::MissingByte)?;
                    if !b3 & 0b1100_0000 == 0b1000_0000 {
                        return Err(MUtf8Error::UnknownByte);
                    }
                    let codepoint: u32 = b3 as u32 & 0b11_1111
                        | ((b2 as u32 & 0b11_1111) << 6)
                        | ((b as u32 & 0b1_1111) << 12);
                    s.push(std::char::from_u32(codepoint).ok_or(MUtf8Error::InvalidChar)?);
                } else if b & 0b1110_0000 == 0b1100_0000 {
                    let b2 = iterator.next().ok_or(MUtf8Error::MissingByte)?;
                    if !b2 & 0b1100_0000 == 0b1000_0000 {
                        return Err(MUtf8Error::UnknownByte);
                    }
                    let codepoint: u32 = b2 as u32 & 0b11_1111 | ((b as u32 & 0b1_1111) << 6);
                    s.push(std::char::from_u32(codepoint).ok_or(MUtf8Error::InvalidChar)?);
                } else if b & 0b1000_0000 == 0 {
                    s.push(b as char);
                } else {
                    return Err(MUtf8Error::UnknownByte);
                }
            } else {
                break;
            }
        }
        Ok(s)
    }
}

#[derive(Debug)]
pub enum ClassFileError {
    InvalidMagic,
    Read,
    InvalidCPType,
    InvalidCPEntry,
    MUtf8Format,
    EndOfFile,
    MoreData,
}

impl From<std::io::Error> for ClassFileError {
    fn from(_: std::io::Error) -> Self {
        ClassFileError::Read
    }
}

impl From<mutf8::MUtf8Error> for ClassFileError {
    fn from(_: mutf8::MUtf8Error) -> Self {
        ClassFileError::MUtf8Format
    }
}

impl Error for ClassFileError {}

impl Display for ClassFileError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "{}",
            match self {
                ClassFileError::InvalidMagic => "invalid magic value",
                ClassFileError::Read => "error reading input",
                ClassFileError::InvalidCPType => "invalid constant pool type",
                ClassFileError::InvalidCPEntry => "invalid index into constant pool",
                ClassFileError::MUtf8Format => "error in mutf8 format",
                ClassFileError::EndOfFile => "end of file",
                ClassFileError::MoreData => "more data after expected end of file",
            }
        )
    }
}

fn read_u8<T: Read>(data: &mut T) -> Result<u8, ClassFileError> {
    let mut buf = [0_u8; 1];
    let amt = data.read(&mut buf)?;
    if amt < 1 {
        return Err(ClassFileError::EndOfFile);
    }
    Ok(buf[0])
}

fn read_u16<T: Read>(data: &mut T) -> Result<u16, ClassFileError> {
    let mut buf = [0_u8; 2];
    let amt = data.read(&mut buf)?;
    if amt < 2 {
        return Err(ClassFileError::EndOfFile);
    }
    let r: u16 = unsafe { std::mem::transmute(buf) };
    Ok(r.to_be())
}

fn read_u32<T: Read>(data: &mut T) -> Result<u32, ClassFileError> {
    let mut buf = [0_u8; 4];
    let amt = data.read(&mut buf)?;
    if amt < 4 {
        return Err(ClassFileError::EndOfFile);
    }
    let r: u32 = unsafe { std::mem::transmute(buf) };
    Ok(r.to_be())
}

#[derive(Debug, Clone)]
pub enum ConstantPoolInfo {
    Class {
        name_index: u16,
    },
    FieldRef {
        class_index: u16,
        name_and_type_index: u16,
    },
    MethodRef {
        class_index: u16,
        name_and_type_index: u16,
    },
    InterfaceMethodRef {
        class_index: u16,
        name_and_type_index: u16,
    },
    String {
        string_index: u16,
    },
    Integer {
        data: i32,
    },
    Float {
        data: f32,
    },
    Long {
        data: i64,
    },
    Double {
        data: f64,
    },
    NameAndType {
        name_index: u16,
        descriptor_index: u16,
    },
    Utf8 {
        length: u16,
        string: String,
    },
    MethodHandle {
        reference_kind: u8,
        reference_index: u16,
    },
    MethodType {
        descriptor_index: u16,
    },
    InvokeDynamic {
        bootstrap_method_attr_index: u16,
        name_and_type_index: u16,
    },
}

#[derive(Debug)]
pub struct ConstantPool {
    data: HashMap<u16, ConstantPoolInfo>,
}

#[derive(Debug, Clone)]
pub struct ConstClassData {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ConstFieldData {
    class: ConstClassData,
    name_and_type: ConstNameTypeData,
}

#[derive(Debug, Clone)]
pub struct ConstMethodData {
    class: ConstClassData,
    pub name_and_type: ConstNameTypeData,
    is_interface: bool,
}

#[derive(Debug, Clone)]
pub struct ConstNameTypeData {
    pub name: String,
    pub descriptor: String,
}

impl ConstantPool {
    pub fn get_entry(&self, index: u16) -> Result<ConstantPoolInfo, ClassFileError> {
        Ok(self
            .data
            .get(&index)
            .ok_or(ClassFileError::InvalidCPEntry)?
            .clone())
    }

    pub fn get_utf8_entry(&self, index: u16) -> Result<String, ClassFileError> {
        if let ConstantPoolInfo::Utf8 { length: _, string } = self.get_entry(index)? {
            Ok(string)
        } else {
            Err(ClassFileError::InvalidCPEntry)
        }
    }

    pub fn get_class_entry(&self, index: u16) -> Result<ConstClassData, ClassFileError> {
        if let ConstantPoolInfo::Class { name_index } = self.get_entry(index)? {
            Ok(ConstClassData {
                name: self.get_utf8_entry(name_index)?,
            })
        } else {
            Err(ClassFileError::InvalidCPEntry)
        }
    }

    pub fn get_name_type_entry(&self, index: u16) -> Result<ConstNameTypeData, ClassFileError> {
        if let ConstantPoolInfo::NameAndType {
            name_index,
            descriptor_index,
        } = self.get_entry(index)?
        {
            Ok(ConstNameTypeData {
                name: self.get_utf8_entry(name_index)?,
                descriptor: self.get_utf8_entry(descriptor_index)?,
            })
        } else {
            Err(ClassFileError::InvalidCPEntry)
        }
    }

    pub fn get_field_entry(&self, index: u16) -> Result<ConstFieldData, ClassFileError> {
        if let ConstantPoolInfo::FieldRef {
            class_index,
            name_and_type_index,
        } = self.get_entry(index)?
        {
            Ok(ConstFieldData {
                class: self.get_class_entry(class_index)?,
                name_and_type: self.get_name_type_entry(name_and_type_index)?,
            })
        } else {
            Err(ClassFileError::InvalidCPEntry)
        }
    }

    pub fn get_method_or_interface_entry(
        &self,
        index: u16,
    ) -> Result<ConstMethodData, ClassFileError> {
        match self.get_entry(index)? {
            ConstantPoolInfo::MethodRef {
                class_index,
                name_and_type_index,
            } => Ok(ConstMethodData {
                class: self.get_class_entry(class_index)?,
                name_and_type: self.get_name_type_entry(name_and_type_index)?,
                is_interface: false,
            }),
            ConstantPoolInfo::InterfaceMethodRef {
                class_index,
                name_and_type_index,
            } => Ok(ConstMethodData {
                class: self.get_class_entry(class_index)?,
                name_and_type: self.get_name_type_entry(name_and_type_index)?,
                is_interface: true,
            }),
            _ => Err(ClassFileError::InvalidCPEntry),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

fn read_constant_pool<T: Read>(data: &mut T) -> Result<ConstantPool, ClassFileError> {
    let constant_pool_count = read_u16(data)?;
    let mut constant_pool = HashMap::new();
    let mut i = 1;
    while i < constant_pool_count {
        let cp_type = read_u8(data)?;
        let entry = match cp_type {
            7 => ConstantPoolInfo::Class {
                name_index: read_u16(data)?,
            },
            9 => ConstantPoolInfo::FieldRef {
                class_index: read_u16(data)?,
                name_and_type_index: read_u16(data)?,
            },
            10 => ConstantPoolInfo::MethodRef {
                class_index: read_u16(data)?,
                name_and_type_index: read_u16(data)?,
            },
            11 => ConstantPoolInfo::InterfaceMethodRef {
                class_index: read_u16(data)?,
                name_and_type_index: read_u16(data)?,
            },
            8 => ConstantPoolInfo::String {
                string_index: read_u16(data)?,
            },
            3 => ConstantPoolInfo::Integer {
                data: unsafe { std::mem::transmute(read_u32(data)?) },
            },
            4 => ConstantPoolInfo::Float {
                data: unsafe { std::mem::transmute(read_u32(data)?) },
            },
            5 => {
                let high = read_u32(data)?;
                let low = read_u32(data)?;
                ConstantPoolInfo::Long {
                    data: unsafe { std::mem::transmute([low, high]) },
                }
            }
            6 => {
                let high = read_u32(data)?;
                let low = read_u32(data)?;
                ConstantPoolInfo::Double {
                    data: unsafe { std::mem::transmute([low, high]) },
                }
            }
            12 => ConstantPoolInfo::NameAndType {
                name_index: read_u16(data)?,
                descriptor_index: read_u16(data)?,
            },
            1 => {
                let length = read_u16(data)?;
                let bytes_result: Result<Vec<_>, _> =
                    (0..length).into_iter().map(|_| read_u8(data)).collect();
                ConstantPoolInfo::Utf8 {
                    length,
                    string: mutf8::to_string(bytes_result?)?,
                }
            }
            15 => ConstantPoolInfo::MethodHandle {
                reference_kind: read_u8(data)?,
                reference_index: read_u16(data)?,
            },
            16 => ConstantPoolInfo::MethodType {
                descriptor_index: read_u16(data)?,
            },
            18 => ConstantPoolInfo::InvokeDynamic {
                bootstrap_method_attr_index: read_u16(data)?,
                name_and_type_index: read_u16(data)?,
            },
            _ => return Err(ClassFileError::InvalidCPType),
        };
        constant_pool.insert(i, entry);
        i += 1;
        if cp_type == 5 || cp_type == 6 {
            i += 1;
        }
    }
    Ok(ConstantPool {
        data: constant_pool,
    })
}

#[derive(Debug)]
pub struct ClassAccessFlags {
    pub acc_public: bool,
    pub acc_final: bool,
    pub acc_super: bool,
    pub acc_interface: bool,
    pub acc_abstract: bool,
    pub acc_synthetic: bool,
    pub acc_annotation: bool,
    pub acc_enum: bool,
}

fn read_class_access_flags<T: Read>(data: &mut T) -> Result<ClassAccessFlags, ClassFileError> {
    let flags = read_u16(data)?;
    Ok(ClassAccessFlags {
        acc_public: flags & 0x0001 > 0,
        acc_final: flags & 0x0010 > 0,
        acc_super: flags & 0x0020 > 0,
        acc_interface: flags & 0x0200 > 0,
        acc_abstract: flags & 0x0400 > 0,
        acc_synthetic: flags & 0x1000 > 0,
        acc_annotation: flags & 0x2000 > 0,
        acc_enum: flags & 0x4000 > 0,
    })
}

fn read_interfaces<T: Read>(data: &mut T) -> Result<Vec<u16>, ClassFileError> {
    let interaces_count = read_u16(data)?;
    let interaces_result: Result<Vec<_>, _> = (0..interaces_count)
        .into_iter()
        .map(|_| read_u16(data))
        .collect();
    Ok(interaces_result?)
}

#[derive(Debug)]
pub struct ExceptionTableInfo {
    start_pc: u16,
    end_pc: u16,
    handler_pc: u16,
    catch_type: u16,
}

#[derive(Debug)]
pub enum AttributeInfo {
    Raw {
        attribute_name: String,
        info: Vec<u8>,
    },
    ConstantValue {
        constant_value_index: u16,
    },
    Code {
        max_stack: u16,
        max_locals: u16,
        code: Vec<u8>,
        exception_table: Vec<ExceptionTableInfo>,
        attributes: Vec<AttributeInfo>,
    },
    SourceFile {
        sourcefile_index: u16,
    },
}

fn read_attributes<T: Read>(
    data: &mut T,
    constant_pool: &ConstantPool,
) -> Result<Vec<AttributeInfo>, ClassFileError> {
    let attributes_count = read_u16(data)?;
    let mut attributes = Vec::new();

    for _ in 0..attributes_count {
        let attribute_name_index = read_u16(data)?;
        let attribute_name = constant_pool.get_utf8_entry(attribute_name_index)?;
        let attribute_length = read_u32(data)?;

        let attribute = match attribute_name.as_str() {
            "ConstantValue" => AttributeInfo::ConstantValue {
                constant_value_index: read_u16(data)?,
            },
            "SourceFile" => AttributeInfo::SourceFile {
                sourcefile_index: read_u16(data)?,
            },
            "Code" => {
                let max_stack = read_u16(data)?;
                let max_locals = read_u16(data)?;
                let code_length = read_u32(data)?;
                let code_result: Result<Vec<_>, _> = (0..code_length)
                    .into_iter()
                    .map(|_| read_u8(data))
                    .collect();
                let exception_table_length = read_u16(data)?;
                let mut exception_table = Vec::with_capacity(exception_table_length as usize);
                for _ in 0..exception_table_length {
                    exception_table.push(ExceptionTableInfo {
                        start_pc: read_u16(data)?,
                        end_pc: read_u16(data)?,
                        handler_pc: read_u16(data)?,
                        catch_type: read_u16(data)?,
                    });
                }
                let inner_attributes = read_attributes(data, constant_pool)?;
                AttributeInfo::Code {
                    max_stack,
                    max_locals,
                    code: code_result?,
                    exception_table,
                    attributes: inner_attributes,
                }
            }
            _ => {
                let bytes_result: Result<Vec<_>, _> = (0..attribute_length)
                    .into_iter()
                    .map(|_| read_u8(data))
                    .collect();
                AttributeInfo::Raw {
                    attribute_name,
                    info: bytes_result?,
                }
            }
        };
        attributes.push(attribute);
    }
    Ok(attributes)
}

#[derive(Debug)]
pub struct FieldAccessFlags {
    pub acc_public: bool,
    pub acc_private: bool,
    pub acc_protected: bool,
    pub acc_static: bool,
    pub acc_final: bool,
    pub acc_volatile: bool,
    pub acc_transient: bool,
    pub acc_synthetic: bool,
    pub acc_enum: bool,
}

fn read_field_access_flags<T: Read>(data: &mut T) -> Result<FieldAccessFlags, ClassFileError> {
    let flags = read_u16(data)?;
    Ok(FieldAccessFlags {
        acc_public: flags & 0x0001 > 0,
        acc_private: flags & 0x0002 > 0,
        acc_protected: flags & 0x0004 > 0,
        acc_static: flags & 0x0008 > 0,
        acc_final: flags & 0x0010 > 0,
        acc_volatile: flags & 0x0040 > 0,
        acc_transient: flags & 0x0080 > 0,
        acc_synthetic: flags & 0x1000 > 0,
        acc_enum: flags & 0x4000 > 0,
    })
}

#[derive(Debug)]
pub struct FieldInfo {
    pub access_flags: FieldAccessFlags,
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes: Vec<AttributeInfo>,
}

fn read_fields<T: Read>(
    data: &mut T,
    constant_pool: &ConstantPool,
) -> Result<Vec<FieldInfo>, ClassFileError> {
    let fields_count = read_u16(data)?;
    let mut fields = Vec::new();
    for _ in 0..fields_count {
        let access_flags = read_field_access_flags(data)?;
        let name_index = read_u16(data)?;
        let descriptor_index = read_u16(data)?;
        let attributes = read_attributes(data, constant_pool)?;
        let field = FieldInfo {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        };
        fields.push(field);
    }
    Ok(fields)
}

#[derive(Debug)]
pub struct MethodAccessFlags {
    pub acc_public: bool,
    pub acc_private: bool,
    pub acc_protected: bool,
    pub acc_static: bool,
    pub acc_final: bool,
    pub acc_synchronized: bool,
    pub acc_bridge: bool,
    pub acc_varargs: bool,
    pub acc_native: bool,
    pub acc_abstract: bool,
    pub acc_strict: bool,
    pub acc_synthetic: bool,
}

fn read_method_access_flags<T: Read>(data: &mut T) -> Result<MethodAccessFlags, ClassFileError> {
    let flags = read_u16(data)?;
    Ok(MethodAccessFlags {
        acc_public: flags & 0x0001 > 0,
        acc_private: flags & 0x0002 > 0,
        acc_protected: flags & 0x0004 > 0,
        acc_static: flags & 0x0008 > 0,
        acc_final: flags & 0x0010 > 0,
        acc_synchronized: flags & 0x0020 > 0,
        acc_bridge: flags & 0x0040 > 0,
        acc_varargs: flags & 0x0080 > 0,
        acc_native: flags & 0x0100 > 0,
        acc_abstract: flags & 0x0400 > 0,
        acc_strict: flags & 0x0800 > 0,
        acc_synthetic: flags & 0x1000 > 0,
    })
}

#[derive(Debug)]
pub struct MethodInfo {
    pub access_flags: MethodAccessFlags,
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes: Vec<AttributeInfo>,
}

fn read_methods<T: Read>(
    data: &mut T,
    constant_pool: &ConstantPool,
) -> Result<Vec<MethodInfo>, ClassFileError> {
    let methods_count = read_u16(data)?;
    let mut methods = Vec::new();
    for _ in 0..methods_count {
        let access_flags = read_method_access_flags(data)?;
        let name_index = read_u16(data)?;
        let descriptor_index = read_u16(data)?;
        let attributes = read_attributes(data, constant_pool)?;
        let field = MethodInfo {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        };
        methods.push(field);
    }
    Ok(methods)
}

#[derive(Debug)]
pub struct ClassFile {
    pub major_version: u16,
    pub minor_version: u16,
    pub constant_pool: ConstantPool,
    pub access_flags: ClassAccessFlags,
    pub this_class: u16,
    pub super_class: u16,
    pub interfaces: Vec<u16>,
    pub fields: Vec<FieldInfo>,
    pub methods: Vec<MethodInfo>,
    pub attributes: Vec<AttributeInfo>,
}

pub fn read_classfile<T: Read>(data: &mut T) -> Result<ClassFile, ClassFileError> {
    if read_u32(data)? != 0xcafebabe {
        return Err(ClassFileError::InvalidMagic);
    }
    let minor_version = read_u16(data)?;
    let major_version = read_u16(data)?;

    let constant_pool = read_constant_pool(data)?;

    let access_flags = read_class_access_flags(data)?;

    let this_class = read_u16(data)?;
    let super_class = read_u16(data)?;

    let interfaces = read_interfaces(data)?;
    let fields = read_fields(data, &constant_pool)?;
    let methods = read_methods(data, &constant_pool)?;
    let attributes = read_attributes(data, &constant_pool)?;

    if let Ok(_) = read_u8(data) {
        return Err(ClassFileError::MoreData);
    }

    Ok(ClassFile {
        major_version,
        minor_version,
        constant_pool,
        access_flags,
        this_class,
        super_class,
        interfaces,
        fields,
        methods,
        attributes,
    })
}
