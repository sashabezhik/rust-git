use anyhow::Context;

use std::ffi::CStr;
use std::io::{self, BufRead, Read, Write};

use crate::objects::{Kind, Object};

pub(crate) fn invoke(name_only: bool, tree_hash: &str) -> anyhow::Result<()> {
    let mut object = Object::read_by_hash(tree_hash).context("parse out tree object file")?;

    match object.kind {
        Kind::Tree => {
            let mut buffer = Vec::new();
            let mut hash_buffer = [0; 20];
            let mut stdout = io::stdout().lock();

            loop {
                buffer.clear();

                let n = object
                    .reader
                    .read_until(0, &mut buffer)
                    .context("read next tree object entry")?;

                if n == 0 {
                    break;
                }

                object
                    .reader
                    .read_exact(&mut hash_buffer[..])
                    .context("read tree entry object hash")?;

                let mode_and_name =
                    CStr::from_bytes_with_nul(&buffer).context("invalid tree entry")?;

                let mut bits = mode_and_name.to_bytes().splitn(2, |&b| b == b' ');
                let mode = bits.next().expect("split always yieldsd once");
                let name = bits
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("tree entry has no file name"))?;

                if name_only {
                    stdout
                        .write_all(name)
                        .context("write tree entry name to stdout")?;
                } else {
                    let mode = std::str::from_utf8(mode).context("mode is always valid utf-8")?;
                    let hash = hex::encode(&hash_buffer);
                    let object = Object::read_by_hash(&hash)
                        .with_context(|| format!("read object for tree entry {hash}"))?;

                    write!(stdout, "{mode:0>6} {} {hash} ", object.kind)
                        .context("write tree entry meta to stdout")?;

                    stdout
                        .write_all(name)
                        .context("write tree entry name to stdout")?;
                }

                writeln!(stdout, "").context("write newline to stdout")?;
            }
        }
        _ => anyhow::bail!("don't yet know how to ls '{}'", object.kind),
    }

    Ok(())
}
