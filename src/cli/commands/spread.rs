use crate::cli::*;
use crate::model::{AnyGitObject, ByTypeFilteringNodePathTransformer, NodePathTransformer};
use clap::Command;
use colored::Colorize;
use std::error::Error;

#[derive(Clone, Debug)]
pub struct SpreadCommand;

impl CommandDefinition for SpreadCommand {
    fn build_command(&self) -> Command {
        Command::new("spread")
            .about("Spread commits across children")
            .disable_help_subcommand(true)
    }
}

impl CommandInterface for SpreadCommand {
    fn run_command(&self, context: &mut CommandContext) -> Result<(), Box<dyn Error>> {
        let current = context.git.assert_current_node_path::<AnyGitObject>()?;
        let type_filter = ByTypeFilteringNodePathTransformer::<_, AnyGitObject>::new();
        context.logger.info("Spreading commits to children");
        for child in type_filter.transform(current.iter_children_req()) {
            context.git.checkout(&child)?;
            let (result, _) = context.git.merge::<AnyGitObject, _>(current.clone())?;
            context.logger.info(result.display_as_path());
            if result.contains_conflicts() {
                return Err(format!(
                    "Encountered conflict while spreading to {}\n\
                        Fix all conflicts, then rerun the spread command from {}",
                    child.to_string().blue(),
                    current.to_string().blue(),
                )
                .into());
            }
        }
        context.git.checkout(&current)?;
        context.logger.info("Spreading completed without conflicts");
        Ok(())
    }
}
