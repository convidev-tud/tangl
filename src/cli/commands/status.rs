use crate::cli::*;
use crate::model::{AnyGitObject, ConcreteProduct};
use crate::spl::{DerivationManager, DerivationState, InspectionManager};
use clap::Command;
use colored::Colorize;
use std::error::Error;

#[derive(Clone, Debug)]
pub struct StatusCommand;

impl CommandDefinition for StatusCommand {
    fn build_command(&self) -> Command {
        Command::new("status")
            .about("Shows details of a run")
            .after_help("More detail")
            .disable_help_subcommand(true)
    }
}

impl CommandInterface for StatusCommand {
    fn run_command(&self, context: &mut CommandContext) -> Result<(), Box<dyn Error>> {
        context.git.colored_output(true);
        let output = context.git.status()?;
        let no_first_line = output.split("\n").collect::<Vec<_>>()[1..]
            .to_vec()
            .join("\n")
            .trim()
            .to_string();
        let current_path = context.git.assert_current_node_path::<AnyGitObject>()?;
        let first_line = format!(
            "On {} branch {}",
            current_path.get_actual_type().get_formatted_name(),
            current_path.to_string().blue()
        );
        context.logger.info(first_line);
        let mut maybe_new_line = "";
        if let Some(product) = current_path.try_convert_to::<ConcreteProduct>() {
            let inspector = InspectionManager::new(&context.git);
            let state = inspector.get_last_derivation_state(&product)?;
            match state.get_state() {
                DerivationState::None => context.logger.info("No derivation in progress"),
                DerivationState::InProgress => {
                    context.logger.info("Derivation in progress");
                    if no_first_line.contains("You have unmerged paths.") {
                        context.logger.info(conflict_hint());
                    } else {
                        context.logger.info(normal_hint());
                    }
                    if !state.get_missing().is_empty() {
                        let manager =
                            DerivationManager::new(&product, &context.git, &context.logger)?;
                        let missing = manager.get_pending_chain()?;
                        context.logger.info("\nFeatures remaining:");
                        for info in missing.display_as_list() {
                            context.logger.info(format!("  {info}"))
                        }
                    }
                    maybe_new_line = "\n";
                }
            }
        };
        context
            .logger
            .info(format!("{maybe_new_line}{no_first_line}"));
        Ok(())
    }
}
