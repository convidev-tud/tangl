use crate::git::conflict::{
    ConflictAnalyzer, ConflictChecker, MergeChainStatistic, MergeConflict, MergePending,
    MergeStatistic, MergeSuccess,
};
use crate::git::error::{GitError, GitModelError, GitSerdeError};
use crate::git::interface::GitInterface;
use crate::logging::TanglLogger;
use crate::model::*;
use crate::spl::{
    AbortDerivationError, ContinueDerivationError, InitializeDerivationError, InspectionManager,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureMetadata {
    path: String,
    conflicting: bool,
}
impl FeatureMetadata {
    pub fn new<S: Into<String>>(path: S, conflicting: bool) -> Self {
        Self {
            path: path.into(),
            conflicting,
        }
    }
    pub fn get_qualified_path(&self) -> QualifiedPath {
        QualifiedPath::from(&self.path)
    }
    pub fn get_conflicting(&self) -> bool {
        self.conflicting
    }
}

impl Into<MergeChainStatistic> for &Vec<FeatureMetadata> {
    fn into(self) -> MergeChainStatistic {
        let mut statistic = MergeChainStatistic::new();
        for value in self {
            let s = if !value.get_conflicting() {
                let success = MergeSuccess::new(value.get_qualified_path());
                MergeStatistic::Success(success)
            } else {
                let conflict = MergeConflict::new(value.get_qualified_path());
                MergeStatistic::Conflict(conflict)
            };
            statistic.push(s)
        }
        statistic
    }
}

impl From<&MergeChainStatistic> for Vec<FeatureMetadata> {
    fn from(value: &MergeChainStatistic) -> Self {
        value
            .get_chain()
            .iter()
            .map(|s| {
                let conflicting = match s {
                    MergeStatistic::Success(_) | MergeStatistic::Base(_) => false,
                    _ => true,
                };
                FeatureMetadata::new(s.get_path().to_string(), conflicting)
            })
            .collect()
    }
}

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
impl DerivationState {
    pub fn from_string<S: Into<String>>(from: S) -> Self {
        let real = from.into();
        if real == "in_progress" {
            Self::InProgress
        } else {
            Self::None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivationData {
    id: String,
    state: String,
    initial_commit: String,
    completed: Vec<FeatureMetadata>,
    missing: Vec<FeatureMetadata>,
    total: Vec<FeatureMetadata>,
}
impl DerivationData {
    pub fn new<S: Into<String>>(
        features: Vec<FeatureMetadata>,
        initial_commit: S,
        previously_finished: Option<&Self>,
    ) -> Self {
        let uuid = Uuid::new_v4();
        if let Some(prev) = previously_finished {
            match prev.get_state() {
                DerivationState::InProgress => Self {
                    id: prev.id.clone(),
                    initial_commit: prev.initial_commit.clone(),
                    state: prev.state.clone(),
                    completed: prev.completed.clone(),
                    missing: prev.missing.clone(),
                    total: prev.total.clone(),
                },
                DerivationState::None => {
                    let mut total = prev.get_total().clone();
                    for f in features.iter() {
                        if !total.contains(f) {
                            total.push(f.clone());
                        }
                    }
                    Self {
                        id: uuid.to_string(),
                        initial_commit: initial_commit.into(),
                        state: DerivationState::InProgress.to_string(),
                        completed: vec![],
                        missing: features.clone(),
                        total,
                    }
                }
            }
        } else {
            Self {
                id: uuid.to_string(),
                initial_commit: initial_commit.into(),
                state: DerivationState::None.to_string(),
                completed: vec![],
                missing: features.clone(),
                total: features,
            }
        }
    }
    pub fn new_initial<S: Into<String>>(initial_commit: S) -> Self {
        Self::new(Vec::new(), initial_commit, None)
    }
    pub fn as_none(&mut self) {
        self.state = DerivationState::None.to_string();
    }
    pub fn as_in_progress(&mut self) {
        self.state = DerivationState::InProgress.to_string();
    }
    pub fn mark_as_completed(&mut self, feature: &QualifiedPath) {
        let old_missing: Vec<FeatureMetadata> = self.missing.clone();
        let missing = old_missing
            .iter()
            .find(|m| m.get_qualified_path() == *feature);
        if missing.is_some() {
            self.missing.retain(|m| m.get_qualified_path() != *feature);
            self.completed.push(missing.unwrap().clone())
        }
    }
    pub fn update_missing(&mut self, new_order: &MergeChainStatistic) {
        self.missing = new_order.into();
    }
    pub fn get_completed(&self) -> &Vec<FeatureMetadata> {
        &self.completed
    }
    pub fn get_missing(&self) -> &Vec<FeatureMetadata> {
        &self.missing
    }
    pub fn get_total(&self) -> &Vec<FeatureMetadata> {
        &self.total
    }
    pub fn get_state(&self) -> DerivationState {
        DerivationState::from_string(&self.state)
    }
    pub fn get_id(&self) -> &String {
        &self.id
    }
    pub fn get_initial_commit(&self) -> &String {
        &self.initial_commit
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivationMetadata {
    pointer: Option<String>,
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
    pub fn new<S: Into<String>>(pointer: Option<S>, data: Option<DerivationData>) -> Self {
        if pointer.is_none() && data.is_none() || pointer.is_some() && data.is_some() {
            panic!("Must have a pointer XOR data")
        }
        if let Some(p) = pointer {
            Self {
                pointer: Some(p.into()),
                data,
            }
        } else {
            Self {
                pointer: None,
                data,
            }
        }
    }
    pub fn get_pointer(&self) -> Option<&String> {
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
    ) -> Result<String, GitSerdeError> {
        let real_message = message.into();
        let container = CommitMetadataContainer::new(metadata)?;
        Ok(self.git.empty_commit(real_message, Some(&container))?)
    }

    fn run_derivation_until_conflict(
        &mut self,
    ) -> Result<Option<NodePath<ConcreteFeature>>, GitModelError> {
        let old_chain: &MergeChainStatistic = &self.current_state.get_missing().into();
        let feature_paths = old_chain.into();
        let features = self
            .git
            .get_model()
            .assert_all::<ConcreteFeature>(&feature_paths)?;
        let mut new_state = self.current_state.clone();
        let mut missing_feature: Option<NodePath<ConcreteFeature>> = None;
        for feature in features {
            let (statistic, _) = self.git.merge(&feature)?;
            if statistic.contains_conflicts() {
                self.git.abort_merge()?;
                missing_feature = Some(feature);
                break;
            } else {
                new_state.mark_as_completed(&feature.to_qualified_path());
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
    pub fn get_pending_chain(&self) -> Result<MergeChainStatistic, GitError> {
        let mut chain: MergeChainStatistic = self.current_state.get_missing().into();
        let base = MergeStatistic::Base(self.product.to_qualified_path());
        chain.insert(0, base);
        if self.git.pending_merge()? {
            let second = chain.remove(1);
            let merging = MergeStatistic::Merging(MergePending::new(second.get_path().clone()));
            chain.insert(1, merging);
        };
        Ok(chain)
    }
    pub fn get_product(&self) -> &NodePath<ConcreteProduct> {
        &self.product
    }

    pub fn predict_conflicts(
        &self,
        order: &Vec<NodePath<AnyHasBranch>>,
        optimize_order: bool,
    ) -> Result<MergeChainStatistic, GitError> {
        let checker = ConflictChecker::new(&self.git);
        let mut analyzer = ConflictAnalyzer::new(checker, self.logger);
        let matrix = analyzer.calculate_2d_heuristics_matrix_with_merge_base(
            &order,
            &self.product.try_convert_to().unwrap(),
        )?;
        let new_order = if optimize_order {
            matrix.calculate_best_path_greedy(&self.product.to_qualified_path())
        } else {
            let mut with_base = vec![self.product.to_qualified_path()];
            let paths: Vec<QualifiedPath> = order.iter().map(|p| p.to_qualified_path()).collect();
            with_base.extend(paths);
            matrix.predict_conflicts(&with_base)
        };
        let chain = new_order.get_chain()[1..].to_vec();
        let modified = chain.into();
        Ok(modified)
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
                let chain = &self.predict_conflicts(&transformed, optimize)?;
                let current_commit = self.git.get_last_commit(&self.product)?;
                let new_data = DerivationData::new(
                    chain.into(),
                    current_commit.get_hash(),
                    Some(&self.current_state),
                );
                let payload = DerivationMetadata::new::<String>(None, Some(new_data.clone()));
                self.derivation_commit("Derivation start", &payload)?;
                self.current_state = new_data;
                Ok(self.current_state.clone())
            }
            DerivationState::InProgress => Err(InitializeDerivationError::DerivationInProgress),
        }
    }

    pub fn continue_derivation(&mut self) -> Result<DerivationData, ContinueDerivationError> {
        match self.current_state.get_state() {
            DerivationState::InProgress => {
                let maybe_merging = self.run_derivation_until_conflict()?;
                let metadata =
                    DerivationMetadata::new::<String>(None, Some(self.current_state.clone()));
                let message = match self.current_state.get_state() {
                    DerivationState::InProgress => "Derivation progress",
                    DerivationState::None => "Derivation finished",
                };
                self.derivation_commit(message, &metadata)?;
                if let Some(merging) = maybe_merging {
                    self.git.merge(&merging)?;
                }
                Ok(self.current_state.clone())
            }
            DerivationState::None => Err(ContinueDerivationError::NoDerivationInProgress),
        }
    }

    pub fn optimize_merge_order(&mut self) -> Result<MergeChainStatistic, Box<dyn Error>> {
        let old_order: &MergeChainStatistic = &self.current_state.get_missing().into();
        let missing: Vec<QualifiedPath> = old_order.into();
        let features = self.git.get_model().assert_all(&missing)?;
        let new_order = self.predict_conflicts(&features, true)?;
        if *old_order != new_order {
            self.current_state.update_missing(&new_order);
            let metadata =
                DerivationMetadata::new::<String>(None, Some(self.current_state.clone()));
            self.derivation_commit("", &metadata)?;
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
}
