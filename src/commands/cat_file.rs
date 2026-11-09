use anyhow::Context;

use std::io;

use crate::objects::{Kind, Object};

pub(crate) fn invoke(pretty_print: bool, object_hash: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        pretty_print,
        "mode must be given without -p, and we don't support mode"
    );

    let mut object = Object::read_by_hash(object_hash).context("parse out blob object file")?;

    match object.kind {
        Kind::Blob => {
            let mut stdout = io::stdout().lock();

            let n = io::copy(&mut object.reader, &mut stdout)
                .context("write .git/objects file into stdout")?;

            anyhow::ensure!(
                n == object.expected_size,
                ".git/objects file was not the expected size (expected: {}, actual: {n})",
                object.expected_size,
            );
        }
        _ => anyhow::bail!("don't yet know how to print '{}'", object.kind),
    }

    Ok(())
}
