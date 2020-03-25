use javaclass::{ClassFile, ClassFileError, FieldAccessFlags, MethodAccessFlags};
use std::fs::File;

fn field_flags_as_modifiers(flags: &FieldAccessFlags) -> String {
    let mut modifiers = String::new();

    if flags.acc_public {
        modifiers += "public ";
    }
    if flags.acc_protected {
        modifiers += "protected ";
    }
    if flags.acc_private {
        modifiers += "private ";
    }
    if flags.acc_static {
        modifiers += "static ";
    }
    if flags.acc_final {
        modifiers += "final ";
    }
    if flags.acc_transient {
        modifiers += "transient ";
    }
    if flags.acc_volatile {
        modifiers += "volatile ";
    }
    modifiers
}

fn method_flags_as_modifiers(flags: &MethodAccessFlags) -> String {
    let mut modifiers = String::new();

    if flags.acc_public {
        modifiers += "public ";
    }
    if flags.acc_protected {
        modifiers += "protected ";
    }
    if flags.acc_private {
        modifiers += "private ";
    }
    if flags.acc_abstract {
        modifiers += "abstract ";
    }
    if flags.acc_static {
        modifiers += "static ";
    }
    if flags.acc_final {
        modifiers += "final ";
    }
    if flags.acc_synchronized {
        modifiers += "synchronized ";
    }
    if flags.acc_native {
        modifiers += "native ";
    }
    if flags.acc_strict {
        modifiers += "strict ";
    }
    modifiers
}

fn decompile(class: ClassFile) -> Result<(), ClassFileError> {
    println!("{}", serde_json::to_string_pretty(&class).unwrap());
    /*for field in class.fields {
        let descriptor = class.constant_pool.get_utf8_entry(field.descriptor_index)?;
        let name = class.constant_pool.get_utf8_entry(field.name_index)?;
        println!(
            "{}{} {}",
            field_flags_as_modifiers(&field.access_flags),
            descriptor,
            name
        );
    }
    for method in class.methods {
        let descriptor = class.constant_pool.get_utf8_entry(method.descriptor_index)?;
        let name = class.constant_pool.get_utf8_entry(method.name_index)?;
        println!(
            "{}{} {}",
            method_flags_as_modifiers(&method.access_flags),
            descriptor,
            name
        );
    }*/
    Ok(())
}

fn main() -> Result<(), ClassFileError> {
    let mut file = File::open("testdata/Particle.class").unwrap();
    let class_file: ClassFile = javaclass::read_classfile(&mut file)?;
    decompile(class_file)?;
    Ok(())
}
