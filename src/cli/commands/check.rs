use crate::cli::completion::CompletionHelper;
use crate::cli::*;
use crate::git::conflict::{ConflictChecker, ConflictStatistics};
use crate::model::{
    ConcreteBranch, ByGlobFilteringNodePathTransformer, ChainingNodePathTransformer, ConcreteFeature,
    FeatureMetadata, FilteringMode, HasBranchFilteringNodePathTransformer, NodePath,
    NodePathTransformer, NodePathTransformers, ConcreteProduct, QualifiedPath, ToQualifiedPath,
};
use clap::{Arg, ArgAction, Command};
use colored::Colorize;
use std::error::Error;

const PATHS: &str = "paths";
const PERMUTATIONS: &str = "permutations";
const BY_ORDER: &str = "by_order";
const ONE_TO_N: &str = "one_to_n";
const PERM_TO_BASE: &str = "perm_to_base";

fn run_checks(
    paths: &Vec<NodePath<ConcreteBranch>>,
    permutations: Option<usize>,
    perm_to_base: Option<usize>,
    by_order: bool,
    test_one_to_n: bool,
    checker: &ConflictChecker,
) -> Result<ConflictStatistics, Box<dyn Error>> {
    if paths.len() < 2 {
        return Err("At least two paths are required to perform merge tests".into());
    }
    let statistics: ConflictStatistics = match (permutations, perm_to_base, by_order, test_one_to_n)
    {
        (None, None, false, false) => return Err("Please choose a test strategy".into()),
        (Some(permutations), None, false, false) => {
            checker.check_k_permutations(paths, permutations).collect()
        }
        (None, Some(permutations), false, false) => {
            let base = paths.get(0).unwrap().clone();
            let rest = paths[1..].to_vec();
            checker
                .check_permutations_against_base(&rest, &base, permutations)
                .collect()
        }
        (None, None, true, false) => {
            ConflictStatistics::from_iter(vec![checker.check_by_order(paths)].into_iter())
        }
        (None, None, false, true) => {
            let first = paths.first().unwrap().clone();
            let rest = paths[1..].to_vec();
            checker
                .check_n_against_permutations(&vec![first], &rest, &1)
                .collect()
        }
        _ => unreachable!(),
    };
    Ok(statistics)
}

#[derive(Clone, Debug)]
pub struct CheckCommand;

impl CommandDefinition for CheckCommand {
    fn build_command(&self) -> Command {
        Command::new("test")
            .about("Test paths for merge conflicts")
            .disable_help_subcommand(true)
            .arg(verbose())
            .arg(
                Arg::new(PATHS)
                    .action(ArgAction::Append)
                    .help("Paths to test"),
            )
            .arg(
                Arg::new(PERMUTATIONS)
                    .long("perm")
                    .conflicts_with_all(&[ONE_TO_N, BY_ORDER, PERM_TO_BASE])
                    .help("Test paths in all possible permutations by given length"),
            )
            .arg(
                Arg::new(BY_ORDER)
                    .long("by-order")
                    .action(ArgAction::SetTrue)
                    .conflicts_with_all(&[ONE_TO_N, PERMUTATIONS, PERM_TO_BASE])
                    .help("Test paths by given order")
                    .long_help(
                        "Test paths in given order. The most left argument serves as the base, \
                        and all following paths will be merged onto the base one by one. \
                        Per permutation, the test aborts if a merge raises a conflict.",
                    ),
            )
            .arg(
                Arg::new(ONE_TO_N)
                    .long("one-to-n")
                    .action(ArgAction::SetTrue)
                    .conflicts_with_all(&[BY_ORDER, PERMUTATIONS, PERM_TO_BASE])
                    .help("Test most left path against all others in pair-wise merge"),
            )
            .arg(
                Arg::new(PERM_TO_BASE)
                    .long("perm-to-base")
                    .conflicts_with_all(&[BY_ORDER, PERMUTATIONS, ONE_TO_N])
                    .help(
                        "The most left path is the base; all other paths are permutated \
                        and tested against the base",
                    ),
            )
    }
}

