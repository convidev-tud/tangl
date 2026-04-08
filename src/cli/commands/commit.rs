use crate::cli::*;
use crate::git::conflict::{CheckMode, ConflictChecker, MergeChainStatistics};
use crate::model::{
    AnyGitObject, CommitMetadataContainer, ConcreteFeature, ConcreteProduct, NodePath,
};
use crate::spl::{DerivationMetadata, DerivationState, InspectionManager};
use clap::{Arg, Command};
use colored::Colorize;
use std::error::Error;

const MESSAGE: &str = "message";

fn handle_feature(
    feature: &NodePath<ConcreteFeature>,
    inspector: &InspectionManager,
    context: &CommandContext,
) -> Result<Option<CommitMetadataContainer>, Box<dyn Error>> {
    let checker = ConflictChecker::new(&context.git, CheckMode::Merge);
    let area = context.git.get_current_area()?;
    let all_features: Vec<NodePath<ConcreteFeature>> = area
        .clone()
        .move_to_feature_root()
        .unwrap()
        .iter_features_req()
        .filter_map(|f| {
            if &f != feature {
                f.try_convert_to()
            } else {
                None
            }
        })
        .collect();
    let all_products = inspector.find_products_containing_feature(&feature)?;

    let feature_statistics: MergeChainStatistics<ConcreteFeature, ConcreteFeature> = all_features
        .iter()
        .map(|f| {
            checker
                .check_permutations_against_base(f, &vec![], &vec![feature.clone()], 1)
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect::<Result<_, _>>()?;

    if feature_statistics.n_conflicts() > 0 {
        context.logger.warn(format!(
            "\nWarning: Feature stands in conflict with {} other feature(s)",
            feature_statistics.n_conflicts().to_string().red()
        ));
        for conflict in feature_statistics.iter_conflicts() {
            for line in conflict.display_as_list() {
                context
                    .logger
                    .warn(format!("  {line}"));
            }
        }
    }
    for error in feature_statistics.iter_errors() {
        context
            .logger
            .warn(format!("  {}", error.display_as_path()));
    }

    let product_statistics: MergeChainStatistics<ConcreteProduct, ConcreteFeature> = all_products
        .iter()
        .map(|product| {
            checker
                .check_permutations_against_base(product, &vec![], &vec![feature.clone()], 1)
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect::<Result<_, _>>()?;

    if product_statistics.n_conflicts() > 0 {
        context.logger.warn(format!(
            "\nWarning: Feature stands in conflict with {} product(s) derived from it",
            product_statistics.n_conflicts().to_string().red()
        ));
        for conflict in product_statistics.iter_conflicts() {
            for line in conflict.display_as_list() {
                context
                    .logger
                    .warn(format!("  {line}"));
            }
        }
    }
    for error in product_statistics.iter_errors() {
        context
            .logger
            .warn(format!("  {}", error.display_as_path()));
    }
    Ok(None)
}

fn handle_product(
    product: &NodePath<ConcreteProduct>,
    inspector: &InspectionManager,
    context: &CommandContext,
) -> Result<Option<CommitMetadataContainer>, Box<dyn Error>> {
    let last_commit = inspector.get_last_derivation_commit(&product)?;
    if let Some(state) = last_commit {
        match state.get_metadata().get_data().unwrap().get_state() {
            DerivationState::None => {
                context.logger.info(
                    "Hint: You commited to a product branch containing features."
                        .yellow()
                        .to_string(),
                );
                context.logger.info(format!(
                    "{} {} {}",
                    "Hint: Use".yellow(),
                    format_command_help("tangl untie"),
                    "to copy the commit onto a feature.".yellow(),
                ));
            }
            _ => {}
        }
        let new_pointer = DerivationMetadata::new(state.get_commit().get_hash().clone(), None);
        Ok(Some(CommitMetadataContainer::new(&new_pointer)?))
    } else {
        Ok(None)
    }
}

#[derive(Clone, Debug)]
pub struct CommitCommand;

impl CommandDefinition for CommitCommand {
    fn build_command(&self) -> Command {
        Command::new("commit")
            .about("Make git commit")
            .disable_help_subcommand(true)
            .arg(Arg::new(MESSAGE).short('m').help("Commit message"))
    }
}

impl CommandInterface for CommitCommand {
    fn run_command(&self, context: &mut CommandContext) -> Result<(), Box<dyn Error>> {
        let maybe_message = context.arg_helper.get_argument_value::<String>(MESSAGE);
        let current = context.git.assert_current_node_path::<AnyGitObject>()?;
        context.git.colored_output(true);
        let inspector = InspectionManager::new(&context.git);
        let metadata: Option<CommitMetadataContainer> =
            if let Some(product) = current.try_convert_to::<ConcreteProduct>() {
                handle_product(&product, &inspector, &context)?
            } else {
                None
            };
        match maybe_message {
            Some(message) => {
                context
                    .git
                    .commit::<_, AnyGitObject>(&message, metadata.as_ref(), false, false)?
            }
            None => todo!(),
        };
        if let Some(feature) = current.try_convert_to::<ConcreteFeature>() {
            handle_feature(&feature, &inspector, &context)?;
        }
        Ok(())
    }
}
