use crate::git::error::{GitCommandError, GitError, GitModelError, GitSerdeError};
use crate::model::{PathNotFoundError, WrongNodeTypeError};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io;

#[derive(Debug)]
pub enum InitializeDerivationError {
    Io(io::Error),
    Git(GitCommandError),
    Serde(serde_json::Error),
    DerivationInProgress,
}
impl From<GitError> for InitializeDerivationError {
    fn from(value: GitError) -> Self {
        match value {
            GitError::Io(e) => Self::Io(e),
            GitError::Git(e) => Self::Git(e),
        }
    }
}
impl From<GitSerdeError> for InitializeDerivationError {
    fn from(value: GitSerdeError) -> Self {
        match value {
            GitSerdeError::Serde(e) => Self::Serde(e),
            GitSerdeError::Io(e) => Self::Io(e),
            GitSerdeError::Git(e) => Self::Git(e),
        }
    }
}
impl Display for InitializeDerivationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => e.fmt(f),
            Self::Serde(e) => e.fmt(f),
            Self::Git(e) => e.fmt(f),
            Self::DerivationInProgress => {
                f.write_str("fatal: a derivation is currently in progress")
            }
        }
    }
}

impl Error for InitializeDerivationError {}

#[derive(Debug)]
pub enum ContinueDerivationError {
    Io(io::Error),
    Git(GitCommandError),
    WrongNodeType(WrongNodeTypeError),
    PathNotFound(PathNotFoundError),
    Serde(serde_json::Error),
    NoDerivationInProgress,
}
impl From<GitModelError> for ContinueDerivationError {
    fn from(value: GitModelError) -> Self {
        match value {
            GitModelError::Io(e) => Self::Io(e),
            GitModelError::Git(e) => Self::Git(e),
            GitModelError::WrongNodeType(e) => Self::WrongNodeType(e),
            GitModelError::PathNotFound(e) => Self::PathNotFound(e),
        }
    }
}
impl From<GitSerdeError> for ContinueDerivationError {
    fn from(value: GitSerdeError) -> Self {
        match value {
            GitSerdeError::Serde(e) => Self::Serde(e),
            GitSerdeError::Io(e) => Self::Io(e),
            GitSerdeError::Git(e) => Self::Git(e),
        }
    }
}
impl Display for ContinueDerivationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => e.fmt(f),
            Self::Git(e) => e.fmt(f),
            Self::Serde(e) => e.fmt(f),
            Self::WrongNodeType(e) => e.fmt(f),
            Self::PathNotFound(e) => e.fmt(f),
            Self::NoDerivationInProgress => f.write_str("fatal: no derivation in progress"),
        }
    }
}

impl Error for ContinueDerivationError {}
