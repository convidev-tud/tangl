use crate::cli::completion::*;
use crate::cli::*;
use crate::git::conflict::MergeChainStatistic;
use crate::logging::TanglLogger;
use crate::model::*;
use crate::spl::*;
use clap::{Arg, ArgAction, Command};
use colored::Colorize;
use std::error::Error;

const FEATURES: &str = "features";
const CONTINUE: &str = "continue";
const ABORT: &str = "abort";
const RESET: &str = "reset";
const OPTIMIZE: &str = "optimize";

pub fn fix_conflicts_hint() -> String {
    format!(
        "\nFix all conflicts, then run {} to commence the derivation.",
        format_command_help("tangl derive --continue"),
    )
}

fn handle_initialize(
    features: Vec<NodePath<ConcreteFeature>>,
    optimize: bool,
    derivation_manager: &mut DerivationManager,
    logger: &TanglLogger,
) -> Result<(), Box<dyn Error>> {
    match derivation_manager.initialize_derivation(features, optimize) {
        Ok(state) => state,
        Err(error) => {
            return match error {
                InitializeDerivationError::DerivationInProgress => {
                    let messages = vec![
                        "fatal: a derivation is already in progress".to_string(),
                        format!(
                            "  (Use {} to continue the derivation)",
                            format_command_help("tangl derive --continue")
                        ),
                        format!(
                            "  (Use {} to reset to the last state)",
                            format_command_help("tangl derive --reset")
                        ),
                        format!(
                            "  (Use {} to abort the derivation)",
                            format_command_help("tangl derive --abort")
                        ),
                    ];
                    Err(messages.join("\n").into())
                }
                _ => Err(error.into()),
            };
        }
    };
    let order: MergeChainStatistic = derivation_manager.get_pending_chain()?;
    logger.info("Derivation Preview\n");
    if optimize {
        logger.info("Suggesting the following merge order:");
    }
    logger.info(order.display_as_path());
    if order.contains_conflicts() {
        logger.info(format!("\nExpecting {} conflicts", order.get_n_conflict()))
    } else {
        logger.info("\nExpecting no conflicts")
    }
    Ok(())
}

fn handle_continue(
    derivation_manager: &mut DerivationManager,
    logger: &TanglLogger,
) -> Result<(), Box<dyn Error>> {
    let old = derivation_manager.get_current_state();
    let next = match derivation_manager.continue_derivation() {
        Ok(state) => state,
        Err(error) => {
            return match error {
                ContinueDerivationError::NoDerivationInProgress => {
                    Err("No derivation in progress, there is nothing to continue".into())
                }
                _ => Err(error.into()),
            };
        }
    };
    let completed: Vec<QualifiedPath> = next
        .get_completed()
        .iter()
        .filter_map(|data| {
            if !old.get_completed().contains(data) {
                Some(data.get_qualified_path())
            } else {
                None
            }
        })
        .collect();
    let still_missing: MergeChainStatistic = next.get_missing().into();
    logger.info(format!("Merged {} feature(s)", completed.len()));
    for complete in completed {
        logger.info(format!("  {}", complete.to_string().green()));
    }
    if !still_missing.get_chain().is_empty() {
        logger.info(format!(
            "\n{} feature(s) remain(s)",
            still_missing.get_chain().len()
        ));
        logger.info(still_missing.display_as_path());
        logger.info(fix_conflicts_hint());
    } else {
        logger.info("\nAll features merged. Derivation complete")
    }
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
                    .conflicts_with_all(vec![FEATURES, OPTIMIZE])
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
                Arg::new(OPTIMIZE)
                    .short('o')
                    .long(OPTIMIZE)
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
        let optimize = context
            .arg_helper
            .get_argument_value::<bool>(OPTIMIZE)
            .unwrap();

        let features = context.git.get_model().assert_all(&all_feature_paths)?;
        let mut derivation_manager =
            DerivationManager::new(&product_path, &context.git, &context.logger)?;

        if abort_derivation {
            let state = derivation_manager.abort_derivation()?;
            context.logger.info(format!(
                "Successfully aborted derivation {}",
                state.get_id()
            ));
            context.logger.info(format!(
                "Reset to state before derivation ({})",
                state.get_initial_commit()
            ));
            return Ok(());
        } else if !features.is_empty() {
            handle_initialize(features, optimize, &mut derivation_manager, &context.logger)?;
        } else if continue_derivation {
            handle_continue(&mut derivation_manager, &context.logger)?;
        } else {
            unreachable!()
        };
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
                    let transformer = GlobToTypeNodePathTransformer::<_, AnyHasBranch>::new(
                        &to_filter,
                        FilteringMode::EXCLUDE,
                    )?;
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
