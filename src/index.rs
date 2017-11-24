extern crate byteorder;

use cache::{Object, ObjectType};
use parse;
use self::byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::fs::{File, Metadata};
use std::io::{self, Read, Write};
use std::os::unix::fs::MetadataExt;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;
use std::time::UNIX_EPOCH;
use tree::{EntryMode, Tree, TreeEntry};
use types::{GitError, GitResult};

pub struct Index {
    pub entries: BTreeMap<Vec<u8>, IndexEntry>,
}

pub struct IndexEntry {
    pub ctime: u32,
    pub ctime_ns: u32,
    pub mtime: u32,
    pub mtime_ns: u32,
    pub dev: u32,
    pub ino: u32,
    pub mode: EntryMode,
    pub uid: u32,
    pub gid: u32,
    pub size: u32,
    pub assume_valid: bool,
    pub hash: [u8; 20],
}

pub fn read() -> GitResult<Index> {
    let mut file = match File::open(".git/index") {
        Ok(f) => f,
        Err(err) => match err.kind() {
            // If there is no index file, use an empty index
            io::ErrorKind::NotFound => return Ok(Index {
                entries: BTreeMap::new(),
            }),
            _ => return Err(GitError::from(err)),
        },
    };

    let mut sig = vec![0; 8];
    file.read_exact(&mut sig)?;
    if sig != b"DIRC\0\0\0\x02" {
        return Err(GitError::from("Bad index file signature"));
    }

    let num_entries = file.read_u32::<BigEndian>()? as usize;
    let mut entries: BTreeMap<Vec<u8>, IndexEntry> = BTreeMap::new();

    while entries.len() < num_entries {
        let ctime = file.read_u32::<BigEndian>()?;
        let ctime_ns = file.read_u32::<BigEndian>()?;
        let mtime = file.read_u32::<BigEndian>()?;
        let mtime_ns = file.read_u32::<BigEndian>()?;
        let dev = file.read_u32::<BigEndian>()?;
        let ino = file.read_u32::<BigEndian>()?;
        let mode = match file.read_u32::<BigEndian>()? {
            0b1000_000_110_100_100 => EntryMode::NormalFile,
            0b1000_000_111_101_101 => EntryMode::ExecutableFile,
            0b1010_000_000_000_000 => EntryMode::Symlink,
            _ => return Err(GitError::from("Bad entry mode in index")),
        };
        let uid = file.read_u32::<BigEndian>()?;
        let gid = file.read_u32::<BigEndian>()?;
        let size = file.read_u32::<BigEndian>()?;

        let mut hash = [0; 20];
        file.read_exact(&mut hash)?;

        let flags = file.read_u16::<BigEndian>()?;
        let assume_valid = flags & 0x8000 != 0;
        if flags & 0x4000 != 0 {
            return Err(GitError::from("Extended flag must be 0"));
        }
        // TODO: Actually do something with the stage
        //let stage = (flags & 0b0011000000000000) >> 12;
        let name_length = (flags & 0xfff) as usize;

        let name = parse::read_until(&mut file, b'\0')?;

        // Verify name length
        if !((name.len() == name_length)
             || (name.len() > 0xfff && name_length == 0xfff)) {
            return Err(GitError::from("Corrupted entry name"));
        }

        // Name is padded with NUL bytes until the entry is a multiple of 8 bytes
        let num_pad = 7 - (name.len() + 6) % 8;
        let mut padding = vec![0; num_pad];
        file.read_exact(&mut padding)?;
        if padding != vec![0; num_pad] {
            return Err(GitError::from("Found bytes in pad field"));
        }

        let entry = IndexEntry {
            ctime: ctime, ctime_ns: ctime_ns, mtime: mtime, mtime_ns: mtime_ns,
            dev: dev, ino: ino, mode: mode, uid: uid, gid: gid, size: size,
            assume_valid: assume_valid, hash: hash,
        };
        entries.insert(name, entry);
    }

    Ok(Index { entries: entries })
}

// Helper to track the SHA of the file's contents as we write to it
struct HashingWriter {
    file: File,
    hash: Sha1,
}

impl HashingWriter {
    fn digest(&self) -> Digest {
        self.hash.digest()
    }
}

impl Write for HashingWriter {
    fn write(&mut self, data: &[u8]) -> Result<usize, io::Error> {
        self.hash.update(data);
        self.file.write(data)
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        self.file.flush()
    }
}

impl Index {
    // Write out to index file
    pub fn write(&self) -> GitResult<()> {
        let file = File::create(".git/index")?;
        let hash = Sha1::new();
        let mut w = HashingWriter {file: file, hash: hash};

        w.write_all(b"DIRC\0\0\0\x02")?;
        w.write_u32::<BigEndian>(self.entries.len() as u32)?;

        for (name, entry) in self.entries.iter() {
            w.write_u32::<BigEndian>(entry.ctime)?;
            w.write_u32::<BigEndian>(entry.ctime_ns)?;
            w.write_u32::<BigEndian>(entry.mtime)?;
            w.write_u32::<BigEndian>(entry.mtime_ns)?;
            w.write_u32::<BigEndian>(entry.dev)?;
            w.write_u32::<BigEndian>(entry.ino)?;
            w.write_u32::<BigEndian>(match entry.mode {
                EntryMode::NormalFile => 0b1000_000_110_100_100,
                EntryMode::ExecutableFile => 0b1000_000_111_101_101,
                EntryMode::Symlink => 0b1010_000_000_000_000,
                _ => return Err(GitError::from("Unsupported index entry type")),
            })?;
            w.write_u32::<BigEndian>(entry.uid)?;
            w.write_u32::<BigEndian>(entry.gid)?;
            w.write_u32::<BigEndian>(entry.size)?;
            w.write_all(&entry.hash)?;

            let flags: u16 = if name.len() <= 0xfff {
                name.len() as u16
            } else {
                0xfff
            };
            w.write_u16::<BigEndian>(flags)?;

            w.write_all(&name)?;
            // Pad entry size to a multiple of 8 bytes, with NUL's
            let num_pad = 8 - (name.len() + 6) % 8;
            let padding = vec![0; num_pad];
            w.write_all(&padding)?;
        }

        let digest = w.digest().bytes();
        w.write_all(&digest)?;

        Ok(())
    }

