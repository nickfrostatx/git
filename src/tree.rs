use cache::{Object, ObjectType};
use std::io::{BufRead, Cursor, Read, Write};
use parse;
use types::{GitError, GitResult};

pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

pub enum EntryMode {
    NormalFile,
    ExecutableFile,
    Symlink,
    Tree,
}

pub struct TreeEntry {
    pub mode: EntryMode,
    pub name: Vec<u8>,
    pub hash: [u8; 20],
}

pub fn from_object(object: &Object) -> GitResult<Tree> {
    if object.kind != ObjectType::Tree {
        return Err(GitError::from("Expected a tree object"));
    }
    let mut cursor = Cursor::new(&object.data);

    let mut entries: Vec<TreeEntry> = Vec::new();
    loop {
        let mut mode_bytes = Vec::new();
        if try!(cursor.read_until(b' ', &mut mode_bytes)) == 0 {
            break;
        }
        let mode = match &*mode_bytes {
            b"100644 " => EntryMode::NormalFile,
            b"100755 " => EntryMode::ExecutableFile,
            b"120000 " => EntryMode::Symlink,
            b"40000 "  => EntryMode::Tree,
            _ => return Err(GitError::from("Malformed tree object")),
        };
        let name = try!(parse::read_until(&mut cursor, b'\0'));
        let mut hash: [u8; 20] = [0; 20];
        try!(cursor.read_exact(&mut hash));
        entries.push(TreeEntry { mode: mode, name: name, hash: hash });
    }

    Ok(Tree { entries: entries })
}

impl Tree {
    pub fn as_object(&self) -> Object {
        let mut data: Vec<u8> = Vec::new();

        for entry in self.entries.iter() {
            let mode_bytes = match entry.mode {
                EntryMode::NormalFile => b"100644".as_ref(),
                EntryMode::ExecutableFile => b"100755".as_ref(),
                EntryMode::Symlink => b"120000".as_ref(),
                EntryMode::Tree => b"40000".as_ref(),
            };
            // Vec<u8>.write_all will never error
            data.write_all(mode_bytes).unwrap();
            data.push(b' ');
            data.write_all(&entry.name).unwrap();
            data.push(b'\0');
            data.write_all(&entry.hash).unwrap();
        }

        Object { kind: ObjectType::Tree, data: data }
    }
}
