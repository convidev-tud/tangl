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
pub struct NodeNotFoundError {
    msg: String,
}
impl NodeNotFoundError {
    pub fn new<S: Into<String>>(msg: S) -> Self {
        Self { msg: msg.into() }
    }
}
impl Display for NodeNotFoundError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
impl Error for NodeNotFoundError {}

#[derive(Debug, Clone)]
pub enum NodeError {
    WrongNodeType(WrongNodeTypeError),
    NodeNotFound(NodeNotFoundError),
}
impl Display for NodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeError::WrongNodeType(err) => err.fmt(f),
            NodeError::NodeNotFound(err) => err.fmt(f),
        }
    }
}
impl Error for NodeError {}
impl From<WrongNodeTypeError> for NodeError {
    fn from(err: WrongNodeTypeError) -> Self {
        NodeError::WrongNodeType(err)
    }
}
impl From<NodeNotFoundError> for NodeError {
    fn from(err: NodeNotFoundError) -> Self {
        NodeError::NodeNotFound(err)
    }
}
