use crate::model::{ModelError, PathNotFoundError, WrongNodeTypeError};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io;

#[derive(Debug, Clone)]
pub struct GitCommandError {
    msg: String,
}
impl GitCommandError {
    pub fn new<S: Into<String>>(msg: S) -> GitCommandError {
        GitCommandError { msg: msg.into() }
    }
}
impl Display for GitCommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
impl Error for GitCommandError {}

#[derive(Debug, Clone)]
pub struct InvalidVersionError {
    msg: String,
}
impl InvalidVersionError {
    pub fn new<S: Into<String>>(msg: S) -> GitCommandError {
        GitCommandError { msg: msg.into() }
    }
}
impl Display for InvalidVersionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
impl Error for InvalidVersionError {}

#[derive(Debug)]
pub enum GitError {
    Io(io::Error),
    Git(GitCommandError),
}
impl Display for GitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::Io(err) => err.fmt(f),
            GitError::Git(err) => err.fmt(f),
        }
    }
}
impl Error for GitError {}
impl From<io::Error> for GitError {
    fn from(err: io::Error) -> GitError {
        GitError::Io(err)
    }
}
impl From<GitCommandError> for GitError {
    fn from(value: GitCommandError) -> Self {
        GitError::Git(value)
    }
}

#[derive(Debug)]
pub enum GitWrongNodeTypeError {
    Io(io::Error),
    Git(GitCommandError),
    WrongNodeType(WrongNodeTypeError),
}
impl Display for GitWrongNodeTypeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Git(err) => err.fmt(f),
            Self::WrongNodeType(err) => err.fmt(f),
            Self::Io(err) => err.fmt(f),
        }
    }
}
impl Error for GitWrongNodeTypeError {}
impl From<GitCommandError> for GitWrongNodeTypeError {
    fn from(err: GitCommandError) -> Self {
        Self::Git(err)
    }
}
impl From<WrongNodeTypeError> for GitWrongNodeTypeError {
    fn from(err: WrongNodeTypeError) -> Self {
        Self::WrongNodeType(err)
    }
}
impl From<io::Error> for GitWrongNodeTypeError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}
impl From<GitError> for GitWrongNodeTypeError {
    fn from(err: GitError) -> Self {
        match err {
            GitError::Io(err) => Self::Io(err),
            GitError::Git(err) => Self::Git(err),
        }
    }
}

#[derive(Debug)]
pub enum GitModelError {
    Io(io::Error),
    Git(GitCommandError),
    WrongNodeType(WrongNodeTypeError),
    PathNotFound(PathNotFoundError),
}
impl Display for GitModelError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Git(err) => err.fmt(f),
            Self::Io(err) => err.fmt(f),
            Self::WrongNodeType(err) => err.fmt(f),
            Self::PathNotFound(err) => err.fmt(f),
        }
    }
}
impl Error for GitModelError {}
impl From<GitCommandError> for GitModelError {
    fn from(err: GitCommandError) -> Self {
        Self::Git(err)
    }
}
impl From<WrongNodeTypeError> for GitModelError {
    fn from(err: WrongNodeTypeError) -> Self {
        Self::WrongNodeType(err)
    }
}
impl From<PathNotFoundError> for GitModelError {
    fn from(err: PathNotFoundError) -> Self {
        Self::PathNotFound(err)
    }
}
impl From<ModelError> for GitModelError {
    fn from(err: ModelError) -> Self {
        match err {
            ModelError::PathNotFound(err) => Self::PathNotFound(err),
            ModelError::WrongNodeType(err) => Self::WrongNodeType(err),
        }
    }
}
impl From<io::Error> for GitModelError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}
impl From<GitError> for GitModelError {
    fn from(err: GitError) -> Self {
        match err {
            GitError::Io(err) => Self::Io(err),
            GitError::Git(err) => Self::Git(err),
        }
    }
}

#[derive(Debug)]
pub enum GitSerdeError {
    Io(io::Error),
    Git(GitCommandError),
    Serde(serde_json::Error),
}
impl Display for GitSerdeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Git(err) => err.fmt(f),
            Self::Serde(err) => err.fmt(f),
            Self::Io(err) => err.fmt(f),
        }
    }
}
impl Error for GitSerdeError {}
impl From<GitCommandError> for GitSerdeError {
    fn from(err: GitCommandError) -> Self {
        Self::Git(err)
    }
}
impl From<serde_json::Error> for GitSerdeError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serde(err)
    }
}
impl From<io::Error> for GitSerdeError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}
impl From<GitError> for GitSerdeError {
    fn from(err: GitError) -> Self {
        match err {
            GitError::Io(err) => Self::Io(err),
            GitError::Git(err) => Self::Git(err),
        }
    }
}

#[derive(Debug)]
pub enum InvalidPathError {
    Io(io::Error),
    Git(GitCommandError),
    PathNotFound(PathNotFoundError),
    WrongNodeType(WrongNodeTypeError),
    InvalidVersion(InvalidVersionError)
}
impl Display for InvalidPathError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Git(err) => err.fmt(f),
            Self::Io(err) => err.fmt(f),
            Self::PathNotFound(err) => err.fmt(f),
            Self::WrongNodeType(err) => err.fmt(f),
            Self::InvalidVersion(err) => err.fmt(f),
        }
    }
}
impl Error for InvalidPathError {}
impl From<GitError> for InvalidPathError {
    fn from(err: GitError) -> Self {
        match err {
            GitError::Io(err) => Self::Io(err),
            GitError::Git(err) => Self::Git(err),
        }
    }
}
impl From<ModelError> for InvalidPathError {
    fn from(err: ModelError) -> Self {
        match err {
            ModelError::PathNotFound(err) => Self::PathNotFound(err),
            ModelError::WrongNodeType(err) => Self::WrongNodeType(err),
        }
    }
}
impl From<InvalidVersionError> for InvalidPathError {
    fn from(value: InvalidVersionError) -> Self {
        Self::InvalidVersion(value)
    }
}
