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
impl From<DerivationCommitError> for InitializeDerivationError {
    fn from(value: DerivationCommitError) -> Self {
        match value {
            DerivationCommitError::PathAssertion(e) => Self::PathAssertion(e),
            DerivationCommitError::Serde(e) => Self::Serde(e),
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
impl From<GitError> for ContinueDerivationError {
    fn from(value: GitError) -> Self {
        Self::PathAssertion(value.into())
    }
}
impl From<DerivationCommitError> for ContinueDerivationError {
    fn from(value: DerivationCommitError) -> Self {
        match value {
            DerivationCommitError::PathAssertion(e) => Self::PathAssertion(e),
            DerivationCommitError::Serde(e) => Self::Serde(e),
        }
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
pub enum ResetDerivationError {
    Git(GitError),
    NoDerivationInProgress,
}
impl Error for ResetDerivationError {}
impl From<GitError> for ResetDerivationError {
    fn from(value: GitError) -> Self {
        Self::Git(value)
    }
}
impl Display for ResetDerivationError {
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
impl From<GitError> for OptimizeMergeOrderError {
    fn from(value: GitError) -> Self {
        Self::PathAssertion(value.into())
    }
}
impl From<DerivationCommitError> for OptimizeMergeOrderError {
    fn from(value: DerivationCommitError) -> Self {
        match value {
            DerivationCommitError::PathAssertion(e) => Self::PathAssertion(e),
            DerivationCommitError::Serde(e) => Self::Serde(e),
        }
    }
}

#[derive(Debug)]
pub enum DerivationCommitError {
    PathAssertion(PathAssertionError),
    Serde(serde_json::Error),
}
impl Error for DerivationCommitError {}
impl Display for DerivationCommitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serde(e) => e.fmt(f),
            Self::PathAssertion(e) => e.fmt(f),
        }
    }
}
impl From<PathAssertionError> for DerivationCommitError {
    fn from(value: PathAssertionError) -> Self {
        Self::PathAssertion(value)
    }
}
impl From<serde_json::Error> for DerivationCommitError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}
