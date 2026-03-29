use crate::git::conflict::{CheckMode, ConflictAnalyzer, ConflictChecker, MergeChainStatistic, MergeResult, MergeStatistic, NormalizedMergeStatistic};
use crate::git::error::PathAssertionError;
use crate::git::interface::GitInterface;
use crate::logging::TanglLogger;
use crate::model::*;
use crate::spl::{AbortDerivationError, ContinueDerivationError, DerivationCommitError, InitializeDerivationError, InspectionManager, OptimizeMergeOrderError, UpdateProductError};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DerivationState {
    InProgress,
    None,
}
impl Display for DerivationState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let out = match self {
            DerivationState::InProgress => "in_progress",
            DerivationState::None => "none",
        };
        f.write_str(out)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivationData {
    id: String,
    state: DerivationState,
    initial_commit: CommitHash,
    completed: Vec<NormalizedMergeStatistic>,
    missing: Vec<NormalizedMergeStatistic>,
    total: Vec<NormalizedPath>,
}
impl DerivationData {
    pub fn new_in_progress(
        features: Vec<NormalizedMergeStatistic>,
        initial_commit: CommitHash,
        previous: &Self,
    ) -> Self {
        let uuid = Uuid::new_v4();
        match previous.get_state() {
            DerivationState::InProgress => Self {
                id: previous.id.clone(),
                initial_commit: previous.initial_commit.clone(),
                state: previous.state.clone(),
                completed: previous.completed.clone(),
                missing: previous.missing.clone(),
                total: previous.total.clone(),
            },
            DerivationState::None => {
                let mut mut_features = features.clone();
                let intermediate: Vec<NormalizedPath> = previous
                    .total
                    .iter()
                    .map(|p| {
                        if let Some(feature) = features
                            .iter()
                            .find(|f| f.get_path().strip_version() == p.strip_version()) {
                            mut_features.retain(|f| f != feature);
                            match feature.get_stat() {
                                MergeResult::UpToDate => p.clone(),
                                _ => feature.get_path().clone(),
                            }
                        } else {
                            p.clone()
                        }
                    })
                    .collect();
                let mut total = intermediate.clone();
                total.extend(mut_features.iter().map(|f| f.get_path().clone()));
                Self {
                    id: uuid.to_string(),
                    initial_commit,
                    state: DerivationState::InProgress,
                    completed: vec![],
                    missing: features.clone(),
                    total,
                }
            }
        }
    }
    pub fn new_initial(initial_commit: CommitHash) -> Self {
        let uuid = Uuid::new_v4();
        Self {
            id: uuid.to_string(),
            initial_commit,
            state: DerivationState::None,
            completed: vec![],
            missing: vec![],
            total: vec![],
        }
    }
    pub fn as_none(&mut self) {
        self.state = DerivationState::None;
    }
    pub fn as_in_progress(&mut self) {
        self.state = DerivationState::InProgress;
    }
    pub fn mark_as_completed(&mut self, feature: &NormalizedPath) {
        let old_missing = self.missing.clone();
        let missing = old_missing.iter().find(|m| m.get_path() == feature);
        if let Some(missing) = missing {
            self.missing.retain(|m| m.get_path() != feature);
            let new =
                NormalizedMergeStatistic::new(missing.get_path().clone(), MergeResult::Success);
            self.completed.push(new)
        }
    }
    pub fn update_missing(&mut self, new_order: &Vec<NormalizedMergeStatistic>) {
        self.missing = new_order.clone()
    }
    pub fn get_completed(&self) -> &Vec<NormalizedMergeStatistic> {
        &self.completed
    }
    pub fn get_missing(&self) -> &Vec<NormalizedMergeStatistic> {
        &self.missing
    }
    pub fn get_total(&self) -> &Vec<NormalizedPath> {
        &self.total
    }
    pub fn get_total_without_versions(&self) -> Vec<NormalizedPath> {
        self.total.iter().map(|p| p.strip_version()).collect()
    }
    pub fn get_state(&self) -> &DerivationState {
        &self.state
    }
    pub fn get_id(&self) -> &String {
        &self.id
    }
    pub fn get_initial_commit(&self) -> &CommitHash {
        &self.initial_commit
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivationMetadata {
    pointer: Option<CommitHash>,
    data: Option<DerivationData>,
}

impl CommitMetadata for DerivationMetadata {
    fn header() -> String {
        "---derivation-metadata---".to_string()
    }
    fn from_json<S: Into<String>>(content: S) -> serde_json::error::Result<Self> {
        serde_json::from_str::<Self>(&content.into())
    }
    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self)
    }
}

