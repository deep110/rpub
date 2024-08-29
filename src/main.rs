use std::{fs, process::exit};

// mod epub;

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

fn get_ebook_path(path: Option<String>) -> Option<std::path::PathBuf> {

    None
}


fn main() {
    let args: Args = argh::from_env();

    if args.history {
        println!("TODO: Print history");

        return
    }

    let path = get_ebook_path(args.path);
    if path.is_none() {
        println!("No ebook provided or in history");
        exit(1);
    }
    let _tpath = path.unwrap();
}