impl CommandInterface for CheckCommand {
    fn run_command(&self, context: &mut CommandContext) -> Result<(), Box<dyn Error>> {
        let paths = context
            .arg_helper
            .get_argument_values::<String>(&PATHS)
            .unwrap_or(Vec::new())
            .iter()
            .map(|p| QualifiedPath::from(p))
            .collect::<Vec<_>>();
        let permutations = context
            .arg_helper
            .get_argument_value::<String>(&PERMUTATIONS);
        let perm_to_base = context
            .arg_helper
            .get_argument_value::<String>(&PERM_TO_BASE);
        let by_order = context
            .arg_helper
            .get_argument_value::<bool>(&BY_ORDER)
            .unwrap();
        let one_to_n = context
            .arg_helper
            .get_argument_value::<bool>(&ONE_TO_N)
            .unwrap();
        let checker = ConflictChecker::new(&context.git);

        let statistics: ConflictStatistics = if paths.is_empty()
            && permutations.is_none()
            && perm_to_base.is_none()
            && !by_order
            && !one_to_n
        {
            let current_path = context.git.get_current_node_path()?;
            if current_path.try_convert_to::<ConcreteFeature>().is_some() {
                let feature_root = context
                    .git
                    .get_current_area()?
                    .move_to_feature_root()
                    .unwrap();
                let mut paths = vec![current_path.clone()];
                paths.extend(feature_root.iter_features_req().filter_map(|path| {
                    if &path == &current_path {
                        None
                    } else {
                        Some(path.try_convert_to().unwrap())
                    }
                }));
                run_checks(&paths, None, None, false, true, &checker)?
            } else if let Some(product) = current_path.try_convert_to::<ConcreteProduct>() {
                let derivation_commits = context.git.get_derivation_commits(&product)?;
                let maybe_last = derivation_commits.first();
                if maybe_last.is_none() {
                    return Err("Nothing to check against: product not derived yet".into());
                }
                let feature_meta = maybe_last.unwrap().get_metadata().get_total();
                let features = FeatureMetadata::qualified_paths(feature_meta);
                let node_paths = context
                    .git
                    .get_model()
                    .assert_all::<ConcreteBranch>(&features)?;
                let mut final_paths: Vec<NodePath<ConcreteBranch>> = vec![product.try_convert_to().unwrap()];
                final_paths.extend(node_paths);
                run_checks(&final_paths, None, Some(1), false, false, &checker)?
            } else {
                return Err("No default available for current node type\n\
                    Please supply test paths and a strategy"
                    .into());
            }
        } else {
            let current_path = context.git.get_current_qualified_path()?;
            let transformed_paths: Vec<QualifiedPath> = paths
                .iter()
                .map(|path| current_path.clone() + path.clone())
                .collect();
            let filter1 = HasBranchFilteringNodePathTransformer::new(true);
            let filter2 = ByGlobFilteringNodePathTransformer::new(
                &transformed_paths,
                FilteringMode::INCLUDE,
            )?;
            let node_finder = ChainingNodePathTransformer::new(vec![
                NodePathTransformers::HasBranchFilteringNodePathTransformer(filter1),
                NodePathTransformers::ByGlobFilteringNodePathTransformer(filter2),
            ]);
            let root = context.git.get_model().get_virtual_root();
            let iterator = root.iter_children_req();
            let final_paths: Vec<NodePath<ConcreteBranch>> = node_finder
                .transform(iterator)
                .map(|path| path.try_convert_to::<ConcreteBranch>().unwrap())
                .collect();
            let perm: Option<usize> = match permutations {
                Some(p) => Some(p.parse()?),
                None => None,
            };
            let perm_to_b: Option<usize> = match perm_to_base {
                Some(p) => Some(p.parse()?),
                None => None,
            };
            run_checks(&final_paths, perm, perm_to_b, by_order, one_to_n, &checker)?
        };

        for ok in statistics.iter_ok() {
            context.debug(ok.display_as_path())
        }
        for conflict in statistics.iter_conflicts() {
            context.warn(conflict.display_as_path())
        }
        for error in statistics.iter_errors() {
            context.error(error.display_as_path())
        }
        if statistics.n_conflict() == 0 {
            context.info("No conflicts".green().to_string());
        }
        Ok(())
    }