impl DerivationMetadata {
    pub fn new(pointer: Option<CommitHash>, data: Option<DerivationData>) -> Self {
        if pointer.is_none() && data.is_none() || pointer.is_some() && data.is_some() {
            panic!("Must have a pointer XOR data")
        }
        if let Some(p) = pointer {
            Self {
                pointer: Some(p),
                data,
            }
        } else {
            Self {
                pointer: None,
                data,
            }
        }
    }
    pub fn get_pointer(&self) -> Option<&CommitHash> {
        self.pointer.as_ref()
    }
    pub fn get_data(&self) -> Option<&DerivationData> {
        self.data.as_ref()
    }
}

pub struct DerivationCommit {
    commit: Commit,
    metadata: DerivationMetadata,
}

impl DerivationCommit {
    pub fn new(commit: Commit, metadata: DerivationMetadata) -> Self {
        Self { commit, metadata }
    }

    pub fn get_commit(&self) -> &Commit {
        &self.commit
    }

    pub fn get_metadata(&self) -> &DerivationMetadata {
        &self.metadata
    }
}

pub struct DerivationManager<'a> {
    product: &'a NodePath<ConcreteProduct>,
    current_state: DerivationData,
    git: &'a GitInterface,
    logger: &'a TanglLogger,
}

impl<'a> DerivationManager<'a> {
    pub fn new(
        product: &'a NodePath<ConcreteProduct>,
        git: &'a GitInterface,
        logger: &'a TanglLogger,
    ) -> Result<Self, Box<dyn Error>> {
        let inspector = InspectionManager::new(git);
        let current_state = inspector.get_last_derivation_state(&product)?;
        Ok(Self {
            product,
            current_state,
            git,
            logger,
        })
    }

    fn derivation_commit<S: Into<String>>(
        &self,
        message: S,
        metadata: &DerivationMetadata,
    ) -> Result<String, DerivationCommitError> {
        let real_message = message.into();
        let container = CommitMetadataContainer::new(metadata)?;
        Ok(self.git.commit::<_, ConcreteProduct>(real_message, Some(&container), true, true)?)
    }

    fn run_derivation_until_conflict(
        &mut self,
    ) -> Result<Option<NodePath<ConcreteFeature>>, PathAssertionError> {
        let mut chain = MergeChainStatistic::<_, ConcreteFeature>::new(self.product.clone());
        chain.fill_from_normalized(self.current_state.get_missing().clone(), self.git)?;
        let mut new_state = self.current_state.clone();
        let mut missing_feature: Option<NodePath<ConcreteFeature>> = None;
        for stat in chain.iter_chain() {
            let feature = stat.get_path();
            let (statistic, _) = self.git.merge::<ConcreteProduct, _>(feature.clone())?;
            if statistic.contains_conflicts() {
                self.git.abort_merge()?;
                missing_feature = Some(feature.clone());
                break;
            } else {
                new_state.mark_as_completed(&stat.get_path().to_normalized_path());
            }
        }
        if missing_feature.is_none() {
            new_state.as_none();
        }
        self.current_state = new_state;
        Ok(missing_feature)
    }

    pub fn get_current_state(&self) -> DerivationData {
        self.current_state.clone()
    }
    pub fn get_pending_chain(
        &self,
    ) -> Result<Option<MergeChainStatistic<ConcreteProduct, ConcreteFeature>>, PathAssertionError>
    {
        let chain = if self.current_state.get_missing().len() == 0 {
            None
        } else {
            let mut chain = MergeChainStatistic::new(self.product.clone());
            chain.fill_from_normalized(self.current_state.missing.clone(), self.git)?;
            if self.git.pending_merge()? {
                let second = chain.remove(0);
                let merging = MergeStatistic::new(second.get_path().clone(), MergeResult::Merging);
                chain.insert(0, merging);
            };
            Some(chain)
        };
        Ok(chain)
    }
    pub fn get_product(&self) -> &NodePath<ConcreteProduct> {
        &self.product
    }

