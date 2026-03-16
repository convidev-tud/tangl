use crate::model::*;
use colored::{ColoredString, Colorize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;
use termtree::Tree;

#[derive(Clone, Debug)]
pub struct NodeMetadata {
    has_branch: bool,
}
impl NodeMetadata {
    pub fn new(has_branch: bool) -> Self {
        Self { has_branch }
    }
    pub fn default() -> Self {
        let i = "".to_string();
        drop(i);
        Self { has_branch: false }
    }
    pub fn has_branch(&self) -> bool {
        self.has_branch
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    name: String,
    node_type: NodeType,
    metadata: NodeMetadata,
    children: HashMap<String, Rc<Node>>,
}

impl Node {
    pub fn new<S: Into<String>>(name: S, node_type: NodeType, metadata: NodeMetadata) -> Self {
        Self {
            name: name.into(),
            node_type,
            metadata,
            children: HashMap::new(),
        }
    }
    pub fn update_metadata(&mut self, metadata: NodeMetadata) {
        self.metadata = metadata;
    }
    pub fn update_type(&mut self, node_type: NodeType) {
        self.node_type = node_type;
    }
    fn build_display_tree(&self, show_tags: bool) -> Tree<String> {
        let mut formatted = ColoredString::from(self.name.clone());
        if self.metadata.has_branch {
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
            match child.node_type {
                NodeType::Tag => {
                    if !show_tags {
                        continue;
                    }
                }
                _ => {}
            }
            tree.leaves.push(child.build_display_tree(show_tags));
        }
        tree
    }
    fn decide_child_type<S: Into<String>>(
        &self,
        name: S,
        metadata: &NodeMetadata,
        is_tag: bool,
    ) -> NodeType {
        let real_name = name.into();
        let new_type = if is_tag {
            NodeType::Tag
        } else {
            self.node_type
                .decide_next_type(real_name.as_str(), metadata)
        };
        new_type
    }
    fn add_child<S: Into<String>>(
        &mut self,
        name: S,
        metadata: NodeMetadata,
        is_tag: bool,
    ) -> NodeType {
        let real_name = name.into();
        let new_type = self.decide_child_type(real_name.clone(), &metadata, is_tag);
        self.children.insert(
            real_name.clone(),
            Rc::new(Node::new(real_name, new_type.clone(), metadata)),
        );
        new_type
    }
    fn update_child<S: Into<String>>(
        &mut self,
        name: S,
        metadata: NodeMetadata,
        is_tag: bool,
    ) -> NodeType {
        let real_name = name.into();
        let new_type = self.decide_child_type(real_name.clone(), &metadata, is_tag);
        let child = self.get_child_mut(real_name).unwrap();
        child.update_metadata(metadata);
        child.update_type(new_type.clone());
        new_type
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
    pub fn get_metadata(&self) -> &NodeMetadata {
        &self.metadata
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
    pub fn insert_node_path(
        &mut self,
        path: &QualifiedPath,
        metadata: NodeMetadata,
        is_tag: bool,
    ) -> NodeType {
        let name = path.get(0).unwrap().to_string();
        match path.len() {
            0 => self.node_type.clone(),
            1 => {
                let new_type = match self.get_child_mut(&name) {
                    Some(_) => {
                        self.update_child(name, metadata, is_tag)
                    }
                    None => {
                        self.add_child(name.clone(), metadata, is_tag)
                    }
                };
                new_type
            }
            _ => {
                let next_child = match self.get_child_mut(&name) {
                    Some(node) => node,
                    None => {
                        self.add_child(name.clone(), NodeMetadata::default(), false);
                        self.get_child_mut(&name).unwrap()
                    }
                };
                next_child.insert_node_path(&path.strip_n_left(1), metadata, is_tag)
            }
        }
    }
    pub fn as_qualified_path(&self) -> QualifiedPath {
        QualifiedPath::from(self.name.clone())
    }
    pub fn get_qualified_paths_by<T, P>(
        &self,
        initial_path: &QualifiedPath,
        predicate: &P,
        categories: &Vec<T>,
    ) -> HashMap<T, Vec<QualifiedPath>>
    where
        P: Fn(&T, &Node) -> bool,
        T: Hash + Eq + Clone + Debug,
    {
        let mut result: HashMap<T, Vec<QualifiedPath>> = HashMap::new();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn prepare_node() -> Node {
        let mut node = Node::new("root", NodeType::ConcreteFeature, NodeMetadata::default());
        node.insert_node_path(
            &QualifiedPath::from("foo/f1"),
            NodeMetadata::default(),
            false,
        );
        node.insert_node_path(
            &QualifiedPath::from("bar/b1"),
            NodeMetadata::default(),
            false,
        );
        node
    }

    #[test]
    fn test_get_qualified_paths_by() {
        let predicate = |_: &i32, node: &Node| -> bool { !node.get_metadata().has_branch };
        let node = prepare_node();
        let result = node
            .get_qualified_paths_by(&QualifiedPath::new(), &predicate, &vec![0])
            .get(&0)
            .unwrap()
            .clone();
        assert!(result.contains(&QualifiedPath::from("foo")));
        assert!(result.contains(&QualifiedPath::from("bar")));
        assert!(result.contains(&QualifiedPath::from("foo/f1")));
        assert!(result.contains(&QualifiedPath::from("bar/b1")));
    }
}
