use crate::cli::completion::*;
use crate::cli::*;
use crate::git::conflict::{ConflictAnalyzer, ConflictChecker, ConflictStatistic};
use crate::git::interface::GitInterface;
use crate::model::*;
use clap::{Arg, ArgAction, Command};
use colored::Colorize;
use std::error::Error;
use serde_json::to_string;
use crate::git::error::GitError;
use crate::spl::{DerivationData, DerivationError, DerivationManager};

const FEATURES: &str = "features";
const CONTINUE: &str = "continue";
const ABORT: &str = "abort";
const RESET: &str = "reset";
const OPTIMIZATION: &str = "optimization";

fn approximate_merge_order(
    features: &Vec<NodePath<ConcreteFeature>>,
    product: &NodePath<ConcreteProduct>,
    context: &CommandContext,
) -> Result<ConflictStatistic, Box<dyn Error>> {
    let checker = ConflictChecker::new(&context.git);
    let mut analyzer = ConflictAnalyzer::new(checker, context);
    let transformed: Vec<NodePath<AnyHasBranch>> = features
        .iter()
        .map(|p| p.try_convert_to().unwrap())
        .collect();
    let matrix = analyzer.calculate_2d_heuristics_matrix_with_merge_base(
        &transformed,
        &product.try_convert_to().unwrap(),
    )?;
    Ok(matrix.calculate_best_path_greedy())
}

fn handle_abort(
    last_state: Option<&DerivationCommit>,
    abort: bool,
    context: &CommandContext,
) -> Result<bool, Box<dyn Error>> {
    match (last_state, abort) {
        (None, true) => Err("No derivation in progress, there is nothing to abort".into()),
        (Some(last_state), true) => match last_state.try_get_metadata().get_state() {
            DerivationState::Finished => {
                Err("No derivation in progress, there is nothing to abort".into())
            }
            _ => {
                context.info("Aborting current derivation process");
                let commit = last_state.try_get_metadata().get_initial_commit();
                context.git.abort_merge()?;
                context.git.reset_hard(commit)?;
                context.info(format!("Reset to state before derivation ({})", commit));
                Ok(true)
            }
        },
        (_, false) => Ok(false),
    }
}

fn handle_continue(
    last_state: Option<&DerivationCommit>,
    continue_derivation: bool,
    optimize: bool,
    context: &mut CommandContext,
) -> Result<bool, Box<dyn Error>> {
    match (last_state, continue_derivation, optimize) {
        (None, true, _) => Err("No derivation in progress, there is nothing to continue".into()),
        (Some(last_state), true, _) => match last_state.try_get_metadata().get_state() {
            DerivationState::Finished => {
                Err("No derivation in progress, there is nothing to continue".into())
            }
            _ => {
                handle_derivation(last_state.try_get_metadata().clone(), context)?;
                Ok(true)
            }
        },
        (Some(last_state), false, false) => match last_state.try_get_metadata().get_state() {
            DerivationState::Starting | DerivationState::InProgress => Err(format!(
                "Derivation incomplete, please use {} to finish it first",
                "tangl derive --continue".purple()
            )
            .into()),
            _ => Ok(false),
        },
        (_, false, _) => Ok(false),
    }
}

fn get_next_state(
    progress: Option<&DerivationCommit>,
    optimization: bool,
    features: &Vec<NodePath<ConcreteFeature>>,
    product: &NodePath<ConcreteProduct>,
    context: &mut CommandContext,
) -> Result<Option<DerivationData>, Box<dyn Error>> {
    let commits = context.git.iter_commit_history(product)?;
    let current_commit = commits[0].clone();
    let mut first = false;
    let mut state = match (progress, optimization, !features.is_empty()) {
        (None, true, false) => {
            return Err("Cannot optimize merge order: No derivation in progress".into());
        }
        (None, _, true) => {
            first = true;
            DerivationData::new_initial(
                FeatureMetadata::from_features(&features),
                current_commit.get_hash(),
            )
        }
        (Some(progress), true, false) => match progress.try_get_metadata().get_state() {
            DerivationState::Finished => {
                return Err("Cannot optimize merge order: No derivation in progress".into());
            }
            _ => progress.try_get_metadata().clone(),
        },
        (Some(progress), _, true) => {
            match progress.try_get_metadata().get_state() {
                DerivationState::Finished => {
                    first = true;
                    DerivationData::new_from_previously_finished(
                        &progress.try_get_metadata(),
                        FeatureMetadata::from_features(&features),
                        current_commit.get_hash(),
                    )
                }
                // handled by continue
                _ => unreachable!(),
            }
        }
        // handled by continue
        _ => unreachable!(),
    };
    if first {
        context
            .git
            .empty_commit(DerivationCommit::make_derivation_message(&state)?.as_str())?;
    }

    let mut original_order: Vec<NodePath<ConcreteFeature>> = vec![];
    for missing in state.get_missing() {
        if let Some(path) = context
            .git
            .get_model()
            .get_node_path::<ConcreteFeature>(&missing.get_qualified_path())
        {
            original_order.push(path);
        } else {
            return Err(format!(
                "Cannot commence derivation: feature {} does not exist in current working tree",
                missing.get_qualified_path().to_string().red()
            )
            .into());
        }
    }
    let original_order_paths: Vec<QualifiedPath> = original_order
        .iter()
        .map(|p| p.to_qualified_path())
        .collect();

    match optimization {
        true => {
            let approximation = approximate_merge_order(&original_order, product, &context)?;
            context.info("Suggesting the following merge order:\n");
            context.info(approximation.display_as_path());

            let new_order = match approximation {
                ConflictStatistic::Success(success) => {
                    context.info("\nExpecting no conflicts");
                    success.paths
                }
                ConflictStatistic::Conflict(conflict) => {
                    context.info(format!(
                        "\nExpecting {} conflicts",
                        conflict.failed_at.len().to_string().red()
                    ));
                    conflict.paths
                }
                ConflictStatistic::Error(error) => return Err(error.error.into()),
            };
            if original_order_paths != new_order[1..].to_vec() {
                state.reorder_missing(&new_order[1..].to_vec());
                state.as_in_progress();
                context
                    .git
                    .empty_commit(DerivationCommit::make_derivation_message(&state)?.as_str())?;
            }
            context.info(format!(
                "\nIf you are satisfied, run {} to commence the derivation",
                "--continue".purple()
            ));
            Ok(None)
        }
        false => Ok(Some(state)),
    }
}

