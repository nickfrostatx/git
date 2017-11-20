use std::io::{self, Read};
use std::fs::File;
use std::path::Path;
use types::GitResult;

// If the file exists, return its contents
// If the file does not exist, resturn none
// If any other error occurred, returns an error
fn read_path(path: &Path) -> GitResult<Option<String>> {
    match File::open(path) {
        Ok(mut f) => {
            let mut buf = String::new();
            f.read_to_string(&mut buf);
            Ok(Some(buf))
        },
        Err(err) => match err.kind() {
            io::ErrorKind::NotFound => Ok(None),
            _ => Err(err.into()),
        },
    }
}

// Macro that returns Ok if an option is a Some, and does nothing otherwise
macro_rules! return_if_some {
    ($e:expr) => (match $e {
        Some(v) => return Ok(v),
        None => (),
    })
}

// Read a rev
pub fn rev_parse(path: &str) -> GitResult<String> {
    return_if_some!(read_path(&Path::new(".git").join(path))?);
    return_if_some!(read_path(&Path::new(".git/refs").join(path))?);
    return_if_some!(read_path(&Path::new(".git/refs/tags").join(path))?);
    return_if_some!(read_path(&Path::new(".git/refs/heads").join(path))?);
    return_if_some!(read_path(&Path::new(".git/refs/remotes").join(path))?);
    return_if_some!(read_path(&Path::new(".git/refs/remotes").join(path)
                                    .join("HEAD"))?);
    Err("unknown revision or path not in the working tree".into())
}

// Read a ref
pub fn read_ref() -> GitResult<Option<String>> {
    Ok(None)
}
