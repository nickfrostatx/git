extern crate chrono;
extern crate flate2;
extern crate sha1;

use cache::{Object, ObjectType, read_obj};
use commit::Commit;
use tree::EntryMode;
use types::GitResult;
use std::env;
use std::io::{self, Write, Read};

mod cache;
mod commit;
mod index;
mod parse;
mod tree;
mod types;

fn cat_file(hash: &str) -> GitResult<()> {
    let obj = try!(read_obj(hash));
    try!(io::stdout().write(&obj.data));
    Ok(())
}

fn hash_object() -> GitResult<()> {
    let mut stdin = std::io::stdin();
    let mut data = Vec::new();
    try!(stdin.read_to_end(&mut data));
    let obj = Object { kind: ObjectType::Blob, data: data };
    let hash = try!(obj.write());
    println!("{}", hash);
    Ok(())
}

fn show_commit(hash: &str) -> GitResult<()> {
    let obj = try!(read_obj(hash));
    let commit = try!(commit::from_object(&obj));
    println!("commit {}", hash);
    println!("Author: {}", commit.author);
    println!("Date:   {}", commit.author_date.format("%a %e %b %H:%M:%S %Y %z"));
    println!("\n{}", commit.message);
    Ok(())
}

fn write_commit(parents: &[String]) -> GitResult<()> {
    let message = {
        let mut stdin = std::io::stdin();
        let mut data = Vec::new();
        try!(stdin.read_to_end(&mut data));
        try!(String::from_utf8(data))
    };
    // TODO
    let author = "Nick Frost <nickfrostatx@gmail.com>";
    let localtime = chrono::Local::now();
    let author_date = localtime.with_timezone(localtime.offset());

    let tree = try!(try!(index::read()).write_tree()).to_string();

    let commit = Commit {
        tree: String::from(tree),
        parents: Vec::from(parents),
        author: String::from(author),
        author_date: author_date,
        committer: String::from(author),
        committer_date: author_date.clone(),
        message: message,
    };
    let hash = try!(commit.as_object().write());
    println!("{}", hash.to_string());

    Ok(())
}

fn show_tree(hash: &str) -> GitResult<()> {
    let obj = try!(read_obj(hash));
    let tree = try!(tree::from_object(&obj));

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
                 try!(String::from_utf8(entry.name)));
    }

    Ok(())
}

fn write_tree() -> GitResult<()> {
    let ndx = try!(index::read());
    println!("{}", try!(ndx.write_tree()));
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("usage: {} <command> [<args>]", &args[0]);
        return;
    }

    let result = match args[1].as_ref() {
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
        "commit" => {
            if args.len() < 2 {
                println!("usage: {} commit [<parent> ..]", &args[0]);
                return;
            }
            write_commit(&args[2..])
        },
        "show-tree" => {
            if args.len() != 3 {
                println!("usage: {} commit <sha1>", &args[0]);
                return;
            }
            show_tree(&args[2])
        },
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
