use std::error;
use std::fmt;
use std::io;
use std::num;
use std::str;
use std::string;

#[derive(Debug)]
pub enum GitError {
    Message(&'static str),
    IoError(io::Error),
}

pub type GitResult<T> = Result<T, GitError>;

impl error::Error for GitError {
    fn description(&self) -> &str {
        match *self {
            GitError::Message(msg) => msg,
            GitError::IoError(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            GitError::IoError(ref err) => Some(err as &error::Error),
            _ => None,
        }
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GitError::Message(msg) => msg.fmt(f),
            GitError::IoError(ref err) => err.fmt(f),
        }
    }
}

impl From<&'static str> for GitError {
    fn from(err: &'static str) -> GitError {
        GitError::Message(err)
    }
}

impl From<io::Error> for GitError {
    fn from(err: io::Error) -> GitError {
        GitError::IoError(err)
    }
}

impl From<string::FromUtf8Error> for GitError {
    fn from(_: string::FromUtf8Error) -> GitError {
        GitError::Message("Invalid UTF-8 data")
    }
}

impl From<num::ParseIntError> for GitError {
    fn from(_: num::ParseIntError) -> GitError {
        GitError::Message("Invalid integer")
    }
}
