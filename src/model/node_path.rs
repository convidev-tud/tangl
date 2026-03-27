use crate::model::*;
use colored::Colorize;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Clone, Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum PointsTo {
    Head,
    Commit(CommitHash),
    Tag(String),
}

impl PointsTo {
    fn formatted(&self, colored: bool) -> String {
        let info = if colored {
            match self {
                Self::Head => "Head".yellow(),
                Self::Commit(c) => c.get_short_hash().yellow(),
                Self::Tag(taq) => taq.green(),
            }
        } else {
            match self {
                Self::Head => "Head".normal(),
                Self::Commit(c) => c.get_short_hash().normal(),
                Self::Tag(tag) => tag.normal(),
            }
        };
        format!(" ({info})")
    }
}

#[derive(Clone, Debug)]
pub struct NodePath<T: SymbolicNodeType> {
    path: Vec<Rc<Node>>,
    unknown_mode: bool,
    points_to: PointsTo,
    _phantom: PhantomData<T>,
}

impl<T: HasFeatureChildren> NodePath<T> {
    pub fn move_to_feature(self, path: &NormalizedPath) -> Option<NodePath<Feature>> {
        self.move_to(path)?.try_convert_to()
    }
    pub fn iter_features(&self) -> impl Iterator<Item = NodePath<Feature>> {
        self.iter_children().map(|p| p.try_convert_to().unwrap())
    }
    pub fn iter_features_req(&self) -> impl Iterator<Item = NodePath<Feature>> {
        self.iter_children_req()
            .map(|p| p.try_convert_to().unwrap())
    }
}

impl<T: HasProductChildren> NodePath<T> {
    pub fn move_to_product(self, path: &NormalizedPath) -> Option<NodePath<Product>> {
        self.move_to(path)?.try_convert_to()
    }
    pub fn iter_products(&self) -> impl Iterator<Item = NodePath<Product>> {
        self.iter_children().map(|p| p.try_convert_to().unwrap())
    }
    pub fn iter_products_req(&self) -> impl Iterator<Item = NodePath<Product>> {
        self.iter_children_req()
            .map(|p| p.try_convert_to().unwrap())
    }
}

impl<T: IsOnOrUnderArea> NodePath<T> {
    pub fn move_to_area(self) -> NodePath<ConcreteArea> {
        self.move_to_index(1).try_convert_to().unwrap()
    }
}

impl<T: IsGitObject> NodePath<T> {
    pub fn get_ref_name(&self) -> &String {
        self.get_node().get_branch_data().get_branch().unwrap()
    }
    pub fn get_raw_object(&self) -> &String {
        match &self.points_to {
            PointsTo::Head => self.get_metadata().get_head().unwrap().get_full_hash(),
            PointsTo::Commit(hash) => hash.get_full_hash(),
            PointsTo::Tag(tag) => tag,
        }
    }
    pub fn get_head(&self) -> &PointsTo {
        &self.points_to
    }
    pub fn update_head(&mut self, head: PointsTo) {
        self.points_to = head;
    }
}

impl NodePath<AnyNode> {
    pub fn from_concrete<T: SymbolicNodeType>(other: &NodePath<T>) -> Self {
        Self::new(
            other.path.clone(),
            other.unknown_mode,
            other.points_to.clone(),
        )
    }
}

impl NodePath<VirtualRoot> {
    pub fn move_to_area(self, area: &NormalizedPath) -> Option<NodePath<ConcreteArea>> {
        self.move_to(area)?.try_convert_to()
    }
}

impl NodePath<ConcreteArea> {
    pub fn get_path_to_feature_root(&self) -> NormalizedPath {
        self.to_normalized_path() + NormalizedPath::from(FEATURES_PREFIX)
    }
    pub fn get_path_to_product_root(&self) -> NormalizedPath {
        self.to_normalized_path() + NormalizedPath::from(PRODUCTS_PREFIX)
    }
    pub fn move_to_feature_root(self) -> Option<NodePath<FeatureRoot>> {
        if self.unknown_mode {
            Some(NodePath::<FeatureRoot>::new(
                vec![self.path[0].clone()],
                self.unknown_mode,
                PointsTo::Head,
            ))
        } else {
            self.move_to(&NormalizedPath::from(FEATURES_PREFIX))?
                .try_convert_to()
        }
    }
    pub fn move_to_product_root(self) -> Option<NodePath<ProductRoot>> {
        if self.unknown_mode {
            Some(NodePath::<ProductRoot>::new(
                vec![self.path[0].clone()],
                self.unknown_mode,
                PointsTo::Head,
            ))
        } else {
            self.move_to(&NormalizedPath::from(PRODUCTS_PREFIX))?
                .try_convert_to()
        }
    }
}

