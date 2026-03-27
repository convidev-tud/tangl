use crate::model::*;
use colored::{ColoredString, Colorize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::rc::Rc;
use termtree::Tree;

#[derive(Clone, Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct CommitHash {
    full_hash: String,
}

impl Display for CommitHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.get_short_hash())
    }
}

impl CommitHash {
    pub fn new<S: Into<String>>(full_hash: S) -> Self {
        let full = full_hash.into();
        if full.len() < 8 {
            panic!("Commit hash must be at least 8 characters long");
        }
        CommitHash { full_hash: full }
    }
    pub fn get_full_hash(&self) -> &String {
        &self.full_hash
    }
    pub fn get_short_hash(&self) -> String {
        self.full_hash[0..8].to_string()
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct CommitTag {
    tag: String,
    full_path: String,
}

impl CommitTag {
    pub fn new<S: Into<String>>(full_path: S) -> Self {
        let full_path = full_path.into();
        let normalized = full_path.to_normalized_path();
        let tag = normalized.last().unwrap().to_string();
        CommitTag { tag, full_path }
    }
    pub fn get_full_path(&self) -> &String {
        &self.full_path
    }
    pub fn get_tag(&self) -> &String {
        &self.tag
    }
}

#[derive(Clone, Debug)]
pub struct BranchData {
    branch: Option<String>,
    head: Option<CommitHash>,
}
impl BranchData {
    pub fn new(branch: Option<String>, head: Option<CommitHash>) -> Self {
        Self { branch, head }
    }
    pub fn empty() -> Self {
        Self {
            branch: None,
            head: None,
        }
    }
    pub fn has_branch(&self) -> bool {
        self.branch.is_some()
    }
    pub fn get_branch(&self) -> Option<&String> {
        self.branch.as_ref()
    }
    pub fn get_head(&self) -> Option<&CommitHash> {
        self.head.as_ref()
    }
}

pub enum PayloadType {
    Branch(BranchData),
    Tag(CommitTag),
}

#[derive(Clone, Debug)]
pub struct Node {
    name: String,
    node_type: NodeType,
    branch_data: BranchData,
    tags: Vec<CommitTag>,
    children: HashMap<String, Rc<Node>>,
}

impl Node {
    pub fn new<S: Into<String>>(
        name: S,
        node_type: NodeType,
        branch_data: BranchData,
        tags: Vec<CommitTag>,
    ) -> Self {
        Self {
            name: name.into(),
            node_type,
            branch_data,
            tags,
            children: HashMap::new(),
        }
    }
    pub fn update_branch_data(&mut self, metadata: BranchData) {
        self.branch_data = metadata;
    }
    pub fn add_tag(&mut self, tag: CommitTag) {
        self.tags.push(tag);
    }
    pub fn update_type(&mut self, node_type: NodeType) {
        self.node_type = node_type;
    }
    fn build_display_tree(&self, show_tags: bool) -> Tree<String> {
        let mut formatted = ColoredString::from(self.name.clone());
        if self.branch_data.has_branch() {
            formatted = formatted.blue()
        }
        let type_display = match self.node_type {
            NodeType::AbstractFeature | NodeType::AbstractProduct => None,
            _ => Some(self.node_type.get_formatted_short_name()),
        };
        let content = if let Some(type_display) = type_display {
            format!("{formatted} [{type_display}]")
        } else {
            formatted.to_string()
        };
        let mut tree = Tree::<String>::new(content);
        let mut sorted_children = self.children.iter().collect::<Vec<_>>();
        sorted_children.sort_by(|a, b| b.0.chars().cmp(a.0.chars()));
        sorted_children.reverse();
        for (_, child) in sorted_children {
            tree.leaves.push(child.build_display_tree(show_tags));
        }
        tree
    }
    fn decide_child_type<S: Into<String>>(&self, name: S, metadata: &BranchData) -> NodeType {
        let real_name = name.into();
        self.node_type
            .decide_next_type(real_name.as_str(), metadata)
    }
    fn add_child<S: Into<String>>(&mut self, name: S, metadata: PayloadType) -> NodeType {
        let real_name = name.into();
        let (branch, tags) = match metadata {
            PayloadType::Branch(branch) => (branch, vec![]),
            PayloadType::Tag(tag) => {
                let branch = BranchData::empty();
                (branch, vec![tag])
            }
        };
        let node_type = self.decide_child_type(real_name.clone(), &branch);
        let child = Rc::new(Node::new(
            real_name.clone(),
            node_type.clone(),
            branch,
            tags,
        ));
        self.children.insert(real_name, child);
        node_type
    }
    fn update_child<S: Into<String>>(&mut self, name: S, metadata: PayloadType) -> NodeType {
        let real_name = name.into();
        let node_type = match metadata {
            PayloadType::Branch(branch) => {
                let new_type = self.decide_child_type(real_name.clone(), &branch);
                let child = self.get_child_mut(real_name).unwrap();
                child.update_type(new_type.clone());
                child.update_branch_data(branch);
                new_type
            }
            PayloadType::Tag(tag) => {
                let child = self.get_child_mut(real_name.clone()).unwrap();
                child.add_tag(tag);
                child.get_type().clone()
            }
        };
        node_type
    }
    fn get_child_mut<S: Into<String>>(&mut self, name: S) -> Option<&mut Node> {
        let real_name = name.into();
        let maybe_mut = Rc::get_mut(self.children.get_mut(&real_name)?);
        match maybe_mut {
            Some(node) => Some(node),
            None => panic!(
                "Tried to get child '{}' as mutable but failed: shared references exist\n\
                Make sure to drop all references to the node tree if you attempt modifications",
                real_name
            ),
        }
    }
    pub fn get_name(&self) -> &String {
        &self.name
    }
    pub fn get_type(&self) -> &NodeType {
        &self.node_type
    }
    pub fn get_branch_data(&self) -> &BranchData {
        &self.branch_data
    }
    pub fn get_tags(&self) -> &Vec<CommitTag> {
        &self.tags
    }
    pub fn get_child<S: Into<String>>(&self, name: S) -> Option<&Rc<Node>> {
        Some(self.children.get(&name.into())?)
    }
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
    pub fn iter_children(&self) -> impl Iterator<Item = (&String, &Rc<Node>)> {
        self.children.iter()
    }
    pub fn insert_path(&mut self, path: &NormalizedPath, metadata: PayloadType) -> NodeType {
        match path.len() {
            0 => self.node_type.clone(),
            1 => {
                let name = path.get(0).unwrap().to_string();
                let new_type = match self.get_child_mut(&name) {
                    Some(_) => self.update_child(name, metadata),
                    None => self.add_child(name.clone(), metadata),
                };
                new_type
            }
            _ => {
                let name = path.get(0).unwrap().to_string();
                let next_child = match self.get_child_mut(&name) {
                    Some(node) => node,
                    None => {
                        self.add_child(name.clone(), PayloadType::Branch(BranchData::empty()));
                        self.get_child_mut(&name).unwrap()
                    }
                };
                next_child.insert_path(&path.strip_n_left(1), metadata)
            }
        }
    }
    pub fn as_qualified_path(&self) -> NormalizedPath {
        NormalizedPath::from(self.name.clone())
    }
    pub fn get_qualified_paths_by<T, P>(
        &self,
        initial_path: &NormalizedPath,
        predicate: &P,
        categories: &Vec<T>,
    ) -> HashMap<T, Vec<NormalizedPath>>
    where
        P: Fn(&T, &Node) -> bool,
        T: Hash + Eq + Clone + Debug,
    {
        let mut result: HashMap<T, Vec<NormalizedPath>> = HashMap::new();
        for child in self.children.values() {
            let path = initial_path.clone() + child.as_qualified_path();
            for t in categories {
                let to_insert = if predicate(t, child) {
                    vec![path.clone()]
                } else {
                    vec![]
                };
                if result.contains_key(t) {
                    result.get_mut(t).unwrap().extend(to_insert);
                } else {
                    result.insert(t.clone(), to_insert);
                }
            }
            let from_child = child.get_qualified_paths_by(&path, predicate, categories);
            for (t, value) in from_child {
                result.get_mut(&t).unwrap().extend(value);
            }
        }
        result
    }
    pub fn display_tree(&self, show_tags: bool) -> String {
        self.build_display_tree(show_tags).to_string()
    }
}
