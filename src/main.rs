mod error;
mod epub;

use std::{fs, path, process::exit};
use error::Error;

pub type Result<T> = std::result::Result<T, error::Error>;

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
    let ebook = epub::Epub::new(path.unwrap()?)?;
    println!("{:?}", ebook.file_path);
    println!("{:?}", ebook.metadata);

    Ok(())
}