impl<T: SymbolicNodeType> ToNormalizedPath for NodePath<T> {
    fn to_normalized_path(&self) -> NormalizedPath {
        let mut path = NormalizedPath::new();
        for p in self.path.iter() {
            path.push(p.get_name());
        }
        match &self.points_to {
            PointsTo::Head => path.set_version_appendix::<String>(None),
            PointsTo::Commit(hash) => path.set_version_appendix(Some(hash.get_short_hash())),
            PointsTo::Tag(tag) => path.set_version_appendix(Some(tag)),
        }
        path
    }
}

impl<T: SymbolicNodeType> Hash for NodePath<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_normalized_path().hash(state);
    }
}

impl<T: SymbolicNodeType> NodePath<T> {
    fn get_node(&self) -> &Node {
        self.path.last().unwrap()
    }
    pub fn new(path: Vec<Rc<Node>>, unknown_mode: bool, head: PointsTo) -> NodePath<T> {
        Self {
            path,
            unknown_mode,
            points_to: head,
            _phantom: PhantomData,
        }
    }
    pub fn try_convert_to<To: SymbolicNodeType>(&self) -> Option<NodePath<To>> {
        let compatible = self.unknown_mode || To::is_compatible(self.get_node().get_type());
        if compatible {
            Some(NodePath::<To>::new(
                self.path.clone(),
                self.unknown_mode,
                self.points_to.clone(),
            ))
        } else {
            None
        }
    }
    pub fn move_to(mut self, path: &NormalizedPath) -> Option<NodePath<AnyNode>> {
        for p in path.iter_string() {
            self.path.push(self.get_node().get_child(p)?.clone());
        }
        let head = match path.get_version_appendix() {
            Some(version) => {
                if self.has_tag(version) {
                    PointsTo::Tag(version.clone())
                } else {
                    PointsTo::Commit(CommitHash::new(version.clone()))
                }
            }
            None => PointsTo::Head,
        };
        Some(NodePath::<AnyNode>::new(self.path, self.unknown_mode, head))
    }
    pub fn move_to_index(self, index: usize) -> NodePath<AnyNode> {
        let path = self.path[0..index + 1].to_vec();
        NodePath::<AnyNode>::new(path, self.unknown_mode, PointsTo::Head)
    }
    pub fn move_to_last_valid(self, path: &NormalizedPath) -> NodePath<AnyNode> {
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
            .map(|(name, _)| self.clone().move_to(&name.to_normalized_path()).unwrap())
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
    pub fn get_tags(&self) -> &Vec<CommitTag> {
        self.get_node().get_tags()
    }
    pub fn has_tag<S: Into<String>>(&self, tag: S) -> bool {
        let mut has_tag = false;
        let into = tag.into();
        for tag in self.get_tags() {
            if tag.get_tag() == &into {
                has_tag = true;
                break;
            }
        }
        has_tag
    }
    pub fn get_metadata(&self) -> &BranchData {
        self.get_node().get_branch_data()
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
        self.to_normalized_path() == other.to_normalized_path()
    }
}

impl<T: SymbolicNodeType> Eq for NodePath<T> {}

impl<T: SymbolicNodeType> Display for NodePath<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_normalized_path().to_string().as_str())
    }
}

impl<T: SymbolicNodeType> PartialOrd for NodePath<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.to_normalized_path() == other.to_normalized_path() {
            Some(Ordering::Equal)
        } else if self.to_normalized_path() > other.to_normalized_path() {
            Some(Ordering::Greater)
        } else if self.to_normalized_path() < other.to_normalized_path() {
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
