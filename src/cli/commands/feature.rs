use crate::cli::completion::*;
use crate::cli::*;
use crate::model::*;
use clap::{Arg, Command};
use std::error::Error;

fn add_feature(
    feature: NormalizedPath,
    context: &mut CommandContext,
) -> Result<(), Box<dyn Error>> {
    let node_path = context.git.assert_current_node_path::<AnyGitObject>()?;
    let current_path = if let Some(path) = node_path.try_convert_to::<ConcreteFeature>() {
        path.to_normalized_path()
    } else if let Some(path) = node_path.as_any_type().try_convert_to::<ConcreteArea>() {
        path.get_path_to_feature_root()
    } else {
        return Err(Box::new(CommandError::new(
            "Cannot create feature: Current branch is not a feature or area branch",
        )));
    };
    drop(node_path);
    let target_path = current_path + feature;
    let result = context.git.create_branch::<ConcreteFeature>(&target_path)?;
    context.logger.info(format!(
        "Created new {} {}",
        NodeType::ConcreteFeature.get_formatted_name(),
        result.to_normalized_path().strip_n_left(3),
    ));
    Ok(())
}
fn print_feature_tree(context: &mut CommandContext, show_tags: bool) -> Result<(), Box<dyn Error>> {
    let area = context.git.get_current_area()?;
    match area.move_to_feature_root() {
        Some(path) => {
            context.logger.info(path.display_tree(show_tags).trim());
        }
        None => {}
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct FeatureCommand;
impl CommandDefinition for FeatureCommand {
    fn build_command(&self) -> Command {
        Command::new("feature")
            .about("Manage features")
            .disable_help_subcommand(true)
            .arg(Arg::new("feature").help("Creates new feature as the child of the current one. Requires to be checked out on a feature branch."))
            .arg(Arg::new("delete").short('D').help("Deletes a feature branch"))
            .arg(show_tags())
    }
}
impl CommandInterface for FeatureCommand {
    fn run_command(&self, context: &mut CommandContext) -> Result<(), Box<dyn Error>> {
        let maybe_feature_name = context.arg_helper.get_argument_value::<String>("feature");
        let maybe_delete = context.arg_helper.get_argument_value::<String>("delete");
        let show_tags = context
            .arg_helper
            .get_argument_value::<bool>("show_tags")
            .unwrap();
        match maybe_delete {
            Some(delete) => {
                let current = context.git.assert_current_node_path::<AnyGitObject>()?;
                let to_delete = if let Some(feature) = current.try_convert_to::<ConcreteFeature>() {
                    feature.to_normalized_path() + delete.to_normalized_path()
                } else {
                    context.git.get_current_area()?.get_path_to_feature_root()
                        + delete.to_normalized_path()
                };
                delete_path::<ConcreteFeature>(&to_delete, context)?;
                return Ok(());
            }
            None => {}
        }
        match maybe_feature_name {
            Some(feature_name) => {
                add_feature(NormalizedPath::from(feature_name), context)?;
            }
            None => {
                print_feature_tree(context, show_tags)?;
            }
        }
        Ok(())
    }
    fn shell_complete(
        &self,
        completion_helper: CompletionHelper,
        context: &mut CommandContext,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let maybe_feature_root = context.git.get_current_area()?.move_to_feature_root();
        if maybe_feature_root.is_none() {
            return Ok(vec![]);
        }
        let feature_root = maybe_feature_root.unwrap();
        let result = match completion_helper.currently_editing() {
            Some(arg) => match arg.get_id().as_str() {
                "delete" => {
                    let current = context.git.assert_current_node_path::<AnyGitObject>()?;
                    let reference =
                        if let Some(feature) = current.try_convert_to::<ConcreteFeature>() {
                            feature.to_normalized_path()
                        } else {
                            feature_root.to_normalized_path()
                        };
                    completion_helper.complete_qualified_paths(
                        reference,
                        HasBranchFilteringNodePathTransformer::new(true)
                            .transform(feature_root.iter_children_req())
                            .map(|path| path.to_normalized_path()),
                    )
                }
                _ => {
                    vec![]
                }
            },
            None => {
                vec![]
            }
        };
        Ok(result)
    }
}
