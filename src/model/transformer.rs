use crate::model::*;
use globset::{GlobBuilder, GlobMatcher};
use std::marker::PhantomData;

pub trait NodePathTransformer<A, B>
where
    A: SymbolicNodeType,
    B: SymbolicNodeType,
{
    fn apply(&self, node_path: Option<NodePath<A>>) -> Option<NodePath<B>>;
    fn transform(
        &self,
        node_paths: impl Iterator<Item = NodePath<A>>,
    ) -> impl Iterator<Item = NodePath<B>> {
        node_paths.filter_map(|path| self.apply(Some(path)))
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
    fn apply(&self, node_path: Option<NodePath<A>>) -> Option<NodePath<A>> {
        let path = node_path?;
        if path.get_metadata().has_branch() == self.has_branch {
            Some(path)
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
    paths: Vec<NormalizedPath>,
    mode: FilteringMode,
}
impl ByQPathFilteringNodePathTransformer {
    pub fn new(paths: Vec<NormalizedPath>, mode: FilteringMode) -> Self {
        Self { paths, mode }
    }
}
impl<A: SymbolicNodeType> NodePathTransformer<A, A> for ByQPathFilteringNodePathTransformer {
    fn apply(&self, node_path: Option<NodePath<A>>) -> Option<NodePath<A>> {
        let path = node_path?;
        match self.mode {
            FilteringMode::INCLUDE => {
                if self.paths.contains(&path.to_normalized_path()) {
                    Some(path)
                } else {
                    None
                }
            }
            FilteringMode::EXCLUDE => {
                if self.paths.contains(&path.to_normalized_path()) {
                    None
                } else {
                    Some(path)
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
        globs: &Vec<NormalizedPath>,
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
impl<A: SymbolicNodeType> NodePathTransformer<A, A> for ByGlobFilteringNodePathTransformer {
    fn apply(&self, node_path: Option<NodePath<A>>) -> Option<NodePath<A>> {
        let path = node_path?;
        let mut found_match = false;
        for glob in self.globs.iter() {
            if glob.is_match(&path.to_string()) {
                found_match = true
            }
        }
        match self.filtering_mode {
            FilteringMode::INCLUDE => {
                if found_match {
                    Some(path)
                } else {
                    None
                }
            }
            FilteringMode::EXCLUDE => {
                if found_match {
                    None
                } else {
                    Some(path)
                }
            }
        }
    }
}

pub struct ByTypeFilteringNodePathTransformer<In, Out>
where
    In: SymbolicNodeType,
    Out: SymbolicNodeType,
{
    _in: PhantomData<In>,
    _out: PhantomData<Out>,
}
impl<In, Out> ByTypeFilteringNodePathTransformer<In, Out>
where
    In: SymbolicNodeType,
    Out: SymbolicNodeType,
{
    pub fn new() -> Self {
        Self {
            _in: PhantomData,
            _out: PhantomData,
        }
    }
}
impl<In, Out> NodePathTransformer<In, Out> for ByTypeFilteringNodePathTransformer<In, Out>
where
    In: SymbolicNodeType,
    Out: SymbolicNodeType,
{
    fn apply(&self, node_path: Option<NodePath<In>>) -> Option<NodePath<Out>> {
        node_path?.try_convert_to::<Out>()
    }
}

// Compound transformers

pub struct GlobToTypeNodePathTransformer<In, Out>
where
    In: SymbolicNodeType,
    Out: SymbolicNodeType,
{
    glob_filter: ByGlobFilteringNodePathTransformer,
    type_filter: ByTypeFilteringNodePathTransformer<In, Out>,
}
impl<In, Out> GlobToTypeNodePathTransformer<In, Out>
where
    In: SymbolicNodeType,
    Out: SymbolicNodeType,
{
    pub fn new(globs: &Vec<NormalizedPath>, mode: FilteringMode) -> Result<Self, globset::Error> {
        let glob_filter = ByGlobFilteringNodePathTransformer::new(globs, mode)?;
        let type_filter = ByTypeFilteringNodePathTransformer::new();
        Ok(Self {
            glob_filter,
            type_filter,
        })
    }
}
impl<In, Out> NodePathTransformer<In, Out> for GlobToTypeNodePathTransformer<In, Out>
where
    In: SymbolicNodeType,
    Out: SymbolicNodeType,
{
    fn apply(&self, node_path: Option<NodePath<In>>) -> Option<NodePath<Out>> {
        self.type_filter.apply(self.glob_filter.apply(node_path))
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::model::TreeDataModel;
//
//     fn prepare_model() -> TreeDataModel {
//         let mut model = TreeDataModel::new();
//         model.insert_git_branch(NormalizedPath::from("/main/feature/root"), false);
//         model.insert_git_branch(NormalizedPath::from("/main/feature/root/foo"), false);
//         model
//     }
//
//     #[test]
//     fn test_q_path_filtering_node_path_transformer_include() {
//         let model = prepare_model();
//         let transformer = ByQPathFilteringNodePathTransformer::new(
//             vec![NormalizedPath::from("/main/feature/root")],
//             FilteringMode::INCLUDE,
//         );
//         let root = model.get_virtual_root();
//         let actual = transformer
//             .transform(root.iter_children_req())
//             .map(|node_path| node_path.to_normalized_path())
//             .collect::<Vec<_>>();
//         assert_eq!(actual, vec!["/main/feature/root"]);
//     }
//
//     #[test]
//     fn test_q_path_filtering_node_path_transformer_exclude() {
//         let model = prepare_model();
//         let transformer = ByQPathFilteringNodePathTransformer::new(
//             vec![NormalizedPath::from("/main/feature/root")],
//             FilteringMode::EXCLUDE,
//         );
//         let root = model.get_virtual_root();
//         let actual = transformer
//             .transform(root.iter_children_req())
//             .map(|node_path| node_path.to_normalized_path())
//             .collect::<Vec<_>>();
//         assert_eq!(
//             actual,
//             vec!["/main", "/main/feature", "/main/feature/root/foo"]
//         );
//     }
// }
