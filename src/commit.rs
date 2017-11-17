extern crate chrono;

use std::io::{Cursor, Read};
use cache::{Object, ObjectType};
use chrono::{DateTime, FixedOffset};
use parse;
use types::{GitError, GitResult};

pub struct Commit {
    pub tree: String,
    pub parents: Vec<String>,
    pub author: String,
    pub author_date: DateTime<FixedOffset>,
    pub committer: String,
    pub committer_date: DateTime<FixedOffset>,
    pub message: String,
}

// Convert a vec of hex bytes into a String
// Only the characters 0-9 and a-f are accepted
fn string_from_hex_bytes(data: &[u8]) -> GitResult<String> {
    let mut result = String::new();
    for byte in data {
        if (*byte < b'0' || *byte > b'9') && (*byte < b'a' && *byte > b'f') {
            return Err(GitError::from("Invalid hex character"));
        }
        result.push(*byte as char);
    }
    Ok(result)
}

// Parse bytestring of the form "blah" to (author, date)
fn parse_author_line(mut line: String) -> GitResult<(String, DateTime<FixedOffset>)> {
    // Get offset string and date timestamp
    let last_space = match line.rfind(' ') {
        Some(ndx) => ndx,
        None => return Err(GitError::from("Malformed author line")),
    };
    let second_to_last_space = match line[..last_space].rfind(' ') {
        Some(ndx) => ndx,
        None => return Err(GitError::from("Malformed author line")),
    };
    let datestr = line.split_off(second_to_last_space).split_off(1);

    let time = match DateTime::parse_from_str(&datestr, "%s %z") {
        Ok(t) => t,
        Err(_) => return Err(GitError::from("Malformed author line")),
    };

    Ok((line, time))
}

pub fn from_object(object: &Object) -> GitResult<Commit> {
    if object.kind != ObjectType::Commit {
        return Err(GitError::from("Expected a commit object"));
    }
    let mut cursor = Cursor::new(&object.data);

    // Parse tree
    let tree = {
        let tree_line = try!(parse::read_until(&mut cursor, b'\n'));
        if tree_line.len() != 45 || &tree_line[0..5] != b"tree " {
            return Err(GitError::from("Malformed commit object"));
        }
        try!(string_from_hex_bytes(&tree_line[5..45]))
    };

    // Parse parents
    let mut parents: Vec<String> = Vec::new();
    let mut line_type = try!(parse::read_until(&mut cursor, b' '));
    while &line_type == b"parent" {
        let parent = try!(parse::read_until(&mut cursor, b'\n'));
        if parent.len() != 40 {
            return Err(GitError::from("Malformed commit object"));
        }
        parents.push(try!(string_from_hex_bytes(&parent)));

        line_type = try!(parse::read_until(&mut cursor, b' '));
    }

    // Parse author
    if &line_type != b"author" {
        return Err(GitError::from("Malformed commit object"));
    }
    let author_line = try!(String::from_utf8(
            try!(parse::read_until(&mut cursor, b'\n'))));
    let (author, author_date) = try!(parse_author_line(author_line));

    // Parse committer
    line_type = try!(parse::read_until(&mut cursor, b' '));
    if &line_type != b"committer" {
        return Err(GitError::from("Malformed commit object"));
    }
    let committer_line = try!(String::from_utf8(
            try!(parse::read_until(&mut cursor, b'\n'))));
    let (committer, committer_date) = try!(parse_author_line(committer_line));
    
    // Read either empty line, or gpgsig
    {
        let mut mt_line = try!(parse::read_until(&mut cursor, b'\n'));
        if &mt_line[..6] == b"gpgsig" {
            loop {
                let gpg_line = try!(parse::read_until(&mut cursor, b'\n'));
                if &gpg_line == b" -----END PGP SIGNATURE-----" {
                    break;
                }
            }
            // Now we should definitely get the empty line
            mt_line = try!(parse::read_until(&mut cursor, b'\n'));
        }
        if &mt_line != b"" {
            return Err(GitError::from("Malformed commit object"));
        }
    }

    // Read the rest of the commit
    let mut msg_bytes = Vec::new();
    try!(cursor.read_to_end(&mut msg_bytes));
    let message = try!(String::from_utf8(msg_bytes));

    Ok(Commit {
        tree: tree,
        parents: parents,
        author: author,
        author_date: author_date,
        committer: committer,
        committer_date: committer_date,
        message: message,
    })
}

impl Commit {
    pub fn to_object(&self) -> Object {
        let mut data = String::new();

        data.push_str(&format!("tree {}\n", self.tree));
        for parent in &self.parents {
            data.push_str(&format!("parent {}\n", parent));
        }
        data.push_str(&format!("author {} ", self.author));
        data.push_str(&self.author_date.format("%s %z\n").to_string());
        data.push_str(&format!("committer {} ", self.committer));
        data.push_str(&self.committer_date.format("%s %z\n").to_string());
        data.push_str(&self.message);

        Object {
            kind: ObjectType::Commit,
            data: data.into_bytes(),
        }
    }
}
