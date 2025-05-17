use std::fs;
use std::io;
use std::io::BufRead;
use std::io::Read;
use std::io::Write;

use anyhow::Context;
use clap::{arg, Parser, Subcommand};
use flate2::Compression;

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
            fn compress_and_write<R, W>(
                mut src: R,
                mut dest: W,
                filesize: u64,
            ) -> anyhow::Result<String>
            where
                R: Read,
                W: Write,
            {
                use sha1::Digest;
                let mut hasher = sha1::Sha1::new();

                let mut total_read = 0;
                let mut buf = vec![0u8; READ_CHUNK_SIZE];

                let header = format!("blob {}\0", filesize);
                dest.write(header.as_bytes())
                    .context("Writing expected header")?;
                Digest::update(&mut hasher, header);

                loop {
                    let n = src
                        .read(&mut buf[..])
                        .context("Reading a chunk of the file")?;
                    if n == 0 {
                        break; // EOF
                    }
                    total_read += n;
                    dest.write(&buf[..n])
                        .context("Writing a chunk of the file")?;
                    Digest::update(&mut hasher, &buf[..n]);
                }
                dest.flush()
                    .context("Flushing remaining bytes to the file")?;

                anyhow::ensure!(
                    total_read as u64 == filesize, // <- usize fits into u64
                    "Expected to read {filesize}, got {total_read}"
                );

                let hash = format!("{:x}", hasher.finalize());
                anyhow::ensure!(hash.len() == 40, "Hash length is not 40");

                Ok(hash)
            }

            let reader = io::BufReader::new(fs::File::open(&filename).context("Opening file")?);

            let attr = fs::metadata(&filename)
                .context(format!("Reading metadata of file {}", &filename))?;
            if !attr.is_file() {
                anyhow::bail!("Only hashing blobs is supported");
            }

            // something might might mingle with the file after calling metadata()
            // so we take a refernece to the original file size, then compare it with
            // the number of bytes we read
            let filesize = attr.len();

            let hash = if write {
                let tmp = "tmp";
                let outf = fs::File::create(tmp).context("Creating temporary file")?;
                let w = io::BufWriter::new(outf);
                let z = flate2::write::ZlibEncoder::new(w, Compression::default());

                let hash = compress_and_write(reader, z, filesize)?;
                let dirpath = format!(".git/objects/{}/", &hash[..2]);
                fs::create_dir_all(&dirpath).with_context(|| {
                    format!("creating directory for compressed file at {}", &dirpath)
                })?;
                fs::rename(tmp, format!("{}/{}", &dirpath, &hash[2..]))?;

                hash
            } else {
                compress_and_write(reader, io::sink(), filesize)?
            };

            println!("{}", hash);
        }
    }

    Ok(())
}
