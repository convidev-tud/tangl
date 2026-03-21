use crate::cli::*;
use crate::model::{AnyHasBranch, ConcreteProduct};
use crate::spl::{DerivationState, InspectionManager};
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
        let mut no_first_line = output.split("\n").collect::<Vec<_>>()[1..]
            .to_vec()
            .join("\n")
            .trim()
            .to_string();
        let current_path = context.git.assert_current_node_path::<AnyHasBranch>()?;
        let first_line = format!(
            "On {} branch {}",
            current_path.get_actual_type().get_formatted_name(),
            current_path.to_string().blue()
        );
        context.info(first_line);

        if let Some(product) = current_path.try_convert_to::<ConcreteProduct>() {
            let inspector = InspectionManager::new(&context.git);
            let state = inspector.get_current_derivation_state(&product)?;
            match state.get_state() {
                DerivationState::None => context.info("No derivation in progress"),
                DerivationState::InProgress => {
                    context.info("\nDerivation in progress");
                    if !state.get_completed().is_empty() {
                        context.info("\nFeatures merged:");
                        for feature in state.get_completed() {
                            context.info(format!(
                                "    {}",
                                feature.get_qualified_path().to_string().green()
                            ));
                        }
                    }
                    if !state.get_missing().is_empty() {
                        context.info("\nFeatures remaining:");
                        for feature in state.get_missing() {
                            context.info(format!(
                                "    {}",
                                feature.get_qualified_path().to_string().red()
                            ));
                        }
                    }
                    if no_first_line.contains("You have unmerged paths.") {
                        context.info("\nCurrently merging:");
                        context.info(format!(
                            "    {}",
                            state
                                .get_missing()
                                .first()
                                .unwrap()
                                .get_qualified_path()
                                .to_string()
                                .yellow()
                        ));
                        no_first_line += format!(
                            "\nWhen all conflicts are fixed, run {} to continue the derivation.",
                            format_command_help("tangl derive --continue")
                        ).as_str();
                    } else {
                        no_first_line += format!(
                            "\nRun {} to continue the derivation.",
                            format_command_help("tangl derive --continue")
                        )
                        .as_str();
                    }
                    context.info("");
                }
            }
        };

        context.info(no_first_line);
        Ok(())
    }
}
