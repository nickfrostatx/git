use std::io::Read;
use std::fs::File;
use std::path::Path;
use types::GitResult;

// Read a ref, recurse if there is ever a symbolic ref
// TODO: deal with symbolic ref loops
fn read_ref_raw(path: &Path) -> GitResult<String> {
    let data = {
        let mut buf = String::new();
        let mut f = File::open(path)?;
        f.read_to_string(&mut buf)?;
        // Don't need a newline
        buf.pop();
        buf
    };

    if data.starts_with("ref: ") {
        // Symbolic ref
        read_ref(&data[5..])
    } else {
        Ok(data)
    }
}

// Find the ref that corresponds to a refname, and read it
pub fn read_ref_from_refname(refname: &str) -> GitResult<String> {
    let to_try = [
        Path::new(".git").join(refname),
        Path::new(".git/refs").join(refname),
        Path::new(".git/refs/tags").join(refname),
        Path::new(".git/refs/heads").join(refname),
        Path::new(".git/refs/remotes").join(refname),
        Path::new(".git/refs/heads").join(refname).join("HEAD"),
    ];
    for path in to_try.iter() {
        if path.is_file() {
            return read_ref_raw(&path);
        }
    }
    Err("unknown revision or refname not in the working tree".into())
}

// Read a ref from an exact refname
pub fn read_ref(refname: &str) -> GitResult<String> {
    read_ref_raw(&Path::new(".git").join(refname))
}
