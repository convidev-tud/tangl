use crate::cli::completion::*;
use crate::cli::*;
use crate::git::conflict::{MergeChainStatistic, MergeResult};
use crate::logging::TanglLogger;
use crate::model::*;
use crate::spl::*;
use clap::{Arg, ArgAction, Command};
use colored::Colorize;
use std::error::Error;
use std::fs::File;
use std::io::Read;

const FEATURES: &str = "features";
const CONTINUE: &str = "continue";
const ABORT: &str = "abort";
const RESET: &str = "reset";
const OPTIMIZE: &str = "optimize";
const UPDATE: &str = "update";
const FROM_FILE: &str = "from_file";

fn fix_conflicts_hint() -> String {
    format!(
        "  (Fix all conflicts, then use {} to continue the derivation)",
        format_command_help("tangl derive --continue")
    )
}

fn continue_hint() -> String {
    format!(
        "  (Use {} to continue the derivation)",
        format_command_help("tangl derive --continue")
    )
}

fn reset_hint() -> String {
    format!(
        "  (Use {} to reset to the last state)",
        format_command_help("tangl derive --reset")
    )
}

fn abort_hint() -> String {
    format!(
        "  (Use {} to abort the derivation)",
        format_command_help("tangl derive --abort")
    )
}

pub fn normal_hint() -> String {
    format!(
        "  {}\n  {}\n  {}",
        continue_hint(),
        reset_hint(),
        abort_hint(),
    )
}

pub fn conflict_hint() -> String {
    format!(
        "  {}\n  {}\n  {}",
        fix_conflicts_hint(),
        reset_hint(),
        abort_hint(),
    )
}

fn initialize_hint(
    state: &DerivationState,
    optimize: bool,
    derivation_manager: &mut DerivationManager,
    logger: &TanglLogger,
) -> Result<(), Box<dyn Error>> {
    match state {
        DerivationState::InProgress => {
            let order: MergeChainStatistic = derivation_manager.get_pending_chain()?;
            logger.info("Derivation Preview\n");
            if optimize {
                logger.info("Suggesting the following merge order:");
            }
            for info in order.display_as_list() {
                logger.info(format!("  {}", info));
            }
            if order.contains_conflicts() {
                logger.info(format!(
                    "\nExpecting {} conflict(s)",
                    order.get_n_conflict().to_string().red()
                ))
            } else {
                logger.info("\nExpecting no conflicts")
            };
            logger.info(continue_hint());
            logger.info(abort_hint());
        }
        DerivationState::None => logger.info("Product already up to date."),
    }
    Ok(())
}

fn initialize_error_hint() -> Box<dyn Error> {
    let messages = vec![
        "fatal: a derivation is already in progress".to_string(),
        normal_hint(),
    ];
    messages.join("\n").into()
}

fn handle_initialize(
    features: Vec<NodePath<ConcreteFeature>>,
    optimize: bool,
    derivation_manager: &mut DerivationManager,
    logger: &TanglLogger,
) -> Result<(), Box<dyn Error>> {
    match derivation_manager.initialize_derivation(features, optimize) {
        Ok(state) => {
            initialize_hint(state.get_state(), optimize, derivation_manager, logger)?;
            Ok(())
        }
        Err(error) => match error {
            InitializeDerivationError::DerivationInProgress => Err(initialize_error_hint()),
            _ => Err(error.into()),
        },
    }
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
    let completed: Vec<MergeResult> = next
        .get_completed()
        .iter()
        .filter_map(|data| {
            if !old.get_completed().contains(data) {
                Some(data.clone())
            } else {
                None
            }
        })
        .collect();
    let mut completed_chain = MergeChainStatistic::new();
    completed_chain.push(MergeResult::Base(derivation_manager.get_product().to_normalized_path()));
    completed_chain.fill(completed);
    let still_missing: MergeChainStatistic = derivation_manager.get_pending_chain()?;
    logger.info(format!("Merged {} feature(s)", completed_chain.len() - 1));
    for complete in completed_chain.iter() {
        logger.info(format!("  {}", complete));
    }
    if !still_missing.is_empty() {
        logger.info("\nEncountered conflicts while merging");
        logger.info(conflict_hint());
        logger.info(format!(
            "\n{} feature(s) remain(s)",
            still_missing.get_chain().len() - 1
        ));
        for info in still_missing.display_as_list() {
            logger.info(format!("  {}", info));
        }
    } else {
        logger.info("\nAll features merged. Derivation complete.")
    }
    Ok(())
}

