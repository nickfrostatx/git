extern crate chrono;
extern crate flate2;
extern crate sha1;

use cache::{Object, ObjectType, read_obj};
use commit::Commit;
use index::Index;
use tree::EntryMode;
use types::{GitError, GitResult};
use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

mod cache;
mod commit;
mod index;
mod parse;
mod refs;
mod tree;
mod types;

fn cat_file(hash: &str) -> GitResult<()> {
    let obj = read_obj(hash)?;
    io::stdout().write(&obj.data)?;
    Ok(())
}

fn hash_object() -> GitResult<()> {
    let mut stdin = std::io::stdin();
    let mut data = Vec::new();
    stdin.read_to_end(&mut data)?;
    let obj = Object { kind: ObjectType::Blob, data: data };
    let hash = obj.write()?;
    println!("{}", hash);
    Ok(())
}

fn show_commit(hash: &str) -> GitResult<()> {
    let obj = read_obj(hash)?;
    let commit = commit::from_object(&obj)?;
    println!("commit {}", hash);
    println!("Author: {}", commit.author);
    println!("Date:   {}", commit.author_date.format("%a %e %b %H:%M:%S %Y %z"));
    println!("\n{}", commit.message);
    Ok(())
}

// Drop the user into vim so they can write a commit message
fn prompt_commit_message() -> GitResult<Option<String>> {
    // Create file
    {
        let mut file = File::create(".git/COMMIT_EDITMSG")?;
        file.write(b"
# Please enter the commit message for your changes. Lines starting
# with '#' will be ignored, and an empty message aborts the commit.\n")?;
    }

    // Drop the user into vim
    Command::new("vim")
                 .arg(".git/COMMIT_EDITMSG")
                 .status()?;

    // Read and parse the file
    let file = File::open(".git/COMMIT_EDITMSG")?;
    parse_commit_message(file)
}

// Read and parse a commit edit message file
fn parse_commit_message(f: File) -> GitResult<Option<String>> {
    // This could enforce some stricter rules
    let reader = BufReader::new(f);
    let mut message: String = String::new();
    let mut has_content = false;
    for line_res in reader.lines() {
        let line = line_res?;
        if line.get(0..1) == Some("#") {
            continue;
        }
        if line.len() > 0 {
            has_content = true;
        }
        message.push_str(&line);
        message.push('\n');
    }
    Ok(match has_content {
        true => Some(message),
        false => None,
    })
}

fn write_commit(parents: &[String]) -> GitResult<()> {
    // TODO
    let author = "Nick Frost <nickfrostatx@gmail.com>";
    let localtime = chrono::Local::now();
    let author_date = localtime.with_timezone(localtime.offset());

    let tree = index::read()?.write_tree()?.to_string();

    let message = match prompt_commit_message()? {
        Some(msg) => msg,
        None => {
            println!("Aborting commit due to empty commit message.");
            return Ok(());
        },
    };

    let commit = Commit {
        tree: String::from(tree),
        parents: Vec::from(parents),
        author: String::from(author),
        author_date: author_date,
        committer: String::from(author),
        committer_date: author_date.clone(),
        message: message,
    };
    let hash = commit.as_object().write()?;
    println!("{}", hash.to_string());

    Ok(())
}

fn show_tree(hash: &str) -> GitResult<()> {
    let obj = read_obj(hash)?;
    let tree = tree::from_object(&obj)?;

    for entry in tree.entries {
        let mode_string = match entry.mode {
            EntryMode::NormalFile => "100644",
            EntryMode::ExecutableFile => "100755",
            EntryMode::Symlink => "120000",
            EntryMode::Tree => "040000",
        };
        let kind_str = match entry.mode {
            EntryMode::Tree => "tree",
            _ => "blob",
        };
        let mut hash_hex = String::new();
        for byte in &entry.hash {
            hash_hex.push_str(&format!("{:02x}", byte));
        }
        println!("{0} {1} {2}    {3}", mode_string, kind_str, hash_hex,
                 String::from_utf8(entry.name)?);
    }

    Ok(())
}

fn write_tree() -> GitResult<()> {
    let ndx = index::read()?;
    println!("{}", ndx.write_tree()?);
    Ok(())
}

fn make_relative(path: &Path) -> Option<PathBuf> {
    // TODO
    Some(path.to_path_buf())
}

fn should_ignore(path: &Path) -> bool {
    const IGNORE_NAMES: [&str; 3] = [".git", "target", "Cargo.lock"];
    let to_ignore: HashSet<OsString> =
            IGNORE_NAMES.iter().map(|p| OsString::from(p)).collect();
    return path.components().any(
            |p| to_ignore.contains(p.as_os_str()))
}

fn add_recursive(ndx: &mut Index, path: &Path) -> GitResult<()> {
    if should_ignore(path) {
        return Ok(());
    }

    let meta = fs::symlink_metadata(path)?;
    let file_type = meta.file_type();

    if file_type.is_file() || file_type.is_symlink() {
        // Just add the file
        ndx.add(&path, &meta)?;
    } else if file_type.is_dir() {
        // Recurse
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            add_recursive(ndx, &entry.path())?;
        }
    } else {
        // Skip any files that aren't paths, symlinks, or directories
    }
    Ok(())
}

fn add(paths: &[String]) -> GitResult<()> {
    if paths.len() == 0 {
        println!("Nothing specified, nothing added.");
        println!("Maybe you wanted to say 'git add .'?");
        return Ok(());
    }

    let mut ndx = index::read()?;
    for path in paths {
        match make_relative(&Path::new(path)) {
            Some(p) => add_recursive(&mut ndx, &p)?,
            None => (),
        }
    }
    ndx.write()
}

fn rev_parse(paths: &[String]) -> GitResult<()> {
    for path in paths {
        println!("{}", refs::rev_parse(&path)?);
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("usage: {} <command> [<args>]", &args[0]);
        return;
    }

    let result = match args[1].as_ref() {
        // Porcelain commands (I plan on implementing all of these)
        "add" => add(&args[2..]),
        "branch" => Err(GitError::from("Command not implemented")),
        "commit" => write_commit(&args[2..]),
        "diff" => Err(GitError::from("Command not implemented")),
        "fsck" => Err(GitError::from("Command not implemented")),
        "init" => Err(GitError::from("Command not implemented")),
        "log" => Err(GitError::from("Command not implemented")),
        "merge" => Err(GitError::from("Command not implemented")),
        "show" => Err(GitError::from("Command not implemented")),
        "status" => Err(GitError::from("Command not implemented")),
        // Plumbing commands
        "cat-file" =>  {
            if args.len() != 3 {
                println!("usage: {} cat-file <sha1>", &args[0]);
                return;
            }
            cat_file(&args[2])
        },
        "hash-object" => hash_object(),
        "show-commit" => {
            if args.len() != 3 {
                println!("usage: {} show-commit <sha1>", &args[0]);
                return;
            }
            show_commit(&args[2])
        },
        "show-tree" => {
            if args.len() != 3 {
                println!("usage: {} commit <sha1>", &args[0]);
                return;
            }
            show_tree(&args[2])
        },
        "rev-parse" => rev_parse(&args[2..]),
        "write-tree" => write_tree(),
        _ => {
            println!("usage: {} <command> [<args>]", &args[0]);
            return;
        },
    };

    match result {
        Ok(_) => (),
        Err(err) => println!("fatal: {}", err),
    }
}
