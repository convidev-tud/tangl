use crate::model::*;
use colored::{ColoredString, Colorize};
use std::fmt::Debug;

pub trait SymbolicNodeType: Clone + Debug {
    fn identifier() -> String;
    fn is_compatible(node_type: &NodeType) -> bool {
        Self::is_compatible_to_node_type(node_type)
    }
    fn is_compatible_to_node_type(node_type: &NodeType) -> bool;
}
pub trait CanHaveBranch: SymbolicNodeType {}

#[derive(Clone, Debug)]
pub struct Feature;
impl SymbolicNodeType for Feature {
    fn identifier() -> String {
        NodeType::Feature.get_type_name()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::Feature => true,
            _ => false,
        }
    }
}
impl CanHaveBranch for Feature {}

#[derive(Clone, Debug)]
pub struct FeatureRoot;
impl SymbolicNodeType for FeatureRoot {
    fn identifier() -> String {
        NodeType::FeatureRoot.get_type_name()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::FeatureRoot => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Product;
impl SymbolicNodeType for Product {
    fn identifier() -> String {
        NodeType::Product.get_type_name()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::Product => true,
            _ => false,
        }
    }
}
impl CanHaveBranch for Product {}

#[derive(Clone, Debug)]
pub struct ProductRoot;
impl SymbolicNodeType for ProductRoot {
    fn identifier() -> String {
        NodeType::ProductRoot.get_type_name()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::ProductRoot => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Area;
impl SymbolicNodeType for Area {
    fn identifier() -> String {
        NodeType::Area.get_type_name()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::Area => true,
            _ => false,
        }
    }
}
impl CanHaveBranch for Area {}

#[derive(Clone, Debug)]
pub struct VirtualRoot;
impl SymbolicNodeType for VirtualRoot {
    fn identifier() -> String {
        NodeType::VirtualRoot.get_type_name()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::VirtualRoot => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Tag;
impl SymbolicNodeType for Tag {
    fn identifier() -> String {
        NodeType::Tag.get_type_name()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::Tag => true,
            _ => false,
        }
    }
}
impl CanHaveBranch for Tag {}

#[derive(Clone, Debug)]
pub struct AnyNode;
impl SymbolicNodeType for AnyNode {
    fn identifier() -> String {
        "any".to_string()
    }

    fn is_compatible_to_node_type(_node_type: &NodeType) -> bool {
        true
    }
}

#[derive(Clone, Debug)]
pub struct BranchAble;
impl SymbolicNodeType for BranchAble {
    fn identifier() -> String {
        "branch able".to_string()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::Feature | NodeType::Product | NodeType::Area => true,
            _ => false,
        }
    }
}
impl CanHaveBranch for BranchAble {}

#[derive(Clone, Debug)]
pub enum NodeType {
    Feature,
    Product,
    FeatureRoot,
    ProductRoot,
    Area,
    VirtualRoot,
    Tag,
}

impl NodeType {
    pub fn build_child_from_name(&mut self, name: &str) -> Result<NodeType, WrongNodeTypeError> {
        match self {
            Self::Feature => Ok(Self::Feature),
            Self::Product => Ok(Self::Product),
            Self::FeatureRoot => Ok(Self::Feature),
            Self::ProductRoot => Ok(Self::Product),
            Self::VirtualRoot => Ok(Self::Area),
            Self::Area => {
                if name.starts_with(FEATURES_PREFIX) {
                    Ok(Self::FeatureRoot)
                } else if name.starts_with(PRODUCTS_PREFIX) {
                    Ok(Self::ProductRoot)
                } else {
                    Err(WrongNodeTypeError::new(format!(
                        "'{}' is no valid child of an area node. Valid childs include: feature, product",
                        name
                    )))
                }
            }
            Self::Tag => Err(WrongNodeTypeError::new("Tags cannot have children")),
        }
    }

    pub fn format_node_display(&self, name: ColoredString) -> ColoredString {
        match self {
            Self::Area => name.yellow().bold(),
            Self::FeatureRoot => name.bright_purple().bold().underline(),
            Self::Feature => name.bright_purple(),
            Self::ProductRoot => name.red().bold().italic(),
            Self::Product => name.red(),
            Self::Tag => name.green(),
            _ => name,
        }
    }

    pub fn get_type_name(&self) -> String {
        let name: &str = match self {
            Self::VirtualRoot => "virtual root",
            Self::Area => "area",
            Self::FeatureRoot => "feature root",
            Self::ProductRoot => "product root",
            Self::Feature => "feature",
            Self::Product => "product",
            Self::Tag => "tag",
        };
        name.to_string()
    }

    pub fn get_formatted_name(&self) -> String {
        self.format_node_display(self.get_type_name().normal())
            .to_string()
    }
}
