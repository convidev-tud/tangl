use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub struct WrongNodeTypeError {
    msg: String,
}
impl WrongNodeTypeError {
    pub fn new<S: Into<String>>(msg: S) -> Self {
        Self { msg: msg.into() }
    }
}
impl Display for WrongNodeTypeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
impl Error for WrongNodeTypeError {}

#[derive(Debug, Clone)]
pub struct PathNotFoundError {
    msg: String,
}
impl PathNotFoundError {
    pub fn new<S: Into<String>>(msg: S) -> Self {
        Self { msg: msg.into() }
    }
}
impl Display for PathNotFoundError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
impl Error for PathNotFoundError {}

#[derive(Debug, Clone)]
pub enum ModelError {
    WrongNodeType(WrongNodeTypeError),
    PathNotFound(PathNotFoundError),
}
impl Display for ModelError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelError::WrongNodeType(err) => err.fmt(f),
            ModelError::PathNotFound(err) => err.fmt(f),
        }
    }
}
impl Error for ModelError {}
impl From<WrongNodeTypeError> for ModelError {
    fn from(err: WrongNodeTypeError) -> Self {
        ModelError::WrongNodeType(err)
    }
}
impl From<PathNotFoundError> for ModelError {
    fn from(err: PathNotFoundError) -> Self {
        ModelError::PathNotFound(err)
    }
}