    fn shell_complete(
        &self,
        completion_helper: CompletionHelper,
        context: &mut CommandContext,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let currently_editing = completion_helper.currently_editing();
        let completion: Vec<String> = if currently_editing.is_some() {
            let root = context.git.get_model().get_virtual_root();
            let transformer = HasBranchFilteringNodePathTransformer::new(true);
            let relevant_paths = transformer.transform(root.iter_children_req());
            match currently_editing.unwrap().get_id().as_str() {
                PATHS => {
                    let current_path = context.git.get_current_qualified_path()?;
                    let to_exclude = completion_helper.get_appendix_of(PATHS);
                    let to_exclude_paths = to_exclude
                        .into_iter()
                        .map(|p| current_path.clone() + QualifiedPath::from(p))
                        .collect();
                    let filter = ByGlobFilteringNodePathTransformer::new(
                        &to_exclude_paths,
                        FilteringMode::EXCLUDE,
                    )?;
                    let filtered = filter.transform(relevant_paths);
                    completion_helper.complete_qualified_paths(
                        context.git.get_current_qualified_path()?,
                        filtered.map(|path| path.to_qualified_path()),
                    )
                }
                _ => {
                    vec![]
                }
            }
        } else {
            vec![]
        };
        Ok(completion)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::interface::test_utils::*;
    use crate::git::interface::{GitInterface, GitPath};
    use crate::model::ImportFormat;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_check_error_if_not_all_and_no_source() {
        let path = TempDir::new().unwrap();
        let path_buf = PathBuf::from(path.path());
        prepare_empty_git_repo(path_buf.clone()).unwrap();
        populate_with_features(path_buf.clone()).unwrap();
        let repo = CommandRepository::new(
            Box::new(CheckCommand),
            GitPath::CustomDirectory(PathBuf::from(path.path())),
        );
        match repo.execute(ArgSource::SUPPLIED(vec!["check"])) {
            Ok(_) => {
                panic!("Should fail")
            }
            Err(_) => {
                assert!(true)
            }
        }
    }

    #[test]
    fn test_check_all() {
        let path = TempDir::new().unwrap();
        let path_buf = PathBuf::from(path.path());
        prepare_empty_git_repo(path_buf.clone()).unwrap();
        populate_with_features(path_buf.clone()).unwrap();
        let repo = CommandRepository::new(
            Box::new(CheckCommand),
            GitPath::CustomDirectory(PathBuf::from(path.path())),
        );
        let context = repo.build_context(
            ArgSource::SUPPLIED(vec!["check", "--all"]),
            ImportFormat::Native,
        );
        match run_check(&context) {
            Ok(statistics) => {
                assert_eq!(statistics.n_ok(), 6);
                assert_eq!(statistics.n_conflict(), 0);
                assert_eq!(statistics.n_errors(), 0);
            }
            Err(_) => {
                panic!()
            }
        }
    }

    #[test]
    fn test_check_current_feature() {
        let path = TempDir::new().unwrap();
        let path_buf = PathBuf::from(path.path());
        prepare_empty_git_repo(path_buf.clone()).unwrap();
        populate_with_features(path_buf.clone()).unwrap();
        GitInterface::new(GitPath::CustomDirectory(path_buf))
            .checkout(&QualifiedPath::from("/main/feature/root/foo"))
            .unwrap();
        let repo = CommandRepository::new(
            Box::new(CheckCommand),
            GitPath::CustomDirectory(PathBuf::from(path.path())),
        );
        let context = repo.build_context(
            ArgSource::SUPPLIED(vec!["check", "."]),
            ImportFormat::Native,
        );
        match run_check(&context) {
            Ok(statistics) => {
                assert_eq!(statistics.n_ok(), 3);
                assert_eq!(statistics.n_conflict(), 0);
                assert_eq!(statistics.n_errors(), 0);
            }
            Err(_) => {
                panic!()
            }
        }
    }

    #[test]
    fn test_check_no_feature() {
        let path = TempDir::new().unwrap();
        let path_buf = PathBuf::from(path.path());
        prepare_empty_git_repo(path_buf.clone()).unwrap();
        populate_with_features(path_buf.clone()).unwrap();
        let repo = CommandRepository::new(
            Box::new(CheckCommand),
            GitPath::CustomDirectory(PathBuf::from(path.path())),
        );
        let context = repo.build_context(
            ArgSource::SUPPLIED(vec!["check", "."]),
            ImportFormat::Native,
        );
        match run_check(&context) {
            Ok(_) => {
                panic!("Should fail")
            }
            Err(_) => {
                assert!(true)
            }
        }
    }

    #[test]
    fn test_check_specific_targets_relative_path() {
        let path = TempDir::new().unwrap();
        let path_buf = PathBuf::from(path.path());
        prepare_empty_git_repo(path_buf.clone()).unwrap();
        populate_with_features(path_buf.clone()).unwrap();
        let repo = CommandRepository::new(
            Box::new(CheckCommand),
            GitPath::CustomDirectory(PathBuf::from(path.path())),
        );
        let context = repo.build_context(
            ArgSource::SUPPLIED(vec![
                "check",
                "feature/root/foo",
                "feature/root/bar",
                "feature/root/baz",
            ]),
            ImportFormat::Native,
        );
        match run_check(&context) {
            Ok(statistics) => {
                assert_eq!(statistics.n_ok(), 2);
                assert_eq!(statistics.n_conflict(), 0);
                assert_eq!(statistics.n_errors(), 0);
            }
            Err(_) => {
                panic!()
            }
        }
    }

    #[test]
    fn test_check_specific_targets_absolute_path() {
        let path = TempDir::new().unwrap();
        let path_buf = PathBuf::from(path.path());
        prepare_empty_git_repo(path_buf.clone()).unwrap();
        populate_with_features(path_buf.clone()).unwrap();
        let repo = CommandRepository::new(
            Box::new(CheckCommand),
            GitPath::CustomDirectory(PathBuf::from(path.path())),
        );
        GitInterface::new(GitPath::CustomDirectory(path_buf))
            .checkout(&QualifiedPath::from("/main/feature/root/foo"))
            .unwrap();
        let context = repo.build_context(
            ArgSource::SUPPLIED(vec![
                "check",
                "/main/feature/root/foo",
                "/main/feature/root/bar",
                "/main/feature/root/baz",
            ]),
            ImportFormat::Native,
        );
        match run_check(&context) {
            Ok(statistics) => {
                assert_eq!(statistics.n_ok(), 2);
                assert_eq!(statistics.n_conflict(), 0);
                assert_eq!(statistics.n_errors(), 0);
            }
            Err(_) => {
                panic!()
            }
        }
    }
}
