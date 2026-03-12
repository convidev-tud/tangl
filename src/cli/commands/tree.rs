use crate::cli::completion::CompletionHelper;
use crate::cli::*;
use crate::model::{AnyNode, ToQualifiedPath};
use clap::{Arg, ArgAction, Command};
use colored::Colorize;
use itertools::Itertools;
use std::error::Error;

const TARGET: &str = "target";
const TREE: &str = "tree";

#[derive(Clone, Debug)]
pub struct LSCommand;

impl CommandDefinition for LSCommand {
    fn build_command(&self) -> Command {
        Command::new("ls")
            .about("Displays the tree structure")
            .disable_help_subcommand(true)
            .arg(Arg::new(TARGET).default_value("."))
            .arg(
                Arg::new(TREE)
                    .short('t')
                    .long("tree")
                    .action(ArgAction::SetTrue)
                    .help("Displays the tree structure under the current path"),
            )
            .arg(show_tags())
    }
}

impl CommandInterface for LSCommand {
    fn run_command(&self, context: &mut CommandContext) -> Result<(), Box<dyn Error>> {
        let current = context.git.get_current_qualified_path()?;
        let mut target = current
            + context
                .arg_helper
                .get_argument_value::<String>(TARGET)
                .unwrap()
                .to_qualified_path();
        if target.is_dir() {
            target = target.strip_n_right(target.len()-1)
        }
        let show_tags = context
            .arg_helper
            .get_argument_value::<bool>(SHOW_TAGS)
            .unwrap();
        let tree = context.arg_helper.get_argument_value::<bool>(TREE).unwrap();
        let node_path = context.git.get_model().assert_path::<AnyNode>(&target)?;
        match tree {
            true => {
                let tree = node_path.display_tree(show_tags);
                context.info(tree.trim());
            }
            false => {
                for child in node_path.iter_children().sorted() {
                    let mut name = child.to_qualified_path().last().unwrap().clone();
                    if child.get_metadata().has_branch() {
                        name = name.blue().to_string()
                    }
                    let node_type = child.get_actual_type().get_formatted_name();
                    if child.has_children() {
                        name += "/...".blue().to_string().as_str();
                    }
                    context.info(format!("{name} [{node_type}]"))
                }
            }
        }
        Ok(())
    }

    fn shell_complete(
        &self,
        completion_helper: CompletionHelper,
        context: &mut CommandContext,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let completion: Vec<String> = if let Some(editing) = completion_helper.currently_editing() {
            match editing.get_id().as_str() {
                TARGET => {
                    let current = context.git.get_current_qualified_path()?;
                    let root = context.git.get_model().get_virtual_root();
                    completion_helper.complete_qualified_paths(
                        current,
                        root.iter_children_req().map(|p|p.to_qualified_path()),
                    )
                }
                _ => vec![],
            }
        } else {
            vec![]
        };
        Ok(completion)
    }
}
