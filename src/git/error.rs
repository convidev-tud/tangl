use crate::model::{ModelError, PathNotFoundError, WrongNodeTypeError};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io;

#[derive(Debug, Clone)]
pub struct GitCommandError {
    git_output: String,
    msg: String,
}
impl GitCommandError {
    pub fn new<S1: Into<String>, S2: Into<String>>(git_output: S1, msg: S2) -> GitCommandError {
        GitCommandError { git_output: git_output.into(), msg: msg.into() }
    }
    pub fn get_git_output(&self) -> &String {
        &self.git_output
    }
}
impl Display for GitCommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\n{}", self.msg, self.git_output)
    }
}
impl Error for GitCommandError {}

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

#[derive(Debug, Clone)]
pub struct InvalidVersionError {
    msg: String,
}
impl InvalidVersionError {
    pub fn new<S: Into<String>>(msg: S) -> InvalidVersionError {
        InvalidVersionError { msg: msg.into() }
    }
}
impl Display for InvalidVersionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
impl Error for InvalidVersionError {}

#[derive(Debug)]
pub enum InvalidPathError {
    PathNotFound(PathNotFoundError),
    WrongNodeType(WrongNodeTypeError),
    InvalidVersion(InvalidVersionError),
}
impl Display for InvalidPathError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PathNotFound(err) => err.fmt(f),
            Self::WrongNodeType(err) => err.fmt(f),
            Self::InvalidVersion(err) => err.fmt(f),
        }
    }
}
impl Error for InvalidPathError {}
impl From<ModelError> for InvalidPathError {
    fn from(err: ModelError) -> Self {
        match err {
            ModelError::PathNotFound(err) => Self::PathNotFound(err),
            ModelError::WrongNodeType(err) => Self::WrongNodeType(err),
        }
    }
}
impl From<PathNotFoundError> for InvalidPathError {
    fn from(value: PathNotFoundError) -> Self {
        Self::PathNotFound(value)
    }
}
impl From<WrongNodeTypeError> for InvalidPathError {
    fn from(value: WrongNodeTypeError) -> Self {
        Self::WrongNodeType(value)
    }
}
impl From<InvalidVersionError> for InvalidPathError {
    fn from(value: InvalidVersionError) -> Self {
        Self::InvalidVersion(value)
    }
}

#[derive(Debug)]
pub enum PathAssertionError {
    Git(GitError),
    InvalidPath(InvalidPathError),
}
impl Error for PathAssertionError {}
impl Display for PathAssertionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Git(err) => err.fmt(f),
            Self::InvalidPath(err) => err.fmt(f),
        }
    }
}
impl From<GitError> for PathAssertionError {
    fn from(err: GitError) -> Self {
        Self::Git(err)
    }
}
impl From<GitCommandError> for PathAssertionError {
    fn from(err: GitCommandError) -> Self {
        Self::Git(err.into())
    }
}
impl From<ModelError> for PathAssertionError {
    fn from(err: ModelError) -> Self {
        match err {
            ModelError::PathNotFound(err) => Self::InvalidPath(err.into()),
            ModelError::WrongNodeType(err) => Self::InvalidPath(err.into()),
        }
    }
}
impl From<InvalidVersionError> for PathAssertionError {
    fn from(value: InvalidVersionError) -> Self {
        Self::InvalidPath(value.into())
    }
}
impl From<PathNotFoundError> for PathAssertionError {
    fn from(value: PathNotFoundError) -> Self {
        Self::InvalidPath(value.into())
    }
}
impl From<WrongNodeTypeError> for PathAssertionError {
    fn from(value: WrongNodeTypeError) -> Self {
        Self::InvalidPath(value.into())
    }
}
impl From<io::Error> for PathAssertionError {
    fn from(value: io::Error) -> Self {
        Self::Git(value.into())
    }
}
