use anyhow::Context;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};

use std::ffi::CStr;
use std::fmt::Display;
use std::fs::{self, File};
use std::io::{self, BufReader, prelude::*};
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Blob,
    Tree,
    Commit,
}

impl Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Kind::Blob => write!(f, "blob"),
            Kind::Tree => write!(f, "tree"),
            Kind::Commit => write!(f, "commit"),
        }
    }
}

pub(crate) struct Object<R> {
    pub(crate) kind: Kind,
    pub(crate) expected_size: u64,
    pub(crate) reader: R,
}

impl Object<()> {
    pub(crate) fn blob_from_file(file_path: impl AsRef<Path>) -> anyhow::Result<Object<impl Read>> {
        let file_path = file_path.as_ref();
        let metadata =
            fs::metadata(file_path).with_context(|| format!("metadata {}", file_path.display()))?;

        let file =
            File::open(file_path).with_context(|| format!("open {}", file_path.display()))?;

        Ok(Object {
            kind: Kind::Blob,
            expected_size: metadata.len(),
            reader: file,
        })
    }

    pub(crate) fn read_by_hash(hash: &str) -> anyhow::Result<Object<impl BufRead>> {
        // TODO: support shortest-unique object hashes
        let file = File::open(format!(".git/objects/{}/{}", &hash[..2], &hash[2..],))
            .context("open in .git/objects")?;

        let decoder = ZlibDecoder::new(file);
        let mut reader = BufReader::new(decoder);

        let mut buffer = Vec::new();
        // read object header into buffer
        reader
            .read_until(0, &mut buffer)
            .context("read header from .git/objects")?;

        let header = CStr::from_bytes_with_nul(&buffer)
            .expect("know there is exactly one nul, and it's at the end");
        let header = header
            .to_str()
            .context(".git/objects file header isn't valid UTF-8")?;

        let Some((kind, size)) = header.split_once(' ') else {
            anyhow::bail!(
                ".git/objects file header has wrong format, expected: 'kind size', got: '{header}'"
            );
        };

        let kind = match kind {
            "blob" => Kind::Blob,
            "tree" => Kind::Tree,
            "commit" => Kind::Commit,
            _ => anyhow::bail!("what even is a'{kind}'"),
        };

        let size = size
            .parse::<u64>()
            .context(format!(".git/objects file header has invalid size: {size}"))?;

        // NOTE: this won't error if decompressed file is too long, but will at least not
        // spam stdout and be vulnerable to a zipbomb.
        let reader = reader.take(size);

        Ok(Object {
            kind,
            expected_size: size,
            reader,
        })
    }
}

impl<R> Object<R>
where
    R: Read,
{
    pub(crate) fn write(mut self, writer: impl Write) -> anyhow::Result<[u8; 20]> {
        let writer = ZlibEncoder::new(writer, Compression::default());
        let mut writer = HashWriter {
            writer,
            hasher: Sha1::new(),
        };
        write!(writer, "{} {}\0", self.kind, self.expected_size)?;

        io::copy(&mut self.reader, &mut writer).context("stream file into blob")?;

        let _ = writer.writer.finish()?;
        let hash = writer.hasher.finalize();

        Ok(hash.into())
    }

    pub(crate) fn write_to_objects(self) -> anyhow::Result<[u8; 20]> {
        let tmp_file_name = "temporary";

        let hash = self
            .write(File::create(tmp_file_name).context("construct temporary file for tree")?)
            .context("stream tree object into tree object file")?;
        let hash_hex = hex::encode(hash);

        fs::create_dir_all(format!(".git/objects/{}/", &hash_hex[..2]))
            .context("create a subdir of .git/objects")?;
        fs::rename(
            tmp_file_name,
            format!(".git/objects/{}/{}", &hash_hex[..2], &hash_hex[2..]),
        )
        .context("move tree file into .git/objects")?;

        Ok(hash)
    }
}

struct HashWriter<W> {
    writer: W,
    hasher: Sha1,
}

impl<W> Write for HashWriter<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.writer.write(buf)?;
        self.hasher.update(&buf[..n]);

        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