    pub fn add(&mut self, path: &Path, meta: &Metadata) -> GitResult<()> {
        let name: Vec<u8> = match path.as_os_str().to_str() {
            Some(s) => s.as_bytes().to_vec(),
            None => return Err(GitError::from("Invalid UTF-8 filename")),
        };

        // Figure out file mode and data
        let file_type = meta.file_type();
        let (mode, data) = if file_type.is_file() {
            // Regular file, just read it
            let mut file = File::open(path)?;
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)?;
            if meta.mode() & 0b1_000_000 != 0 {
                (EntryMode::ExecutableFile, contents)
            } else {
                (EntryMode::NormalFile, contents)
            }
        } else if file_type.is_symlink() {
            // The data is the symlink destination
            let dest = path.read_link()?;
            (EntryMode::Symlink, dest.into_os_string().into_vec())
        } else {
            return Err(GitError::from("Tried to add a non-file"));
        };

        // Write the object and get the hash
        let hash: [u8; 20] = {
            let obj = Object { kind: ObjectType::Blob, data: data };
            obj.write()?.bytes()
        };

        // Unpack timestamps
        let (ctime, ctime_ns) = {
            let duration = meta.created()?.duration_since(UNIX_EPOCH)?;
            (duration.as_secs() as u32, duration.subsec_nanos())
        };
        let (mtime, mtime_ns) = {
            let duration = meta.modified()?.duration_since(UNIX_EPOCH)?;
            (duration.as_secs() as u32, duration.subsec_nanos())
        };

        self.entries.insert(name, IndexEntry {
            ctime: ctime,
            ctime_ns: ctime_ns,
            mtime: mtime,
            mtime_ns: mtime_ns,
            dev: meta.dev() as u32,
            ino: meta.ino() as u32,
            mode: mode,
            uid: meta.uid() as u32,
            gid: meta.gid() as u32,
            size: meta.size() as u32,
            assume_valid: false,
            hash: hash,
        });
        Ok(())
    }

    // Create trees
    pub fn write_tree(&self) -> GitResult<Digest> {
        // Create a stack of trees, With just the root initially
        let mut tree_stack: Vec<(Vec<u8>, Tree)> = Vec::new();
        tree_stack.push((b"root".to_vec(), Tree { entries: Vec::new() }));

        for (name, entry) in self.entries.iter() {
            // TODO: use std::path::Path for this
            let parts: Vec<&[u8]> = name.split(|c| *c == b'/').collect();

            // Figure out if we need to write out some trees from the stack
            for i in 1..tree_stack.len() {
                if i >= parts.len() || tree_stack[i].0 != parts[i - 1] {
                    truncate_tree_stack(&mut tree_stack, i)?;
                    break;
                }
            }

            // Append any new trees to the stack
            for part in parts[(tree_stack.len() - 1)..(parts.len() - 1)].iter() {
                let new_tree = Tree { entries: Vec::new() };
                tree_stack.push((Vec::from(*part), new_tree));
            }

            let bottom_tree = match tree_stack.last_mut() {
                Some(&mut (_, ref mut tree)) => tree,
                None => return Err(GitError::from("Unexpected error")),
            };
            bottom_tree.entries.push(TreeEntry {
                mode: entry.mode.clone(),
                name: Vec::from(parts[parts.len() - 1]),
                hash: entry.hash.clone(),
            });
        }

        // Write out the rest of the trees
        truncate_tree_stack(&mut tree_stack, 0)
    }
}

// Remove and write the trees on the stack starting from position at
// Return the hash of the highest level tree written
fn truncate_tree_stack(stack: &mut Vec<(Vec<u8>, Tree)>, at: usize)
        -> GitResult<Digest> {
    let mut result: Option<Digest> = None;
    while stack.len() > at {
        let (name, tree) = match stack.pop() {
            Some(tup) => tup,
            // This probably can't happen
            None => return Err(GitError::from("Unexpected error")),
        };
        // Write the tre
        let digest = tree.as_object().write()?;

        // Add an entry for tree in its parent
        match stack.last_mut() {
            Some(&mut (_, ref mut parent)) => parent.entries.push(TreeEntry {
                mode: EntryMode::Tree,
                name: name,
                hash: digest.bytes(),
            }),
            None => (),
        }
        result = Some(digest);
    }
    match result {
        Some(digest) => Ok(digest),
        None => Err(GitError::from("Tried to truncate empty stack")),
    }
}
