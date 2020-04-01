extern crate javadec;

use clap::{App, Arg, ArgMatches};
use std::error::Error;
use std::fmt::Display;
use std::fs::File;
use std::io::Read;

#[derive(Debug)]
struct ContextError {
    error: Box<dyn Error>,
    context: String,
}

impl Error for ContextError {}

impl Display for ContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", self.context, self.error)
    }
}

trait ToContextError<T, E> {
    fn context_err(self, context: &str) -> Result<T, ContextError>
    where
        E: Error + Send + Sync + 'static;
}

impl<T, E> ToContextError<T, E> for Result<T, E> {
    fn context_err(self, context: &str) -> Result<T, ContextError>
    where
        E: Error + Send + Sync + 'static,
    {
        match self {
            Ok(val) => Ok(val),
            Err(e) => Err(ContextError {
                error: Box::new(e),
                context: String::from(context),
            }),
        }
    }
}

fn run(matches: ArgMatches) -> Result<(), ContextError> {
    for val in matches
        .values_of("INPUT")
        .expect("missing required argument")
    {
        let mut file = File::open(val).context_err(val)?;
        if val.ends_with(".jar") {
            let mut archive = zip::ZipArchive::new(file).context_err(val)?;
            for i in 0..archive.len() {
                let mut zfile = archive.by_index(i).context_err(val)?;
                println!("{}", zfile.name());
                if zfile.name().ends_with(".class") {
                    let mut full = Vec::new();
                    zfile.read_to_end(&mut full).context_err(val)?;
                    let mut data = std::io::Cursor::new(full);
                    let classfile = javaclass::read_classfile(&mut data).context_err(val)?;
                    javadec::decompile(classfile).context_err(val)?;
                }
            }
        } else {
            let classfile = javaclass::read_classfile(&mut file).context_err(val)?;
            javadec::decompile(classfile).context_err(val)?;
        }
    }
    Ok(())
}

fn main() {
    let name = "javadec";
    let matches = App::new(name)
        .version("0.1.0")
        .author("Ian Rehwinkel <ian.rehwinkel@tutanota.com>")
        .about("Java 8 decompiler")
        .arg(
            Arg::with_name("INPUT")
                .required(true)
                .multiple(true)
                .help("Files to be decompiled (.jar or .class)"),
        )
        .get_matches();

    match run(matches) {
        Err(e) => eprintln!("{}: {}", name, e),
        Ok(_) => {}
    }
}
