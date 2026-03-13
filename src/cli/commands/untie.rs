use crate::cli::*;
use crate::git::interface::GitInterface;
use crate::model::*;
use clap::{Arg, Command};
use colored::Colorize;
use std::error::Error;

const COMMIT: &str = "commit";
const FEATURE: &str = "feature";

fn assert_target_commit_between_derivations(
    target_hash: &String,
    commits: &Vec<Commit>,
) -> Result<DerivationCommit, Box<dyn Error>> {
    let mut commit_found = false;
    let mut derivation_commit: Option<DerivationCommit> = None;
    for commit in commits {
        if commit.get_hash() == target_hash {
            if DerivationCommit::from_commit(commit.clone()).is_some() {
                return Err("Cannot untie: target is a derivation commit".into());
            }
            commit_found = true;
        }
        if commit_found {
            if let Some(result) = DerivationCommit::from_commit(commit.clone()) {
                match result {
                    Ok(dc) => match dc.get_metadata().get_state() {
                        DerivationState::Finished => {
                            derivation_commit = Some(dc);
                            break;
                        }
                        _ => return Err("Cannot untie: target happened while deriving".into()),
                    },
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            }
        }
    }
    Ok(derivation_commit.unwrap())
}

fn find_features_that_contain_files<'a>(
    commit: &'a String,
    features: &'a Vec<NodePath<ConcreteFeature>>,
    git_interface: &GitInterface,
) -> Result<Vec<&'a NodePath<ConcreteFeature>>, Box<dyn Error>> {
    let files_of_commit = git_interface.get_files_changed_by_commit(commit)?;
    let all = features
        .iter()
        .filter(|feature| {
            let files_of_feature = git_interface.get_files_managed_by_branch(feature).unwrap();
            let mut found_all = true;
            for file in files_of_commit.iter() {
                if !files_of_feature.contains(file) {
                    found_all = false
                }
            }
            found_all
        })
        .collect();
    Ok(all)
}

#[derive(Clone, Debug)]
pub struct UntieCommand;

impl CommandDefinition for UntieCommand {
    fn build_command(&self) -> Command {
        Command::new("untie")
            .about("Untie commit from product and merge back into feature")
            .disable_help_subcommand(true)
            .arg(
                Arg::new(COMMIT)
                    .default_value("HEAD")
                    .help("Specific commit to untie"),
            )
            .arg(Arg::new(FEATURE).help("Feature to untie to"))
    }
}

impl CommandInterface for UntieCommand {
    fn run_command(&self, context: &mut CommandContext) -> Result<(), Box<dyn Error>> {
        let product = if let Some(path) = context
            .git
            .get_current_node_path()?
            .try_convert_to::<ConcreteProduct>()
        {
            path
        } else {
            return Err("Not on product branch".into());
        };
        let target_commit = context
            .arg_helper
            .get_argument_value::<String>(COMMIT)
            .unwrap();
        let maybe_feature = context.arg_helper.get_argument_value::<String>(FEATURE);

        let commits = context.git.get_commit_history(&product)?;
        if commits.is_empty() {
            context.info("No commits on product");
            return Ok(());
        }
        let target_commit_hash: String = match target_commit.as_str() {
            "HEAD" => commits.get(0).unwrap().get_hash().clone(),
            _ => target_commit,
        };
        let finished_derivation =
            assert_target_commit_between_derivations(&target_commit_hash, &commits)?;
        let raw_features = finished_derivation
            .get_metadata()
            .get_total()
            .iter()
            .map(|p| p.get_qualified_path())
            .collect();
        let features = context
            .git
            .get_model()
            .assert_all::<ConcreteFeature>(&raw_features)?;
        let found = find_features_that_contain_files(&target_commit_hash, &features, &context.git)?;
        let feature: NodePath<ConcreteFeature> = match maybe_feature {
            Some(feature) => context
                .git
                .get_model()
                .assert_path(&feature.to_qualified_path())?,
            None => match found.len() {
                0 => return Err(
                    "There are no features matching all changed files. Please choose one manually."
                        .into(),
                ),
                1 => features.get(0).unwrap().clone(),
                _ => {
                    return Err(
                        "There are multiple potential untie targets. Please choose one manually."
                            .into(),
                    );
                }
            },
        };
        let current_path = context.git.get_current_node_path()?;
        context.git.checkout(&feature)?;
        let output = context.git.cherry_pick(&target_commit_hash)?;
        if !output.status.success() {
            context.git.abort_merge()?;
            context.info(format!("Unable to untie commit {}", &target_commit_hash));
        } else {
            context.info(format!(
                "Untied commit {} to {}",
                target_commit_hash.blue(),
                feature.to_qualified_path().to_string().blue()
            ));
        }
        context.git.checkout(&current_path)?;
        Ok(())
    }
}
