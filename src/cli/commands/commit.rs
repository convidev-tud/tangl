use crate::cli::*;
use crate::git::conflict::{ConflictChecker, ConflictStatistics};
use crate::model::{AnyHasBranch, ConcreteFeature, ConcreteProduct, FeatureMetadata, NodePath, ToQualifiedPath};
use clap::{Arg, Command};
use colored::Colorize;
use std::error::Error;

const MESSAGE: &str = "message";

#[derive(Clone, Debug)]
pub struct CommitCommand;

impl CommandDefinition for CommitCommand {
    fn build_command(&self) -> Command {
        Command::new("commit")
            .about("Make git commit")
            .disable_help_subcommand(true)
            .arg(
                Arg::new(MESSAGE)
                    .short('m')
                    .help("Commit message")
            )
    }
}

impl CommandInterface for CommitCommand {
    fn run_command(&self, context: &mut CommandContext) -> Result<(), Box<dyn Error>> {
        let maybe_message = context.arg_helper.get_argument_value::<String>(MESSAGE);
        let current = context.git.get_current_node_path::<AnyHasBranch>()?.unwrap();
        context.git.colored_output(true);
        let out = match maybe_message {
            Some(message) => context.git.commit(&message)?,
            None => context.git.interactive_commit()?,
        };
        context.log_from_output(&out);
        if let Some(feature) = current.try_convert_to::<ConcreteFeature>() {
            let checker = ConflictChecker::new(&context.git);
            let n = feature.try_convert_to::<AnyHasBranch>().unwrap();
            let area = context.git.get_current_area()?;
            let all_features: Vec<NodePath<AnyHasBranch>> = area
                .clone()
                .move_to_feature_root()
                .unwrap()
                .iter_features_req()
                .filter_map(|feature| {
                    if feature != n {
                        feature.try_convert_to::<AnyHasBranch>()
                    } else { None }
                })
                .collect();
            let all_products: Vec<NodePath<AnyHasBranch>> = if let Some(pr) = area.move_to_product_root() {
                pr
                    .iter_products_req()
                    .filter_map(|p| {
                        if let Some(concrete) = p.try_convert_to::<ConcreteProduct>() {
                            let derivation_commits = context.git.get_derivation_commits(&concrete).unwrap();
                            if derivation_commits.is_empty() {
                                None
                            } else {
                                let last = derivation_commits.first().unwrap();
                                let features = FeatureMetadata::qualified_paths(last.get_metadata().get_total());
                                if features.contains(&feature.to_qualified_path()) {
                                    concrete.try_convert_to::<AnyHasBranch>()
                                } else { None }
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
            } else { vec![] };
            let feature_statistics: ConflictStatistics = checker.check_n_against_permutations(&vec![n.clone()], &all_features, &1).collect();
            if feature_statistics.n_conflicts() > 0 {
                context.warn("Feature stands in conflict with other features");
                for conflict in feature_statistics.iter_conflicts() {
                    context.warn(conflict.display_as_path());
                }
            }
            let product_statistics: ConflictStatistics = checker.check_n_against_permutations(&vec![n], &all_products, &1).collect();
            if product_statistics.n_conflicts() > 0 {
                context.warn("\nFeature stands in conflict with products derived from it");
                for conflict in product_statistics.iter_conflicts() {
                    context.warn(conflict.display_as_path());
                }
            }
        } else if let Some(_) = current.try_convert_to::<ConcreteProduct>() {
            context.info("Hint: You commited to a product branch".yellow().to_string());
            context.info(format!(
                "{} {} {}",
                "Use".yellow(),
                format_command_help("tangl untie"),
                "to copy the commit onto a feature".yellow(),
            ));
        }
        Ok(())
    }
}