fn handle_derivation(
    mut progress: DerivationData,
    context: &mut CommandContext,
) -> Result<(), Box<dyn Error>> {
    let missing_qualified = progress
        .get_missing()
        .iter()
        .map(|m| m.get_qualified_path())
        .collect::<Vec<QualifiedPath>>();
    let missing = assert_features_exist(&missing_qualified, &context.git)?;
    let mut completed: Vec<QualifiedPath> = Vec::new();
    for path in missing.iter() {
        let result = context.git.merge(&path)?;
        match result.status.success() {
            true => {
                progress.mark_as_completed(&vec![path.to_qualified_path()]);
                completed.push(path.to_qualified_path());
            }
            false => {
                context.git.abort_merge()?;
                break;
            }
        }
    }
    context.info(format!(
        "{} features where merged {}:\n",
        completed.len().to_string().green(),
        "successfully".green(),
    ));
    for path in completed.iter() {
        context.info(format!("  {}", path.to_string().green()))
    }
    match progress.get_missing().len() {
        0 => {
            progress.as_finished();
            context
                .git
                .empty_commit(DerivationCommit::make_derivation_message(&progress)?.as_str())?;
            context.info(format!(
                "\nNo missing features remain; Derivation {}",
                "complete".green(),
            ));
        }
        _ => {
            progress.as_in_progress();
            context
                .git
                .empty_commit(DerivationCommit::make_derivation_message(&progress)?.as_str())?;
            context.info(format!(
                "\n{} {} feature(s) remain(s):\n",
                progress.get_missing().len().to_string().red(),
                "conflicting".red()
            ));
            for data in progress.get_missing().iter() {
                context.info(format!("  {}", data.get_qualified_path().to_string().red()))
            }
            let to_merge = context
                .git
                .get_model()
                .get_node_path::<ConcreteFeature>(&progress.get_missing()[0].get_qualified_path())
                .unwrap();
            context.info(format!(
                "\nNow merging:\n\n   {}",
                to_merge.to_string().yellow(),
            ));
            context.git.merge(&to_merge)?;
            context.info(format!(
                "\nPlease solve all conflicts and commit your changes; thereafter, run {} to continue the derivation",
                "tangl derive --continue".purple()
            ));
            context.info(format!(
                "Use {} to abort the current derivation process",
                "tangl derive --abort".purple()
            ));
        }
    }
    Ok(())
}

fn handle_initialize(features: Vec<NodePath<ConcreteFeature>>, derivation_manager: &mut DerivationManager) -> Result<(), Box<dyn Error>> {
    let state = match derivation_manager.initialize_derivation(&features) {
        Ok(state) => state,
        Err(error) => match error {
            DerivationError::DerivationInProgress => {
                let messages = vec![
                    "A derivation is already in progress".to_string(),
                    format!("  (Use {} to continue the derivation)", format_command_help("tangl derive --continue")),
                    format!("  (Use {} to reset to the last state)", format_command_help("tangl derive --reset")),
                    format!("  (Use {} to abort the derivation)", format_command_help("tangl derive --abort")),
                ];
                return Err(messages.join("\n").into())
            },
            _ => unreachable!(),
        },
    };
    Ok(())
}