    pub fn predict_conflicts(
        &self,
        order: &Vec<NodePath<ConcreteFeature>>,
        optimize_order: bool,
    ) -> Result<MergeChainStatistic<ConcreteProduct, ConcreteFeature>, PathAssertionError> {
        let checker = ConflictChecker::new(&self.git, CheckMode::Merge);
        let mut analyzer = ConflictAnalyzer::new(checker, self.logger);
        let matrix = analyzer.calculate_2d_heuristics_matrix_with_merge_base(
            &ByTypeFilteringNodePathTransformer::new()
                .transform(order.iter().cloned())
                .collect(),
            &self.product.try_convert_to().unwrap(),
        )?;
        let new_order = if optimize_order {
            matrix.estimate_best_path(&self.product.try_convert_to().unwrap())
        } else {
            matrix.predict_conflicts(&self.product, &order)
        };
        Ok(new_order.unwrap())
    }

    pub fn initialize_derivation(
        &mut self,
        features: Vec<NodePath<ConcreteFeature>>,
        optimize: bool,
    ) -> Result<DerivationData, InitializeDerivationError> {
        match self.current_state.get_state() {
            DerivationState::None => {
                let transformer = ByTypeFilteringNodePathTransformer::new();
                let transformed = transformer.transform(features.into_iter()).collect();
                let chain = self.predict_conflicts(&transformed, optimize)?;
                if !chain.all_up_to_date() {
                    let current_commit = self.git.get_commit(&self.product)?;
                    let new_data = DerivationData::new_in_progress(
                        chain.to_normalized(),
                        current_commit.get_hash().clone(),
                        &self.current_state,
                    );
                    let payload = DerivationMetadata::new(None, Some(new_data.clone()));
                    self.derivation_commit("Derivation start", &payload)?;
                    self.current_state = new_data;
                }
                Ok(self.current_state.clone())
            }
            DerivationState::InProgress => Err(InitializeDerivationError::DerivationInProgress),
        }
    }

    pub fn continue_derivation(&mut self) -> Result<DerivationData, ContinueDerivationError> {
        match self.current_state.get_state() {
            DerivationState::InProgress => {
                let maybe_merging = self.run_derivation_until_conflict()?;
                let metadata = DerivationMetadata::new(None, Some(self.current_state.clone()));
                let message = match self.current_state.get_state() {
                    DerivationState::InProgress => "Derivation progress",
                    DerivationState::None => "Derivation finished",
                };
                self.derivation_commit(message, &metadata)?;
                if let Some(merging) = maybe_merging {
                    self.git
                        .merge::<ConcreteProduct, ConcreteFeature>(merging)?;
                }
                Ok(self.current_state.clone())
            }
            DerivationState::None => Err(ContinueDerivationError::NoDerivationInProgress),
        }
    }

    pub fn optimize_merge_order(
        &mut self,
    ) -> Result<MergeChainStatistic<ConcreteProduct, ConcreteFeature>, OptimizeMergeOrderError>
    {
        let old_order = self.get_pending_chain()?;
        let missing = self.current_state.get_missing().to_normalized_paths();
        let features = self.git.assert_paths(&missing)?;
        let new_order = self.predict_conflicts(&features, true)?;
        if old_order.is_none() || old_order.unwrap() != new_order {
            self.current_state
                .update_missing(&new_order.to_normalized());
            let metadata = DerivationMetadata::new(None, Some(self.current_state.clone()));
            self.derivation_commit("Optimize order", &metadata)?;
        }
        Ok(new_order)
    }

    pub fn abort_derivation(self) -> Result<DerivationData, AbortDerivationError> {
        match self.current_state.get_state() {
            DerivationState::InProgress => {
                let previous = self.current_state.get_initial_commit();
                self.git.reset_hard(previous)?;
            }
            DerivationState::None => return Err(AbortDerivationError::NoDerivationInProgress),
        };
        Ok(self.current_state)
    }

    pub fn update_product(&mut self, optimize: bool) -> Result<DerivationData, UpdateProductError> {
        match self.current_state.get_state() {
            DerivationState::None => {
                let all_features = self.current_state.get_total_without_versions();
                let node_paths = self.git.assert_paths::<ConcreteFeature>(&all_features)?;
                Ok(self.initialize_derivation(node_paths, optimize)?)
            }
            DerivationState::InProgress => Err(UpdateProductError::DerivationInProgress),
        }
    }
}
