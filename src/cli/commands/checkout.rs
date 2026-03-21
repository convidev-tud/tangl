use crate::cli::completion::*;
use crate::cli::*;
use crate::model::{AnyHasBranch, ModelError, QualifiedPath, ToQualifiedPath};
use clap::{Arg, Command};
use colored::Colorize;
use std::error::Error;

#[derive(Clone, Debug)]
pub struct CheckoutCommand;
impl CommandDefinition for CheckoutCommand {
    fn build_command(&self) -> Command {
        Command::new("checkout")
            .about("Switch branches")
            .disable_help_subcommand(true)
            .arg(Arg::new("branch").required(true))
    }
}
impl CommandInterface for CheckoutCommand {
    fn run_command(&self, context: &mut CommandContext) -> Result<(), Box<dyn Error>> {
        let branch = context
            .arg_helper
            .get_argument_value::<String>("branch")
            .unwrap();
        let full_target = context.git.get_current_qualified_path()? + QualifiedPath::from(branch);
        let node_path = match context
            .git
            .get_model()
            .assert_path::<AnyHasBranch>(&full_target)
        {
            Ok(node_path) => node_path,
            Err(error) => {
                return match error {
                    ModelError::PathNotFound(_) => Err(format!(
                        "Cannot checkout {}: path does not exist",
                        full_target.to_string()
                    )
                    .into()),
                    ModelError::WrongNodeType(_) => Err(format!(
                        "Cannot checkout {}: target does not support branches",
                        full_target
                    )
                    .into()),
                };
            }
        };
        let current = context.git.get_current_qualified_path()?;
        let out = context.git.checkout(&node_path)?;
        if current == node_path.to_qualified_path() {
            context.info(format!(
                "Already on branch {}",
                node_path.to_string().blue(),
            ));
        } else {
            context.info(format!(
                "Switched to {} branch {}",
                node_path.get_actual_type().get_formatted_name(),
                node_path.to_string().blue(),
            ));
            let rest = out.split("\n").collect::<Vec<&str>>()[1..]
                .join("\n")
                .trim()
                .to_string();
            context.info(rest)
        }
        Ok(())
    }
    fn shell_complete(
        &self,
        completion_helper: CompletionHelper,
        context: &mut CommandContext,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let maybe_editing = completion_helper.currently_editing();
        if maybe_editing.is_none() {
            return Ok(vec![]);
        }
        let all_branches = context.git.get_model().get_qualified_paths_with_branches();
        let result = match maybe_editing.unwrap().get_id().as_str() {
            "branch" => completion_helper.complete_qualified_paths(
                context.git.get_current_qualified_path()?,
                all_branches.iter().map(|path| path.clone()),
            ),
            _ => vec![],
        };
        Ok(result)
    }
}
