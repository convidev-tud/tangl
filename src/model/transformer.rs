use crate::model::*;
use globset::{GlobBuilder, GlobMatcher};

pub trait NodePathTransformer<A, B>
where
    A: SymbolicNodeType,
    B: SymbolicNodeType,
{
    fn apply(&self, node_path: NodePath<A>) -> Option<NodePath<B>>;
    fn transform(
        &self,
        node_paths: impl Iterator<Item = NodePath<A>>,
    ) -> impl Iterator<Item = NodePath<B>> {
        node_paths.filter_map(|path| self.apply(path))
    }
}

pub enum NodePathTransformers {
    ChainingNodePathTransformer(ChainingNodePathTransformer),
    HasBranchFilteringNodePathTransformer(HasBranchFilteringNodePathTransformer),
    ByQPathFilteringNodePathTransformer(ByQPathFilteringNodePathTransformer),
    ByGlobFilteringNodePathTransformer(ByGlobFilteringNodePathTransformer),
}
impl NodePathTransformer<AnyNode, AnyNode> for NodePathTransformers {
    fn apply(&self, node_path: NodePath<AnyNode>) -> Option<NodePath<AnyNode>> {
        match self {
            NodePathTransformers::ChainingNodePathTransformer(t) => t.apply(node_path),
            NodePathTransformers::HasBranchFilteringNodePathTransformer(t) => t.apply(node_path),
            NodePathTransformers::ByQPathFilteringNodePathTransformer(t) => t.apply(node_path),
            NodePathTransformers::ByGlobFilteringNodePathTransformer(t) => t.apply(node_path),
        }
    }
}

pub struct ChainingNodePathTransformer {
    transformers: Vec<NodePathTransformers>,
}
impl ChainingNodePathTransformer {
    pub fn new(transformers: Vec<NodePathTransformers>) -> Self {
        Self { transformers }
    }
}
impl NodePathTransformer<AnyNode, AnyNode> for ChainingNodePathTransformer {
    fn apply(&self, node_path: NodePath<AnyNode>) -> Option<NodePath<AnyNode>> {
        let mut result: Option<NodePath<AnyNode>> = Some(node_path);
        for transformer in self.transformers.iter() {
            result = Some(transformer.apply(result.take()?)?);
        }
        result
    }
}

pub struct HasBranchFilteringNodePathTransformer {
    has_branch: bool,
}
impl HasBranchFilteringNodePathTransformer {
    pub fn new(has_branch: bool) -> HasBranchFilteringNodePathTransformer {
        Self { has_branch }
    }
}
impl<A: SymbolicNodeType> NodePathTransformer<A, A> for HasBranchFilteringNodePathTransformer {
    fn apply(&self, node_path: NodePath<A>) -> Option<NodePath<A>> {
        if node_path.get_metadata().has_branch() == self.has_branch {
            Some(node_path)
        } else {
            None
        }
    }
}

pub enum FilteringMode {
    INCLUDE,
    EXCLUDE,
}
pub struct ByQPathFilteringNodePathTransformer {
    paths: Vec<QualifiedPath>,
    mode: FilteringMode,
}
impl ByQPathFilteringNodePathTransformer {
    pub fn new(paths: Vec<QualifiedPath>, mode: FilteringMode) -> Self {
        Self { paths, mode }
    }
}
impl<A: SymbolicNodeType> NodePathTransformer<A, A> for ByQPathFilteringNodePathTransformer {
    fn apply(&self, node_path: NodePath<A>) -> Option<NodePath<A>> {
        match self.mode {
            FilteringMode::INCLUDE => {
                if self.paths.contains(&node_path.to_qualified_path()) {
                    Some(node_path)
                } else {
                    None
                }
            }
            FilteringMode::EXCLUDE => {
                if self.paths.contains(&node_path.to_qualified_path()) {
                    None
                } else {
                    Some(node_path)
                }
            }
        }
    }
}

