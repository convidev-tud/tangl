use crate::cli::completion::*;
use crate::cli::*;
use crate::git::error::{InvalidPathError, PathAssertionError};
use crate::model::{
    AnyGitObject, ByTypeFilteringNodePathTransformer, NodePathTransformer, NormalizedPath,
    ToNormalizedPath,
};
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
        let full_target = context.git.get_current_normalized_path()? + NormalizedPath::from(branch);
        let node_path = match context.git.assert_path::<AnyGitObject>(&full_target) {
            Ok(node_path) => node_path,
            Err(error) => {
                return match error {
                    PathAssertionError::InvalidPath(error) => match error {
                        InvalidPathError::PathNotFound(_) => Err(format!(
                            "Cannot checkout {}: path does not exist",
                            full_target.to_string()
                        )
                        .into()),
                        InvalidPathError::WrongNodeType(_) => Err(format!(
                            "Cannot checkout {}: target does not support branches",
                            full_target
                        )
                        .into()),
                        _ => Err(error.into()),
                    },
                    _ => Err(error.into()),
                };
            }
        };
        let current = context.git.get_current_normalized_path()?;
        let out = context.git.checkout(&node_path)?;
        if current == node_path.to_normalized_path() {
            context.logger.info(format!(
                "Already on branch {}",
                node_path.to_string().blue(),
            ));
        } else {
            context.logger.info(format!(
                "Switched to {} branch {}",
                node_path.get_actual_type().get_formatted_name(),
                node_path.to_string().blue(),
            ));
        }
        let rest = out
            .split("\n")
            .filter(|s| !s.contains("Switched") && !s.contains("Already"))
            .collect::<Vec<&str>>()
            .join("\n")
            .trim()
            .to_string();
        context.logger.info(rest);
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
        let transformer = ByTypeFilteringNodePathTransformer::<_, AnyGitObject>::new();
        let root = context.git.get_virtual_root();
        let all_branches = transformer.transform(root.iter_children_req());
        let result = match maybe_editing.unwrap().get_id().as_str() {
            "branch" => completion_helper.complete_normalized_paths(
                context.git.get_current_normalized_path()?,
                all_branches.map(|p| p.to_normalized_path()),
            ),
            _ => vec![],
        };
        Ok(result)
    }
}