fn handle_update(
    optimize: bool,
    derivation_manager: &mut DerivationManager,
    logger: &TanglLogger,
) -> Result<(), Box<dyn Error>> {
    match derivation_manager.update_product(optimize) {
        Ok(state) => {
            initialize_hint(state.get_state(), optimize, derivation_manager, logger)?;
            Ok(())
        }
        Err(error) => match error {
            UpdateProductError::DerivationInProgress => Err(initialize_error_hint()),
            _ => Err(error.into()),
        },
    }
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
            .args(vec![
                Arg::new(CONTINUE)
                    .long("continue")
                    .action(ArgAction::SetTrue)
                    .conflicts_with_all(vec![FEATURES, OPTIMIZE, UPDATE])
                    .help("Continue the ongoing derivation process"),
                Arg::new(ABORT)
                    .long(ABORT)
                    .action(ArgAction::SetTrue)
                    .exclusive(true)
                    .help("Abort the ongoing derivation process"),
                Arg::new(RESET)
                    .long(RESET)
                    .exclusive(true)
                    .help("Reset the ongoing derivation process to the last state"),
                Arg::new(OPTIMIZE)
                    .short('o')
                    .long(OPTIMIZE)
                    .action(ArgAction::SetTrue)
                    .help("Attempt to optimize the order of merges"),
                Arg::new(UPDATE)
                    .short('u')
                    .long(UPDATE)
                    .conflicts_with_all(vec![FEATURES, FROM_FILE])
                    .action(ArgAction::SetTrue)
                    .help("Updates product with newest commits of contained features"),
                Arg::new(FROM_FILE)
                    .short('f')
                    .long("from-file")
                    .conflicts_with_all(vec![FEATURES, UPDATE])
                    .help("Get features from external product configuration"),
            ])
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
            .map(|e| current_area.get_path_to_feature_root() + NormalizedPath::from(e))
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
        let update = context
            .arg_helper
            .get_argument_value::<bool>(UPDATE)
            .unwrap();
        let file_path = context.arg_helper.get_argument_value::<String>(FROM_FILE);

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
        } else if update {
            handle_update(optimize, &mut derivation_manager, &context.logger)?;
        } else if let Some(file_path) = file_path {
            let feature_root = context.git.get_current_area()?.get_path_to_feature_root();
            let mut file = File::open(file_path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            let parser = ModelParser::new(&context.import_format);
            let paths = parser
                .import(&content)?
                .into_iter()
                .map(|p| feature_root.clone() + p)
                .collect();
            let features = context.git.get_model().assert_all::<Feature>(&paths)?;
            let transformer = ByTypeFilteringNodePathTransformer::<_, ConcreteFeature>::new();
            let node_paths = transformer.transform(features.into_iter()).collect();
            handle_initialize(
                node_paths,
                optimize,
                &mut derivation_manager,
                &context.logger,
            )?;
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
        let feature_root_path = feature_root.to_normalized_path();
        let current = completion_helper.currently_editing();
        let result = match current {
            Some(value) => match value.get_id().as_str() {
                FEATURES => {
                    let to_filter = completion_helper
                        .get_appendix_of(FEATURES)
                        .into_iter()
                        .map(|p| feature_root_path.clone() + NormalizedPath::from(p))
                        .collect();
                    let transformer = GlobToTypeNodePathTransformer::<_, AnyGitObject>::new(
                        &to_filter,
                        FilteringMode::EXCLUDE,
                    )?;
                    completion_helper.complete_qualified_paths(
                        feature_root.to_normalized_path(),
                        transformer
                            .transform(feature_root.iter_children_req())
                            .map(|path| path.to_normalized_path()),
                    )
                }
                _ => vec![],
            },
            None => vec![],
        };
        Ok(result)
    }
}
