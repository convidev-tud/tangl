use crate::cli::CommandContext;
use crate::model::{IsGitObject, NormalizedPath};
use clap::{Arg, ArgAction};
use colored::Colorize;
use std::error::Error;

pub const VERBOSE: &str = "verbose";
pub const SHOW_TAGS: &str = "show_tags";

pub fn show_tags() -> Arg {
    Arg::new(SHOW_TAGS)
        .long("show-tags")
        .action(ArgAction::SetTrue)
        .help("Also show tags")
}

pub fn delete(force: bool) -> Arg {
    let short = if force { 'D' } else { 'd' };
    Arg::new("delete").short(short)
}

pub fn verbose() -> Arg {
    Arg::new(VERBOSE)
        .short('v')
        .long("verbose")
        .action(ArgAction::Count)
        .help(
            "Set verbosity of output. \
            Verbosity increases with number of occurrences.",
        )
}

pub fn format_command_help<S: Into<String>>(command: S) -> String {
    format!("\"{}\"", command.into())
}

pub fn delete_path<T: IsGitObject>(
    path: &NormalizedPath,
    context: &mut CommandContext,
) -> Result<(), Box<dyn Error>> {
    match context.git.assert_path::<T>(&path) {
        Ok(concrete_path) => {
            let concrete_type = concrete_path.get_actual_type().clone();
            context.git.delete_branch(concrete_path)?;
            context.logger.info(format!(
                "Deleted {} branch {}",
                concrete_type.get_formatted_name(),
                path.to_string().blue()
            ));
        }
        Err(error) => {
            return Err(error.into());
        }
    };
    Ok(())
}
