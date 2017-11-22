use std::io::Read;
use std::fs::File;
use std::path::Path;
use types::GitResult;

// Read a ref, recurse if there is ever a symbolic ref
// TODO: deal with symbolic ref loops
pub fn read_ref(name: &str) -> GitResult<String> {
    let data = {
        let mut buf = String::new();
        let mut f = File::open(&Path::new(".git").join(name))?;
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
pub fn expand_refname(refname: &str) -> GitResult<String> {
    let to_try = [
        Path::new(refname).to_path_buf(),
        Path::new("refs").join(refname),
        Path::new("refs/tags").join(refname),
        Path::new("refs/heads").join(refname),
        Path::new("refs/remotes").join(refname),
        Path::new("refs/heads").join(refname).join("HEAD"),
    ];
    for path in to_try.iter() {
        if Path::new(".git").join(path).is_file() {
            return match path.to_str() {
                Some(s) => Ok(String::from(s)),
                None => Err("Invalid UTF-8 string".into()),
            };
        }
    }
    Err("unknown revision or refname not in the working tree".into())
}
