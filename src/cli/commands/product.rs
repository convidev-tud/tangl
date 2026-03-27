use crate::cli::completion::*;
use crate::cli::*;
use crate::model::*;
use clap::{Arg, Command};
use std::error::Error;

const PRODUCT: &str = "product";

fn add_product(
    product: NormalizedPath,
    context: &mut CommandContext,
) -> Result<(), Box<dyn Error>> {
    let node_path = context.git.assert_current_node_path::<AnyGitObject>()?;
    let current_path = if let Some(path) = node_path.try_convert_to::<ConcreteProduct>() {
        path.to_normalized_path()
    } else if let Some(path) = node_path.as_any_type().try_convert_to::<ConcreteArea>() {
        path.get_path_to_product_root()
    } else {
        return Err(Box::new(CommandError::new(
            "Cannot create product: Current branch is not a product or area branch",
        )));
    };
    drop(node_path);
    let target_path = current_path + product;
    let result = context.git.create_branch::<ConcreteProduct>(&target_path)?;
    context.logger.info(format!(
        "Created new {} {}",
        NodeType::ConcreteProduct.get_formatted_name(),
        result.to_normalized_path().strip_n_left(3),
    ));
    Ok(())
}

fn print_product_tree(context: &mut CommandContext) -> Result<(), Box<dyn Error>> {
    let area = context.git.get_current_area()?;
    match area.move_to_product_root() {
        Some(path) => {
            context.logger.info(path.display_tree(false).trim());
        }
        None => {}
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct ProductCommand;
impl CommandDefinition for ProductCommand {
    fn build_command(&self) -> Command {
        Command::new("product")
            .about("Manage products")
            .disable_help_subcommand(true)
            .arg(Arg::new(PRODUCT))
            .arg(
                Arg::new("delete")
                    .short('D')
                    .exclusive(true)
                    .help("Deletes a product branch"),
            )
    }
}
impl CommandInterface for ProductCommand {
    fn run_command(&self, context: &mut CommandContext) -> Result<(), Box<dyn Error>> {
        let maybe_delete = context.arg_helper.get_argument_value::<String>("delete");
        let maybe_product = context.arg_helper.get_argument_value::<String>(PRODUCT);
        if let Some(delete) = maybe_delete {
            let current = context.git.assert_current_node_path::<AnyGitObject>()?;
            let to_delete = if let Some(product) = current.try_convert_to::<ConcreteProduct>() {
                product.to_normalized_path() + delete.to_normalized_path()
            } else {
                context.git.get_current_area()?.get_path_to_product_root()
                    + delete.to_normalized_path()
            };
            delete_path::<ConcreteProduct>(&to_delete, context)?;
        } else if maybe_product.is_some() {
            add_product(maybe_product.unwrap().to_normalized_path(), context)?;
        } else {
            print_product_tree(context)?;
        }
        Ok(())
    }
    fn shell_complete(
        &self,
        completion_helper: CompletionHelper,
        context: &mut CommandContext,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let result = match completion_helper.currently_editing() {
            Some(arg) => match arg.get_id().as_str() {
                "delete" => {
                    let maybe_feature_root = context.git.get_current_area()?.move_to_product_root();
                    match maybe_feature_root {
                        Some(path) => completion_helper.complete_normalized_paths(
                            path.to_normalized_path(),
                            HasBranchFilteringNodePathTransformer::new(true)
                                .transform(path.iter_children_req())
                                .map(|path| path.to_normalized_path()),
                        ),
                        None => {
                            vec![]
                        }
                    }
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
