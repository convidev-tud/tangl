use crate::model::*;
use colored::{ColoredString, Colorize};
use std::fmt::Debug;
use std::hash::Hash;

pub trait SymbolicNodeType: Clone + Debug + Eq + PartialEq + Hash {
    fn identifier() -> String;
    fn is_compatible(node_type: &NodeType) -> bool {
        Self::is_compatible_to_node_type(node_type)
    }
    fn is_compatible_to_node_type(node_type: &NodeType) -> bool;
}

pub trait HasFeatureChildren: SymbolicNodeType {}
pub trait HasProductChildren: SymbolicNodeType {}
pub trait IsGitObject: SymbolicNodeType {}
pub trait IsOnOrUnderArea: SymbolicNodeType {}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ConcreteFeature;
impl SymbolicNodeType for ConcreteFeature {
    fn identifier() -> String {
        NodeType::ConcreteFeature.get_type_name()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::ConcreteFeature => true,
            _ => false,
        }
    }
}
impl HasFeatureChildren for ConcreteFeature {}
impl IsGitObject for ConcreteFeature {}
impl IsOnOrUnderArea for ConcreteFeature {}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct AbstractFeature;
impl SymbolicNodeType for AbstractFeature {
    fn identifier() -> String {
        NodeType::AbstractFeature.get_type_name()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::AbstractFeature => true,
            _ => false,
        }
    }
}
impl HasFeatureChildren for AbstractFeature {}
impl IsOnOrUnderArea for AbstractFeature {}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Feature;
impl SymbolicNodeType for Feature {
    fn identifier() -> String {
        "generic feature".to_string()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::AbstractFeature | NodeType::ConcreteFeature => true,
            _ => false,
        }
    }
}
impl HasFeatureChildren for Feature {}
impl IsOnOrUnderArea for Feature {}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
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
impl HasFeatureChildren for FeatureRoot {}
impl IsOnOrUnderArea for FeatureRoot {}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ConcreteProduct;
impl SymbolicNodeType for ConcreteProduct {
    fn identifier() -> String {
        NodeType::ConcreteProduct.get_type_name()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::ConcreteProduct => true,
            _ => false,
        }
    }
}
impl HasProductChildren for ConcreteProduct {}
impl IsGitObject for ConcreteProduct {}
impl IsOnOrUnderArea for ConcreteProduct {}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct AbstractProduct;
impl SymbolicNodeType for AbstractProduct {
    fn identifier() -> String {
        NodeType::AbstractProduct.get_type_name()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::ConcreteProduct => true,
            _ => false,
        }
    }
}
impl HasProductChildren for AbstractProduct {}
impl IsOnOrUnderArea for AbstractProduct {}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Product;
impl SymbolicNodeType for Product {
    fn identifier() -> String {
        "generic product".to_string()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::ConcreteProduct | NodeType::AbstractProduct => true,
            _ => false,
        }
    }
}
impl HasProductChildren for Product {}
impl IsOnOrUnderArea for Product {}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
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
impl HasProductChildren for ProductRoot {}
impl IsOnOrUnderArea for ProductRoot {}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ConcreteArea;
impl SymbolicNodeType for ConcreteArea {
    fn identifier() -> String {
        NodeType::ConcreteArea.get_type_name()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::ConcreteArea => true,
            _ => false,
        }
    }
}
impl IsGitObject for ConcreteArea {}
impl IsOnOrUnderArea for ConcreteArea {}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
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

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct AnyNode;
impl SymbolicNodeType for AnyNode {
    fn identifier() -> String {
        "any".to_string()
    }

    fn is_compatible_to_node_type(_node_type: &NodeType) -> bool {
        true
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct AnyGitObject;
impl SymbolicNodeType for AnyGitObject {
    fn identifier() -> String {
        "branch able".to_string()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::ConcreteFeature | NodeType::ConcreteProduct | NodeType::ConcreteArea => true,
            _ => false,
        }
    }
}
impl IsGitObject for AnyGitObject {}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum NodeType {
    ConcreteFeature,
    AbstractFeature,
    ConcreteProduct,
    AbstractProduct,
    FeatureRoot,
    ProductRoot,
    ConcreteArea,
    VirtualRoot,
    Unknown,
}

impl NodeType {
    pub fn decide_next_type(&self, name: &str, metadata: &BranchData) -> NodeType {
        match self {
            Self::ConcreteFeature | Self::AbstractFeature | Self::FeatureRoot => {
                if metadata.has_branch() {
                    Self::ConcreteFeature
                } else {
                    Self::AbstractFeature
                }
            }
            Self::ConcreteProduct | Self::AbstractProduct | Self::ProductRoot => {
                if metadata.has_branch() {
                    Self::ConcreteProduct
                } else {
                    Self::AbstractProduct
                }
            }
            Self::VirtualRoot => Self::ConcreteArea,
            Self::ConcreteArea => {
                if name.starts_with(FEATURES_PREFIX) {
                    Self::FeatureRoot
                } else if name.starts_with(PRODUCTS_PREFIX) {
                    Self::ProductRoot
                } else {
                    Self::Unknown
                }
            }
            Self::Unknown => Self::Unknown,
        }
    }

    pub fn format_node_display(&self, name: ColoredString) -> ColoredString {
        match self {
            Self::ConcreteArea => name.yellow().bold(),
            Self::FeatureRoot => name.bright_purple().bold().italic(),
            Self::ConcreteFeature => name.purple(),
            Self::ProductRoot => name.truecolor(231, 100, 18).bold().italic(),
            Self::ConcreteProduct => name.truecolor(231, 100, 18),
            _ => name,
        }
    }

    pub fn get_type_name(&self) -> String {
        let name: &str = match self {
            Self::VirtualRoot => "virtual root",
            Self::ConcreteArea => "area",
            Self::FeatureRoot => "feature root",
            Self::ProductRoot => "product root",
            Self::ConcreteFeature => "feature",
            Self::AbstractFeature => "abstract feature",
            Self::ConcreteProduct => "product",
            Self::AbstractProduct => "abstract product",
            Self::Unknown => "",
        };
        name.to_string()
    }

    pub fn get_short_type_name(&self) -> String {
        let name: &str = match self {
            Self::VirtualRoot => "vr",
            Self::ConcreteArea => "a",
            Self::FeatureRoot => "fr",
            Self::ProductRoot => "pr",
            Self::ConcreteFeature => "f",
            Self::AbstractFeature => "f'",
            Self::ConcreteProduct => "p",
            Self::AbstractProduct => "p'",
            Self::Unknown => "",
        };
        name.to_string()
    }

    pub fn get_formatted_name(&self) -> String {
        self.format_node_display(self.get_type_name().normal())
            .to_string()
    }

    pub fn get_formatted_short_name(&self) -> String {
        self.format_node_display(self.get_short_type_name().normal())
            .to_string()
    }
}
