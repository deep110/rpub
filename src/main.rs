mod error;
mod xml;
mod epub;
mod reader;

use std::{fs::{self}, path, process::exit};

pub type Result<T> = std::result::Result<T, error::Error>;

use std::fs::OpenOptions;
use std::io::Write;
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref LOG_FILE: Mutex<std::fs::File> = Mutex::new(
        OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open("debug.log")
            .expect("Failed to open log file")
    );
}

pub fn log(message: &str) {
    if let Ok(mut file) = LOG_FILE.lock() {
        if let Err(e) = writeln!(file, "{}", message) {
            eprintln!("Logging error: {}", e);
        }
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        crate::log(&format!($($arg)*));
    };
}


#[derive(argh::FromArgs)]
// #[argh(help_triggers("-h", "--help", "help"))]
/// read a book
struct Args {
    #[argh(positional)]
    path: Option<String>,

    /// print reading history
    #[argh(switch, short = 'r')]
    history: bool,

    /// characters per line
    #[argh(option, short = 'w', default = "75")]
    width: u16,
}

fn get_ebook_path(path: Option<String>) -> Option<Result<path::PathBuf>> {
    return match path {
        None => {
            // TODO: read from history
            None
        },
        Some(actual_path) => {
            Some(fs::canonicalize(&actual_path)
                    .map_err(|_| error::to_fnf_error(actual_path))
            )
        },
    }
}


fn main() -> Result<()> {
    lazy_static::initialize(&LOG_FILE);

    let args: Args = argh::from_env();

    if args.history {
        println!("TODO: Print history");
        return Ok(());
    }

    let path = get_ebook_path(args.path);
    if path.is_none() {
        println!("No ebook provided or in history");
        exit(1);
    }
    let mut ebook = epub::Epub::new(path.unwrap()?)?;

    reader::read_ebook(&mut ebook)?;

    // println!("{:?}", ebook.chapters);
    // println!("TOC: {:?}", ebook.toc);

    Ok(())
}
