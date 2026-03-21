use crate::cli::completion::CompletionHelper;
use crate::cli::*;
use crate::git::conflict::{ConflictChecker, MergeChainStatistics};
use crate::git::error::GitError;
use crate::model::*;
use crate::spl::{FeatureMetadata, InspectionManager};
use clap::{Arg, ArgAction, Command};
use colored::Colorize;
use std::error::Error;

const PATHS: &str = "paths";
const PERMUTATIONS: &str = "permutations";
const BY_ORDER: &str = "by_order";
const ONE_TO_N: &str = "one_to_n";
const PERM_TO_BASE: &str = "perm_to_base";

fn run_checks(
    paths: &Vec<NodePath<AnyHasBranch>>,
    permutations: Option<usize>,
    perm_to_base: Option<usize>,
    by_order: bool,
    test_one_to_n: bool,
    checker: &ConflictChecker,
) -> Result<MergeChainStatistics, Box<dyn Error>> {
    if paths.len() < 2 {
        return Err("At least two paths are required to perform merge tests".into());
    }
    let statistics: Result<MergeChainStatistics, GitError> =
        match (permutations, perm_to_base, by_order, test_one_to_n) {
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
                let mut statistics = MergeChainStatistics::new();
                statistics.fill_from_iter(vec![checker.check_by_order(paths)?].into_iter());
                Ok(statistics)
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
    Ok(statistics?)
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

        let statistics: MergeChainStatistics = if paths.is_empty()
            && permutations.is_none()
            && perm_to_base.is_none()
            && !by_order
            && !one_to_n
        {
            let current_path = context.git.assert_current_node_path()?;
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
                let inspector = InspectionManager::new(&context.git);
                let state = inspector.get_current_derivation_state(&product)?;
                if state.get_total().len() == 0 {
                    return Err("Nothing to check against: product not derived yet".into());
                }
                let feature_meta = state.get_total();
                let features = FeatureMetadata::qualified_paths(feature_meta);
                let node_paths = context
                    .git
                    .get_model()
                    .assert_all::<AnyHasBranch>(&features)?;
                let mut final_paths: Vec<NodePath<AnyHasBranch>> =
                    vec![product.try_convert_to().unwrap()];
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
            let mut final_paths: Vec<NodePath<AnyHasBranch>> = Vec::new();
            for path in transformed_paths.iter() {
                let s = path.to_string();
                if s.contains("*") || s.contains("[") || s.contains("]") {
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
                    let found: Vec<NodePath<AnyHasBranch>> = node_finder
                        .transform(iterator)
                        .map(|path| path.try_convert_to::<AnyHasBranch>().unwrap())
                        .collect();
                    final_paths.extend(found);
                } else {
                    final_paths.push(context.git.get_model().assert_path(&path)?)
                }
            }
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

        for statistic in statistics.iter_all() {
            if statistic.contains_conflicts() {
                context.info(statistic.display_as_path());
            } else {
                context.debug(statistic.display_as_path());
            }
        }
        if statistics.n_conflicts() == 0 {
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
