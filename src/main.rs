use anyhow::Context;
use clap::{Parser, Subcommand};

use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

mod commands;
mod objects;

use commands::{cat_file, commit_tree, hash_object, ls_tree, write_tree};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Init,
    CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,

        object_hash: String,
    },
    HashObject {
        #[clap(short = 'w')]
        write: bool,

        file: PathBuf,
    },
    LsTree {
        #[clap(long)]
        name_only: bool,

        tree_hash: String,
    },
    WriteTree,
    CommitTree {
        #[clap(short = 'm')]
        message: String,

        #[clap(short = 'p')]
        parent_hash: Option<String>,

        tree_hash: String,
    },
    Commit {
        #[clap(short = 'm')]
        message: String,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Init => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();

            println!("Initialized git directory");
        }
        Command::CatFile {
            pretty_print,
            object_hash,
        } => cat_file::invoke(pretty_print, &object_hash)?,
        Command::HashObject { write, file } => hash_object::invoke(write, &file)?,
        Command::LsTree {
            name_only,
            tree_hash,
        } => ls_tree::invoke(name_only, &tree_hash)?,
        Command::WriteTree => write_tree::invoke()?,
        Command::CommitTree {
            message,
            tree_hash,
            parent_hash,
        } => commit_tree::invoke(message, tree_hash, parent_hash)?,
        Command::Commit { message } => {
            let head_ref = fs::read_to_string(".git/HEAD").context("read HEAD")?;
            let Some(head_ref) = head_ref.strip_prefix("ref: ") else {
                anyhow::bail!("refusing to commit onto detached HEAD");
            };
            let head_ref = head_ref.trim();

            let parent_hash = fs::read_to_string(format!(".git/{head_ref}"))
                .with_context(|| format!("read HEAD reference target '{head_ref}'"))?;
            let parent_hash = parent_hash.trim();

            let Some(tree_hash) = write_tree::write_tree_for(Path::new("."))? else {
                eprintln!("not committing empty tree");
                return Ok(());
            };

            let commit_hash =
                commit_tree::write_commit(&message, &hex::encode(tree_hash), Some(parent_hash))
                    .context("create commit")?;
            let commit_hash = hex::encode(commit_hash);

            fs::write(format!(".git/{head_ref}"), &commit_hash)
                .with_context(|| format!("update HEAD reference target {head_ref}"))?;

            println!("HEAD is now at {commit_hash}");
        }
    }

    Ok(())
}
