use crate::git::error::*;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum InitializeDerivationError {
    PathAssertion(PathAssertionError),
    Serde(serde_json::Error),
    DerivationInProgress,
}
impl Error for InitializeDerivationError {}
impl Display for InitializeDerivationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serde(e) => e.fmt(f),
            Self::PathAssertion(e) => e.fmt(f),
            Self::DerivationInProgress => {
                f.write_str("fatal: a derivation is currently in progress")
            }
        }
    }
}
impl From<PathAssertionError> for InitializeDerivationError {
    fn from(value: PathAssertionError) -> Self {
        Self::PathAssertion(value)
    }
}
impl From<GitError> for InitializeDerivationError {
    fn from(value: GitError) -> Self {
        Self::PathAssertion(value.into())
    }
}
impl From<GitSerdeError> for InitializeDerivationError {
    fn from(value: GitSerdeError) -> Self {
        match value {
            GitSerdeError::Serde(e) => Self::Serde(e),
            GitSerdeError::Git(e) => Self::PathAssertion(e.into()),
        }
    }
}

#[derive(Debug)]
pub enum ContinueDerivationError {
    PathAssertion(PathAssertionError),
    Serde(serde_json::Error),
    NoDerivationInProgress,
}
impl Error for ContinueDerivationError {}
impl Display for ContinueDerivationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serde(e) => e.fmt(f),
            Self::PathAssertion(e) => e.fmt(f),
            Self::NoDerivationInProgress => f.write_str("fatal: no derivation in progress"),
        }
    }
}
impl From<PathAssertionError> for ContinueDerivationError {
    fn from(value: PathAssertionError) -> Self {
        Self::PathAssertion(value)
    }
}
impl From<GitSerdeError> for ContinueDerivationError {
    fn from(value: GitSerdeError) -> Self {
        match value {
            GitSerdeError::Serde(e) => Self::Serde(e),
            GitSerdeError::Git(e) => Self::PathAssertion(e.into()),
        }
    }
}
impl From<GitError> for ContinueDerivationError {
    fn from(value: GitError) -> Self {
        Self::PathAssertion(value.into())
    }
}

#[derive(Debug)]
pub enum AbortDerivationError {
    Git(GitError),
    NoDerivationInProgress,
}
impl Error for AbortDerivationError {}
impl From<GitError> for AbortDerivationError {
    fn from(value: GitError) -> Self {
        Self::Git(value)
    }
}
impl Display for AbortDerivationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Git(e) => e.fmt(f),
            Self::NoDerivationInProgress => f.write_str("fatal: no derivation in progress"),
        }
    }
}

#[derive(Debug)]
pub enum UpdateProductError {
    PathAssertion(PathAssertionError),
    Serde(serde_json::Error),
    DerivationInProgress,
}
impl Error for UpdateProductError {}
impl Display for UpdateProductError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serde(e) => e.fmt(f),
            Self::PathAssertion(e) => e.fmt(f),
            Self::DerivationInProgress => {
                f.write_str("fatal: a derivation is currently in progress")
            }
        }
    }
}
impl From<InitializeDerivationError> for UpdateProductError {
    fn from(value: InitializeDerivationError) -> Self {
        match value {
            InitializeDerivationError::PathAssertion(e) => Self::PathAssertion(e),
            InitializeDerivationError::Serde(e) => Self::Serde(e),
            InitializeDerivationError::DerivationInProgress => Self::DerivationInProgress,
        }
    }
}
impl From<PathAssertionError> for UpdateProductError {
    fn from(value: PathAssertionError) -> Self {
        Self::PathAssertion(value)
    }
}

#[derive(Debug)]
pub enum OptimizeMergeOrderError {
    PathAssertion(PathAssertionError),
    Serde(serde_json::Error),
}
impl Error for OptimizeMergeOrderError {}
impl Display for OptimizeMergeOrderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serde(e) => e.fmt(f),
            Self::PathAssertion(e) => e.fmt(f),
        }
    }
}
impl From<PathAssertionError> for OptimizeMergeOrderError {
    fn from(value: PathAssertionError) -> Self {
        Self::PathAssertion(value)
    }
}
impl From<GitSerdeError> for OptimizeMergeOrderError {
    fn from(value: GitSerdeError) -> Self {
        match value {
            GitSerdeError::Serde(e) => Self::Serde(e),
            GitSerdeError::Git(e) => Self::PathAssertion(e.into()),
        }
    }
}
