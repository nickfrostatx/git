use std::fs;
use std::io::prelude::*;
use std::io;
use std::path::PathBuf;
use std::str;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};

use parse;
use types::{GitError, GitResult};

#[derive(PartialEq, Eq)]
pub enum ObjectType {
    Blob,
    Tree,
    Commit,
    Tag,
}

pub struct Object {
    pub kind: ObjectType,
    pub data: Vec<u8>,
}

// The directory that contains an object
fn dir_for_hash(obj_hash: &str) -> PathBuf {
    let mut path = PathBuf::from(".git/objects");
    path.push(&obj_hash[..2]);
    path
}

// The full filename for an object
fn path_for_hash(obj_hash: &str) -> PathBuf {
    let mut path = dir_for_hash(obj_hash);
    path.push(&obj_hash[2..]);
    path
}

pub fn read_obj(hash: &str) -> GitResult<Object> {
    let f = fs::File::open(path_for_hash(hash))?;
    let mut decoder = ZlibDecoder::new(f);
    let type_str = parse::read_until(&mut decoder, b' ')?;

    let kind: ObjectType = match &type_str[..] {
        b"blob" => ObjectType::Blob,
        b"commit" => ObjectType::Commit,
        b"tree" => ObjectType::Tree,
        b"tag" => ObjectType::Tag,
        _ => return Err(GitError::from("Invalid object type")),
    };
    
    let expected_size = {
        let bytes = parse::read_until(&mut decoder, b'\0')?;
        let s = String::from_utf8(bytes)?;
        s.parse::<usize>()?
    };

    let mut data = vec![0; expected_size];
    decoder.read_exact(&mut data)?;

    Ok(Object{
        kind: kind,
        data: data,
    })
}

impl Object {
    pub fn write(self) -> GitResult<Digest> {
        let kind_name = match self.kind {
            ObjectType::Blob => "blob",
            ObjectType::Commit => "commit",
            ObjectType::Tree => "tree",
            ObjectType::Tag => "tag",
        };

        let header = format!("{0} {1}\0", kind_name, self.data.len()).into_bytes();

        // Compute object SHA1
        let mut m = Sha1::new();
        m.update(&header);
        m.update(&self.data);
        let digest = m.digest();
        let name = digest.to_string();

        // Create containing directory
        match fs::create_dir(dir_for_hash(&name)) {
            Ok(_) => (),
            Err(err) => match err.kind() {
                io::ErrorKind::AlreadyExists => (),
                _ => return Err(GitError::from(err)),
            },
        }

        // Actually create the file
        let file_path = path_for_hash(&name);
        match fs::OpenOptions::new().write(true).create_new(true).open(file_path) {
            Ok(f) => {
                // Write object
                let mut encoder = ZlibEncoder::new(f, Compression::Default);
                encoder.write(&header)?;
                encoder.write(&self.data)?;
                encoder.finish()?;
            },
            Err(err) => match err.kind() {
                // It's fine if the object with that hash already exists
                io::ErrorKind::AlreadyExists => (),
                _ => return Err(GitError::from(err)),
            },
        }

        Ok(digest)
    }
}