pub struct ByGlobFilteringNodePathTransformer {
    globs: Vec<GlobMatcher>,
    filtering_mode: FilteringMode,
}
impl ByGlobFilteringNodePathTransformer {
    pub fn new(
        globs: &Vec<QualifiedPath>,
        filtering_mode: FilteringMode,
    ) -> Result<Self, globset::Error> {
        let mut built = Vec::new();
        for glob in globs {
            built.push(
                GlobBuilder::new(glob.to_string().as_str())
                    .build()?
                    .compile_matcher(),
            );
        }
        Ok(Self {
            globs: built,
            filtering_mode,
        })
    }
}
impl NodePathTransformer<AnyNode, AnyNode> for ByGlobFilteringNodePathTransformer {
    fn apply(&self, node_path: NodePath<AnyNode>) -> Option<NodePath<AnyNode>> {
        let mut found_match = false;
        for glob in self.globs.iter() {
            if glob.is_match(&node_path.to_string()) {
                found_match = true
            }
        }
        match self.filtering_mode {
            FilteringMode::INCLUDE => {
                if found_match {
                    Some(node_path)
                } else {
                    None
                }
            }
            FilteringMode::EXCLUDE => {
                if found_match {
                    None
                } else {
                    Some(node_path)
                }
            }
        }
    }
}

pub struct ToTypeNodePathTransformer {}
impl ToTypeNodePathTransformer {
    pub fn new() -> Self {
        Self {}
    }
}
impl<T1, T2> NodePathTransformer<T1, T2> for ToTypeNodePathTransformer
where
    T1: SymbolicNodeType,
    T2: SymbolicNodeType,
{
    fn apply(&self, node_path: NodePath<T1>) -> Option<NodePath<T2>> {
        node_path.try_convert_to::<T2>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TreeDataModel;

    fn prepare_model() -> TreeDataModel {
        let mut model = TreeDataModel::new();
        model.insert_qualified_path(QualifiedPath::from("/main/feature/root"), false);
        model.insert_qualified_path(QualifiedPath::from("/main/feature/root/foo"), false);
        model
    }

    #[test]
    fn test_chaining_node_path_transformer() {
        let model = prepare_model();
        let chain = ChainingNodePathTransformer::new(vec![
            NodePathTransformers::ByQPathFilteringNodePathTransformer(
                ByQPathFilteringNodePathTransformer::new(
                    vec![QualifiedPath::from("/main/feature/root")],
                    FilteringMode::EXCLUDE,
                ),
            ),
            NodePathTransformers::HasBranchFilteringNodePathTransformer(
                HasBranchFilteringNodePathTransformer::new(true),
            ),
        ]);
        let root = model.get_virtual_root();
        let actual = chain
            .transform(root.iter_children_req())
            .map(|node_path| node_path.to_qualified_path())
            .collect::<Vec<_>>();
        assert_eq!(actual, vec!["/main/feature/root/foo"]);
    }

    #[test]
    fn test_q_path_filtering_node_path_transformer_include() {
        let model = prepare_model();
        let transformer = ByQPathFilteringNodePathTransformer::new(
            vec![QualifiedPath::from("/main/feature/root")],
            FilteringMode::INCLUDE,
        );
        let root = model.get_virtual_root();
        let actual = transformer
            .transform(root.iter_children_req())
            .map(|node_path| node_path.to_qualified_path())
            .collect::<Vec<_>>();
        assert_eq!(actual, vec!["/main/feature/root"]);
    }

    #[test]
    fn test_q_path_filtering_node_path_transformer_exclude() {
        let model = prepare_model();
        let transformer = ByQPathFilteringNodePathTransformer::new(
            vec![QualifiedPath::from("/main/feature/root")],
            FilteringMode::EXCLUDE,
        );
        let root = model.get_virtual_root();
        let actual = transformer
            .transform(root.iter_children_req())
            .map(|node_path| node_path.to_qualified_path())
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec!["/main", "/main/feature", "/main/feature/root/foo"]
        );
    }
}
