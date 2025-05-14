#[allow(unused_imports)]
use std::env;
use std::ffi::CString;
#[allow(unused_imports)]
use std::fs;
use std::io;
use std::io::BufRead;
use std::io::Read;
use std::io::Write;

use anyhow::Context;
use bytes::Buf;
use bytes::BufMut;
use clap::{arg, Parser, Subcommand};
use flate2::FlushCompress;

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
            anyhow::ensure!(pretty_print, "Only pretty printing is supported now");
            anyhow::ensure!(hash.len() == 40, "Only 40 length hashes are supported");

            let path = format!(".git/objects/{}/{}", &hash[..2], &hash[2..]);
            let file = fs::File::open(path).context("Failed to open file")?;
            let z = flate2::read::ZlibDecoder::new(file);
            let mut bz = io::BufReader::new(z);
            let mut buf = Vec::new();
            bz.read_until(0, &mut buf)
                .context("Failed to read header")?;
            let header = std::str::from_utf8(&buf[..buf.len()]).context("Parsing header")?;
            let (kind, size) = header.strip_suffix("\0").unwrap().split_once(" ").unwrap();
            let size = size
                .parse::<usize>()
                .context("Couldnt parse header's size")?;

            if kind != "blob" {
                anyhow::bail!("Only reading blobs is supported");
            }
            buf.clear();
            buf.resize(size, 0);
            bz.read_exact(&mut buf)
                .context(format!("Couldnt read blob file"))?;

            io::stdout()
                .lock()
                .write_all(&buf)
                .context("Couldnt write to stdout")?;
        }
    }

    Ok(())
}
