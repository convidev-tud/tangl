use crate::git::error::GitError;
use crate::git::interface::GitInterface;
use crate::model::*;
use crate::spl::{DerivationData, DerivationMetadata, FeatureMetadata};
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
                        match git.get_commit_from_hash(&pointer) {
                            Ok(next_commit) => get_current_state(&next_commit, git),
                            Err(error) => {
                                match error {
                                    GitError::Git(_) => Err(format!("fatal: derivation metadata of commit {} points to commit {} which does not exist", commit.get_hash(), pointer).into()),
                                    GitError::Io(e) => Err(e.into())
                                }
                            }
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

    pub fn find_products_containing_feature(
        &self,
        feature: &NodePath<ConcreteFeature>,
    ) -> Result<Vec<NodePath<ConcreteProduct>>, Box<dyn Error>> {
        if let Some(product_root) = feature.clone().move_to_area().move_to_product_root() {
            let mut products: Vec<NodePath<ConcreteProduct>> = vec![];
            for product in product_root.iter_products_req() {
                if let Some(concrete) = product.try_convert_to::<ConcreteProduct>() {
                    let state = self.get_current_derivation_state(&concrete)?;
                    let features = FeatureMetadata::qualified_paths(state.get_total());
                    if features.contains(&feature.to_qualified_path()) {
                        products.push(concrete);
                    }
                }
            };
            Ok(products)
        } else {
            Ok(vec![])
        }
    }
}
