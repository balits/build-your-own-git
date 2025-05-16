use std::fs;
use std::io;
use std::io::BufRead;
use std::io::Read;
use std::io::Write;

use anyhow::Context;
use clap::{arg, Parser, Subcommand};
use flate2::Compression;
use sha1::digest::Update;
use sha1::{Digest, Sha1};

const READ_CHUNK_SIZE: usize = 1024 * 1;

#[derive(Parser, Debug)]
#[command(version,about,long_about=None)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command()]
    Init,
    #[command()]
    CatFile {
        #[arg(short = 'p')]
        pretty_print: bool,
        #[arg()]
        hash: String,
    },
    #[command()]
    HashObject {
        #[arg(short)]
        write: bool,

        #[arg()]
        filename: String,
    },
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    match args.cmd {
        Commands::Init => {
            fs::create_dir(".git")?;
            fs::create_dir(".git/objects")?;
            fs::create_dir(".git/refs")?;
            fs::write(".git/HEAD", "ref: refs/heads/main\n")?;
            println!("Initialized git directory")
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
        Commands::HashObject { write, filename } => {
            let mut total = 0;
            let mut reader = io::BufReader::new(fs::File::open(&filename).context("Opening file")?);
            let mut buf = vec![0u8; READ_CHUNK_SIZE];
            let mut hasher = Sha1::new();

            let attr = fs::metadata(&filename)
                .context(format!("Reading metadata of file {}", &filename))?;
            let filesize = attr.len(); // < TOUTOC or smth
            if !attr.is_file() {
                anyhow::bail!("Only hashing blobs is supported");
            }

            if write {
                let outf = fs::File::create("tmp").context("Creating temporary file")?;
                let w = io::BufWriter::new(outf);
                let mut z = flate2::write::ZlibEncoder::new(w, Compression::default());
                let mut i = 0;
                loop {
                    let n = reader
                        .read(&mut buf[..])
                        .context("Reading chunks of the file")?;
                    if n == 0 {
                        //EOF
                        break;
                    }
                    total += n;
                    log::debug!("Chunk({}):  {} bytes of {}", i, total, attr.len());
                    Update::update(&mut hasher, &buf[..n]);
                    z.write(&buf[..n]).context("Writing chunk to the file")?;
                    i += 1;
                }
                z.flush().context("Flushing remaining bytes to the file")?;

                log::debug!("File size {}, got {}", &filesize, &total);
            } else {
                loop {
                    let n = reader
                        .read(&mut buf[..])
                        .context("Reading chunks of the file")?;
                    if n == 0 {
                        //EOF
                        break;
                    }
                    total += n;
                    Update::update(&mut hasher, &buf[..n]);
                }
            }

            anyhow::ensure!(
                total as u64 == filesize,
                "Expected file size {filesize} got {total}"
            );
            Update::update(&mut hasher, "blob \0".as_bytes());
            Update::update(&mut hasher, format!("{}", filesize).as_bytes());

            println!("{:x}", hasher.finalize());
        }
    }

    Ok(())
}
