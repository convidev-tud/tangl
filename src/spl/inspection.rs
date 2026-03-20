use crate::git::interface::GitInterface;
use crate::model::*;
use crate::spl::{DerivationData, DerivationMetadata};
use std::error::Error;

pub struct InspectionManager<'a> {
    git: &'a GitInterface,
}

impl<'a> InspectionManager<'a> {
    pub fn new(git: &'a GitInterface) -> Self {
        Self { git }
    }

    pub fn get_current_derivation_state(
        &self,
        product: &NodePath<ConcreteProduct>,
    ) -> Result<DerivationData, Box<dyn Error>> {
        fn get_current_state(
            commit: &Commit,
            git: &GitInterface,
        ) -> Result<DerivationData, Box<dyn Error>> {
            let mut metadata: Vec<DerivationMetadata> = vec![];
            for data in commit.get_metadata() {
                if let Some(result) = DerivationMetadata::from_commit_message(data) {
                    metadata.push(result?)
                }
            }
            match metadata.len() {
                0 => Ok(DerivationData::new(vec![], commit.get_hash(), None)),
                1 => {
                    let maybe_data = metadata.pop().unwrap();
                    if let Some(data) = maybe_data.get_data() {
                        Ok(data.clone())
                    } else {
                        let pointer = maybe_data.get_pointer().clone().unwrap();
                        let next_commit = git.get_commit_from_hash(&pointer)?;
                        if let Some(next_commit) = next_commit {
                            get_current_state(&next_commit, git)
                        } else {
                            Err(format!("fatal: derivation metadata of commit {} points to commit {} which does not exist", commit.get_hash(), pointer).into())
                        }
                    }
                }
                _ => Err(format!(
                    "fatal: commit {} contains multiple derivation metadata",
                    commit.get_hash()
                )
                .into()),
            }
        }

        let last_commit = self.git.get_last_commit(&product)?;
        get_current_state(&last_commit, self.git)
    }
}
