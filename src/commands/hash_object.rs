use anyhow::Context;

use std::io;
use std::path::Path;

use crate::objects::Object;

pub(crate) fn invoke(write: bool, file_path: &Path) -> anyhow::Result<()> {
    let object = Object::blob_from_file(file_path).context("open blob input file")?;
    let hash = if write {
        object
            .write_to_objects()
            .context("stream file into blob object file")?
    } else {
        object
            .write(io::sink())
            .context("stream file into blob object")?
    };

    println!("{}", hex::encode(hash));

    Ok(())
}
