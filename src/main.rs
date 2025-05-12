#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;

use clap::{arg, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version,about,long_about=None)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Init,
    #[command()]
    CatFile {
        #[arg(short = 'p')]
        pretty_print: bool,
        #[arg()]
        hash: String,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.cmd {
        Commands::Init => {
            let args: Vec<String> = env::args().collect();
            if args[1] == "init" {
                fs::create_dir(".git")?;
                fs::create_dir(".git/objects")?;
                fs::create_dir(".git/refs")?;
                fs::write(".git/HEAD", "ref: refs/heads/main\n")?;
                println!("Initialized git directory")
            } else {
                println!("unknown command: {}", args[1])
            }
        }
        Commands::CatFile { pretty_print, hash } => {
            todo!()
        }
    }

    Ok(())
}
