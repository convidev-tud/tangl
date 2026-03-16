use crate::cli::completion::*;
use crate::cli::*;
use crate::model::*;
use clap::{Arg, Command};
use colored::Colorize;
use std::error::Error;

fn add_feature(feature: QualifiedPath, context: &mut CommandContext) -> Result<(), Box<dyn Error>> {
    let node_path = context.git.get_current_node_path::<AnyHasBranch>()?.unwrap();
    let current_path = if let Some(path) = node_path.try_convert_to::<ConcreteFeature>() {
        path.to_qualified_path()
    } else if let Some(path) = node_path.as_any_type().try_convert_to::<ConcreteArea>() {
        path.get_path_to_feature_root()
    } else {
        return Err(Box::new(CommandError::new(
            "Cannot create feature: Current branch is not a feature or area branch",
        )));
    };
    drop(node_path);
    let target_path = current_path + feature;
    let result = context.git.create_branch::<ConcreteProduct>(&target_path)?;
    context.info(format!(
        "Created new {} {}",
        NodeType::ConcreteFeature.get_formatted_name(),
        result.to_qualified_path().strip_n_left(3),
    ));
    Ok(())
}
fn print_feature_tree(context: &mut CommandContext, show_tags: bool) -> Result<(), Box<dyn Error>> {
    let area = context.git.get_current_area()?;
    match area.move_to_feature_root() {
        Some(path) => {
            context.info(path.display_tree(show_tags));
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
                let to_delete = if let Some(current) = context.git.get_current_node_path::<ConcreteFeature>()? {
                    current.to_qualified_path() + delete.to_qualified_path()
                } else {
                    context.git.get_current_area()?.get_path_to_feature_root() + delete.to_qualified_path()
                };
                delete_path::<ConcreteFeature>(&to_delete, context)?;
                return Ok(());
            }
            None => {}
        }
        match maybe_feature_name {
            Some(feature_name) => {
                add_feature(QualifiedPath::from(feature_name), context)?;
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
        if maybe_feature_root.is_none() { return Ok(vec![]); }
        let feature_root = maybe_feature_root.unwrap();
        let result = match completion_helper.currently_editing() {
            Some(arg) => match arg.get_id().as_str() {
                "delete" => {
                    let current = context.git.get_current_node_path::<AnyHasBranch>()?.unwrap();
                    let reference = if let Some(feature) = current.try_convert_to::<ConcreteFeature>() {
                        feature.to_qualified_path()
                    } else {
                        feature_root.to_qualified_path()
                    };
                    completion_helper.complete_qualified_paths(
                        reference,
                        HasBranchFilteringNodePathTransformer::new(true)
                            .transform(feature_root.iter_children_req())
                            .map(|path| path.to_qualified_path()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::interface::test_utils::{
        populate_with_features, populate_with_products, prepare_empty_git_repo,
    };
    use crate::git::interface::{GitInterface, GitPath};
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_feature_add_root_from_area() {
        fn check_existence(interface: &GitInterface) -> Option<NodePath<ConcreteFeature>> {
            interface
                .get_current_area()
                .unwrap()
                .move_to_feature_root()?
                .move_to_feature(&QualifiedPath::from("root"))
        }

        let path = TempDir::new().unwrap();
        let path_buf = PathBuf::from(path.path());
        prepare_empty_git_repo(path_buf.clone()).unwrap();
        let repo = CommandRepository::new(
            Box::new(FeatureCommand),
            GitPath::CustomDirectory(PathBuf::from(path.path())),
        );
        match repo.execute(ArgSource::SUPPLIED(vec!["feature", "root"])) {
            Ok(_) => {
                let interface = GitInterface::in_directory(path_buf);
                check_existence(&interface).unwrap();
                let branch_history = interface
                    .get_commit_history(&QualifiedPath::from("/main/feature/root"))
                    .unwrap();
                let main_history = interface
                    .get_commit_history(&QualifiedPath::from("/main"))
                    .unwrap();
                assert_eq!(branch_history, main_history);
            }
            Err(e) => panic!("{}", e),
        }
    }

    #[test]
    fn test_feature_add_recursive_from_area() {
        fn check_existence(interface: &GitInterface) -> Option<NodePath<ConcreteFeature>> {
            interface
                .get_current_area()
                .unwrap()
                .move_to_feature_root()?
                .move_to_feature(&QualifiedPath::from("root"))?
                .move_to_feature(&QualifiedPath::from("foo"))?
                .move_to_feature(&QualifiedPath::from("1"))
        }

        let path = TempDir::new().unwrap();
        let path_buf = PathBuf::from(path.path());
        prepare_empty_git_repo(path_buf.clone()).unwrap();
        populate_with_features(path_buf.clone()).unwrap();
        let interface = GitInterface::in_directory(path_buf.clone());
        interface
            .checkout(&QualifiedPath::from("/main/feature/root/foo"))
            .unwrap();
        interface.empty_commit("test").unwrap();
        interface.checkout(&QualifiedPath::from("/main")).unwrap();
        let repo = CommandRepository::new(
            Box::new(FeatureCommand),
            GitPath::CustomDirectory(path_buf.clone()),
        );
        match repo.execute(ArgSource::SUPPLIED(vec!["feature", "root/foo/1"])) {
            Ok(_) => {
                let interface = GitInterface::in_directory(path_buf);
                check_existence(&interface).unwrap();
                let branch_history = interface
                    .get_commit_history(&QualifiedPath::from("/main/feature/root/foo/1"))
                    .unwrap();
                let main_history = interface
                    .get_commit_history(&QualifiedPath::from("/main"))
                    .unwrap();
                assert_eq!(branch_history, main_history);
            }
            Err(e) => panic!("{}", e),
        }
    }

    #[test]
    fn test_feature_add_error() {
        let path = TempDir::new().unwrap();
        let path_buf = PathBuf::from(path.path());
        prepare_empty_git_repo(path_buf.clone()).unwrap();
        populate_with_features(path_buf.clone()).unwrap();
        populate_with_products(path_buf.clone()).unwrap();
        let interface = GitInterface::in_directory(path_buf.clone());
        interface
            .checkout(&QualifiedPath::from("/main/product/myprod"))
            .unwrap();
        let repo = CommandRepository::new(
            Box::new(FeatureCommand),
            GitPath::CustomDirectory(path_buf.clone()),
        );
        match repo.execute(ArgSource::SUPPLIED(vec!["feature", "root/foo/1"])) {
            Ok(_) => panic!("Unexpected success"),
            Err(_) => assert!(true),
        }
    }
}
