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

pub trait HasFeatureChildren: SymbolicNodeType {}
pub trait HasProductChildren: SymbolicNodeType {}
pub trait HasBranch: SymbolicNodeType {}

#[derive(Clone, Debug)]
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
impl HasBranch for ConcreteFeature {}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct Feature;
impl SymbolicNodeType for Feature {
    fn identifier() -> String {
        NodeType::AbstractFeature.get_type_name()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::AbstractFeature | NodeType::ConcreteFeature => true,
            _ => false,
        }
    }
}
impl HasFeatureChildren for Feature {}

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
impl HasFeatureChildren for FeatureRoot {}

#[derive(Clone, Debug)]
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
impl HasBranch for ConcreteProduct {}

#[derive(Clone, Debug)]
pub struct AbstractProduct;
impl SymbolicNodeType for AbstractProduct {
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
impl HasProductChildren for AbstractProduct {}

#[derive(Clone, Debug)]
pub struct Product;
impl SymbolicNodeType for Product {
    fn identifier() -> String {
        NodeType::ConcreteProduct.get_type_name()
    }

    fn is_compatible_to_node_type(node_type: &NodeType) -> bool {
        match node_type {
            NodeType::ConcreteProduct | NodeType::AbstractProduct => true,
            _ => false,
        }
    }
}
impl HasProductChildren for Product {}

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
impl HasProductChildren for ProductRoot {}

#[derive(Clone, Debug)]
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
impl HasBranch for ConcreteArea {}

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
pub struct ConcreteBranch;
impl SymbolicNodeType for ConcreteBranch {
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
impl HasBranch for ConcreteBranch {}

#[derive(Clone, Debug)]
pub enum NodeType {
    ConcreteFeature,
    AbstractFeature,
    ConcreteProduct,
    AbstractProduct,
    FeatureRoot,
    ProductRoot,
    ConcreteArea,
    VirtualRoot,
    Tag,
    Unknown,
}

impl NodeType {
    pub fn decide_next_type(
        &self,
        name: &str,
        metadata: &NodeMetadata,
    ) -> Result<NodeType, WrongNodeTypeError> {
        match self {
            Self::ConcreteFeature | Self::AbstractFeature | Self::FeatureRoot => {
                if metadata.has_branch() {
                    Ok(Self::ConcreteFeature)
                } else {
                    Ok(Self::AbstractFeature)
                }
            }
            Self::ConcreteProduct | Self::AbstractProduct | Self::ProductRoot => {
                if metadata.has_branch() {
                    Ok(Self::ConcreteProduct)
                } else {
                    Ok(Self::AbstractProduct)
                }
            }
            Self::VirtualRoot => Ok(Self::ConcreteArea),
            Self::ConcreteArea => {
                if name.starts_with(FEATURES_PREFIX) {
                    Ok(Self::FeatureRoot)
                } else if name.starts_with(PRODUCTS_PREFIX) {
                    Ok(Self::ProductRoot)
                } else {
                    Ok(Self::Unknown)
                }
            }
            Self::Tag => Err(WrongNodeTypeError::new("Tags cannot have children")),
            Self::Unknown => Ok(Self::Unknown),
        }
    }

    pub fn format_node_display(&self, name: ColoredString) -> ColoredString {
        match self {
            Self::ConcreteArea => name.yellow().bold(),
            Self::FeatureRoot => name.bright_purple().bold().italic(),
            Self::ConcreteFeature => name.purple(),
            Self::ProductRoot => name.truecolor(231, 100, 18).bold().italic(),
            Self::ConcreteProduct => name.truecolor(231, 100, 18),
            Self::Tag => name.green(),
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
            Self::Tag => "tag",
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
            Self::Tag => "t",
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
