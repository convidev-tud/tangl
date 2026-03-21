use crate::cli::*;
use clap::Command;
use std::error::Error;

#[derive(Clone, Debug)]
pub struct InitCommand;

impl CommandDefinition for InitCommand {
    fn build_command(&self) -> Command {
        Command::new("init")
            .about("Initialize a repository")
            .disable_help_subcommand(true)
    }
}

impl CommandInterface for InitCommand {
    fn run_command(&self, context: &mut CommandContext) -> Result<(), Box<dyn Error>> {
        let output = context.git.initialize_repo()?;
        context.info(output);
        Ok(())
    }
}