fn handle_continue2(derivation_manager: &mut DerivationManager) -> Result<(), Box<dyn Error>> {
    let current = derivation_manager.get_current_state();
    let next = match derivation_manager.continue_derivation() {
        Ok(state) => state,
        Err(error) => match error {
            DerivationError::NoDerivationInProgress => {
                return Err("No derivation in progress, there is nothing to continue".into())
            },
            _ => unreachable!(),
        },
    };
    Ok(())
}

#[derive(Clone, Debug)]
pub struct DeriveCommand;

impl CommandDefinition for DeriveCommand {
    fn build_command(&self) -> Command {
        Command::new("derive")
            .about("Derive a product")
            .disable_help_subcommand(true)
            .arg_required_else_help(true)
            .arg(Arg::new(FEATURES).action(ArgAction::Append))
            .arg(
                Arg::new(CONTINUE)
                    .long("continue")
                    .action(ArgAction::SetTrue)
                    .conflicts_with(FEATURES)
                    .help("Continue the ongoing derivation process"),
            )
            .arg(
                Arg::new(ABORT)
                    .long(ABORT)
                    .action(ArgAction::SetTrue)
                    .exclusive(true)
                    .help("Abort the ongoing derivation process"),
            )
            .arg(
                Arg::new(RESET)
                    .long(RESET)
                    .exclusive(true)
                    .help("Reset the ongoing derivation process to the last state"),
            )
            .arg(
                Arg::new(OPTIMIZATION)
                    .short('o')
                    .long("optimize")
                    .action(ArgAction::SetTrue)
                    .help("Attempt to optimize the order of merges"),
            )
            .arg(verbose())
    }
}

impl CommandInterface for DeriveCommand {
    fn run_command(&self, context: &mut CommandContext) -> Result<(), Box<dyn Error>> {
        let current_area = context.git.get_current_area()?;
        let product_path = context.git.assert_current_node_path::<ConcreteProduct>()?;
        let all_feature_paths = context
            .arg_helper
            .get_argument_values::<String>(FEATURES)
            .unwrap_or(Vec::new())
            .into_iter()
            .map(|e| current_area.get_path_to_feature_root() + QualifiedPath::from(e))
            .collect::<Vec<_>>();
        drop(current_area);
        let continue_derivation = context
            .arg_helper
            .get_argument_value::<bool>(CONTINUE)
            .unwrap();
        let abort_derivation = context
            .arg_helper
            .get_argument_value::<bool>(ABORT)
            .unwrap();
        let optimization = context
            .arg_helper
            .get_argument_value::<bool>(OPTIMIZATION)
            .unwrap();

        let features = context.git.get_model().assert_all(&all_feature_paths)?;
        let mut derivation_manager = DerivationManager::new(&product_path, &context.git)?;
        
        if !features.is_empty() {
            handle_initialize(features, &mut derivation_manager)?;
        } else if continue_derivation {
            todo!()
        } else {
            unreachable!()
        };
        

        let commits = context.git.iter_commit_history(&product_path)?;
        let last_state = commits.get(0);

        if handle_abort(last_state.clone(), abort_derivation, context)? {
            return Ok(());
        }
        if handle_continue(
            last_state.clone(),
            continue_derivation,
            optimization,
            context,
        )? {
            return Ok(());
        }
        let new_state = get_next_state(
            last_state.clone(),
            optimization,
            &features,
            &product_path,
            context,
        )?;
        if new_state.is_none() {
            return Ok(());
        }
        handle_derivation(new_state.unwrap(), context)?;
        Ok(())
    }

    fn shell_complete(
        &self,
        completion_helper: CompletionHelper,
        context: &mut CommandContext,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let maybe_feature_root = context.git.get_current_area()?.move_to_feature_root();
        if maybe_feature_root.is_none() {
            return Ok(vec![]);
        }
        let feature_root = maybe_feature_root.unwrap();
        let feature_root_path = feature_root.to_qualified_path();
        let current = completion_helper.currently_editing();
        let result = match current {
            Some(value) => match value.get_id().as_str() {
                FEATURES => {
                    let to_filter = completion_helper
                        .get_appendix_of(FEATURES)
                        .into_iter()
                        .map(|p| feature_root_path.clone() + QualifiedPath::from(p))
                        .collect();
                    let transformer = ChainingNodePathTransformer::new(vec![
                        NodePathTransformers::HasBranchFilteringNodePathTransformer(
                            HasBranchFilteringNodePathTransformer::new(true),
                        ),
                        NodePathTransformers::ByQPathFilteringNodePathTransformer(
                            ByQPathFilteringNodePathTransformer::new(
                                to_filter,
                                FilteringMode::EXCLUDE,
                            ),
                        ),
                    ]);
                    completion_helper.complete_qualified_paths(
                        feature_root.to_qualified_path(),
                        transformer
                            .transform(feature_root.iter_children_req())
                            .map(|path| path.to_qualified_path()),
                    )
                }
                _ => vec![],
            },
            None => vec![],
        };
        Ok(result)
    }
}
