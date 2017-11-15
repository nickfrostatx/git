extern crate flate2;
extern crate sha1;

use cache::{Object, ObjectType, write_obj, read_obj};
use types::GitResult;
use std::env;
use std::io::{self, Write, Read};

mod cache;
mod types;

fn cat_file(hash: &str) -> GitResult<()> {
    let obj = try!(read_obj(hash));
    try!(io::stdout().write(&obj.data));
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("usage: {} <command> [<args>]", &args[0]);
        return;
    }

    if args[1] == "cat-file" {
        if args.len() != 3 {
            println!("usage: {} cat-file <sha1>", &args[0]);
            return;
        }
        match cat_file(&args[2]) {
            Ok(_) => (),
            Err(e) => println!("fatal: {}", e),
        }
    }
    else if args[1] == "hash-object" {
        let mut stdin = std::io::stdin();
        let mut data = Vec::with_capacity(1024);
        stdin.read_to_end(&mut data).expect("Reading from stdin");
        let obj = Object { kind: ObjectType::Blob, data: data };
        match write_obj(&obj) {
            Ok(hash) => println!("{}", hash),
            Err(e) => println!("fatal: {}", e),
        }
    }
}
