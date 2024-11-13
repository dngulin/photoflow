mod db;
mod exif_orientation;
mod index_cmd;
mod load_cmd;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

fn main() {
    let args = Args::parse();

    match args.command {
        Command::Index { db, source } => {
            index_cmd::execute(&db, &source[0]).unwrap();
        }
        Command::Open { db } => {
            if let Err(e) = load_cmd::execute(&db) {
                eprintln!("Error: {}", e);
            }
        }
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Index {
        db: PathBuf,
        #[clap(short, long)]
        source: Vec<PathBuf>,
    },
    Open {
        db: PathBuf,
    },
}
