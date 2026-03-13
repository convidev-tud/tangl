use crate::model::*;
use itertools::Itertools;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct NodePath<T: SymbolicNodeType> {
    path: Vec<Rc<Node>>,
    _phantom: PhantomData<T>,
}

impl<T: HasFeatureChildren> NodePath<T> {
    pub fn move_to_feature(self, path: &QualifiedPath) -> Option<NodePath<Feature>> {
        self.move_to(path)?.try_convert_to()
    }
}

impl<T: HasProductChildren> NodePath<T> {
    pub fn move_to_product(self, path: &QualifiedPath) -> Option<NodePath<Product>> {
        self.move_to(path)?.try_convert_to()
    }
}

impl NodePath<AnyNode> {
    pub fn from_concrete<T: SymbolicNodeType>(other: &NodePath<T>) -> Self {
        Self::new(other.path.clone())
    }
}

impl NodePath<VirtualRoot> {
    pub fn to_area(self, area: &QualifiedPath) -> Option<NodePath<ConcreteArea>> {
        self.move_to(area)?.try_convert_to()
    }
}

impl NodePath<ConcreteArea> {
    pub fn get_path_to_feature_root(&self) -> QualifiedPath {
        self.to_qualified_path() + QualifiedPath::from(FEATURES_PREFIX)
    }
    pub fn get_path_to_product_root(&self) -> QualifiedPath {
        self.to_qualified_path() + QualifiedPath::from(PRODUCTS_PREFIX)
    }
    pub fn move_to_feature_root(self) -> Option<NodePath<FeatureRoot>> {
        self.move_to(&QualifiedPath::from(FEATURES_PREFIX))?
            .try_convert_to()
    }
    pub fn move_to_product_root(self) -> Option<NodePath<ProductRoot>> {
        self.move_to(&QualifiedPath::from(PRODUCTS_PREFIX))?
            .try_convert_to()
    }
}

impl NodePath<FeatureRoot> {
    pub fn iter_root_features(&self) -> impl Iterator<Item = NodePath<Feature>> {
        self.iter_children().map(|p| p.try_convert_to().unwrap())
    }
    pub fn iter_features_req(&self) -> impl Iterator<Item = NodePath<Feature>> {
        self.iter_children_req()
            .map(|p| p.try_convert_to().unwrap())
    }
}

impl<T: SymbolicNodeType> ToQualifiedPath for NodePath<T> {
    fn to_qualified_path(&self) -> QualifiedPath {
        let mut path = QualifiedPath::new();
        for p in self.path.iter() {
            path.push(p.get_name());
        }
        path
    }
}

impl<T: SymbolicNodeType> NodePath<T> {
    fn get_node(&self) -> &Node {
        self.path.last().unwrap()
    }
    pub fn new(path: Vec<Rc<Node>>) -> NodePath<T> {
        Self {
            path,
            _phantom: PhantomData,
        }
    }
    pub fn try_convert_to<To: SymbolicNodeType>(&self) -> Option<NodePath<To>> {
        if To::is_compatible(self.get_node().get_type()) {
            Some(NodePath::<To>::new(self.path.clone()))
        } else {
            None
        }
    }
    pub fn move_to(mut self, path: &QualifiedPath) -> Option<NodePath<AnyNode>> {
        for p in path.iter_string() {
            self.path.push(self.get_node().get_child(p)?.clone());
        }
        Some(NodePath::<AnyNode>::new(self.path))
    }

    pub fn move_to_last_valid(self, path: &QualifiedPath) -> NodePath<AnyNode> {
        let mut current = self.as_any_type();
        for part in path.iter() {
            let next = current.clone().move_to(&part);
            if next.is_some() {
                current = next.unwrap();
            } else {
                break;
            }
        }
        current
    }
    pub fn has_children(&self) -> bool {
        self.get_node().has_children()
    }
    pub fn iter_children(&self) -> impl Iterator<Item = NodePath<AnyNode>> {
        self.get_node()
            .iter_children()
            .map(|(name, _)| self.clone().move_to(&name.to_qualified_path()).unwrap())
            .sorted()
    }
    pub fn iter_children_req(&self) -> impl Iterator<Item = NodePath<AnyNode>> {
        self.iter_children().flat_map(|path| {
            let mut to_iter = Vec::new();
            to_iter.push(path.clone());
            to_iter.extend(path.iter_children_req());
            to_iter
        })
    }
    pub fn get_tags(&self) -> Vec<QualifiedPath> {
        self.get_node()
            .iter_children()
            .filter_map(|(name, child)| match child.get_type() {
                NodeType::Tag => Some(QualifiedPath::from(name.clone())),
                _ => None,
            })
            .collect()
    }
    pub fn get_metadata(&self) -> &NodeMetadata {
        self.get_node().get_metadata()
    }
    pub fn get_actual_type(&self) -> &NodeType {
        self.get_node().get_type()
    }
    pub fn as_any_type(&self) -> NodePath<AnyNode> {
        NodePath::<AnyNode>::from_concrete(self)
    }
    pub fn display_tree(&self, show_tags: bool) -> String {
        self.get_node().display_tree(show_tags)
    }
}

impl<A, B> PartialEq<NodePath<A>> for NodePath<B>
where
    A: SymbolicNodeType,
    B: SymbolicNodeType,
{
    fn eq(&self, other: &NodePath<A>) -> bool {
        self.to_qualified_path() == other.to_qualified_path()
    }
}

impl<T: SymbolicNodeType> Eq for NodePath<T> {}

impl<T: SymbolicNodeType> Display for NodePath<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_qualified_path().to_string().as_str())
    }
}

impl<T: SymbolicNodeType> PartialOrd for NodePath<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.to_qualified_path() == other.to_qualified_path() {
            Some(Ordering::Equal)
        } else if self.to_qualified_path() > other.to_qualified_path() {
            Some(Ordering::Greater)
        } else if self.to_qualified_path() < other.to_qualified_path() {
            Some(Ordering::Less)
        } else {
            None
        }
    }
}

impl<T: SymbolicNodeType> Ord for NodePath<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(&other).unwrap()
    }
}
