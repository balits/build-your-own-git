#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;
use std::io;
use std::io::BufRead;
use std::io::Read;
use std::io::Write;

use anyhow::Context;
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
            anyhow::ensure!(pretty_print, "Only pretty printing is supported now");
            anyhow::ensure!(hash.len() == 40, "Only supporting 40 length hashes");

            let path = format!(".git/objects/{}/{}", &hash[..2], &hash[2..]);
            let file = fs::File::open(path).context("failed to open file")?;
            let mut reader = io::BufReader::new(file);
            let mut buf = Vec::new();

            reader
                .read_until(0, &mut buf)
                .context("Failed to read header")?;
            let header = std::str::from_utf8(&buf).context("Parsing header")?;
            let (kind, size) = header.strip_suffix("\0").unwrap().split_once(" ").unwrap();
            let size = size
                .parse::<usize>()
                .context("Couldnt parse header's size")?;

            if kind != "blob" {
                anyhow::bail!("Only reading blobs is supported");
            }

            let mut z = flate2::bufread::ZlibDecoder::new(reader);

            buf.clear();
            buf.resize(size, 0);
            z.read_to_end(&mut buf)
                .context(format!("Reading {} bytes from object", size))?;
            let more = z
                .read(&mut buf)
                .context("Attempting to read more from object")?;
            anyhow::ensure!(
                more == 0,
                format!(
                    "Object contained {} more bytes than expected {}",
                    more, size
                )
            );
            io::stdout()
                .lock()
                .write_all(&buf)
                .context("Couldnt write to stdout")?;
        }
    }

    Ok(())
}
