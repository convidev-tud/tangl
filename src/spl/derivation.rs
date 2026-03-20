use crate::git::interface::GitInterface;
use crate::model::*;
use crate::spl::InspectionManager;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io;
use std::process::Output;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureMetadata {
    path: String,
}
impl FeatureMetadata {
    pub fn new<S: Into<String>>(path: S) -> Self {
        Self { path: path.into() }
    }
    pub fn from_qualified_paths(paths: &Vec<QualifiedPath>) -> Vec<Self> {
        paths.iter().map(|path| Self::new(path.clone())).collect()
    }
    pub fn from_features(features: &Vec<NodePath<ConcreteFeature>>) -> Vec<Self> {
        features
            .iter()
            .map(|path| Self::new(path.to_qualified_path()))
            .collect()
    }
    pub fn qualified_paths(metadata: &Vec<Self>) -> Vec<QualifiedPath> {
        metadata.iter().map(|m| m.get_qualified_path()).collect()
    }
    pub fn get_qualified_path(&self) -> QualifiedPath {
        QualifiedPath::from(&self.path)
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
                    for f in prev.total.iter() {
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
                state: DerivationState::InProgress.to_string(),
                completed: vec![],
                missing: features.clone(),
                total: features,
            }
        }
    }
    pub fn as_finished(&mut self) {
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
    pub fn reorder_missing(&mut self, new_order: &Vec<QualifiedPath>) {
        let old_missing = FeatureMetadata::qualified_paths(&self.missing);
        let mut new_missing: Vec<QualifiedPath> = Vec::new();
        for new in new_order.iter() {
            if !old_missing.contains(new) {
                panic!("Cannot reorder: tried to introduce new feature")
            }
            new_missing.push(new.clone());
        }
        self.missing = FeatureMetadata::from_qualified_paths(&new_missing);
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
    pub fn get_pointer(&self) -> &Option<String> {
        &self.pointer
    }
    pub fn get_data(&self) -> &Option<DerivationData> {
        &self.data
    }
}

#[derive(Debug)]
pub enum DerivationError {
    Io(io::Error),
    Model(ModelError),
    DerivationInProgress,
    NoDerivationInProgress,
}

impl From<ModelError> for DerivationError {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}

impl From<io::Error> for DerivationError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl Display for DerivationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => err.fmt(f),
            Self::Model(err) => err.fmt(f),
            Self::DerivationInProgress => f.write_str("fatal: a derivation is currently in progress"),
            Self::NoDerivationInProgress => f.write_str("fatal: no derivation in progress"),
        }
    }
}

impl Error for DerivationError {}

pub struct DerivationManager<'a> {
    product: &'a NodePath<ConcreteProduct>,
    current_state: DerivationData,
    git: &'a GitInterface,
}

impl<'a> DerivationManager<'a> {
    pub fn new(
        product: &'a NodePath<ConcreteProduct>,
        git: &'a GitInterface,
    ) -> Result<Self, Box<dyn Error>> {
        let inspector = InspectionManager::new(git);
        let current_state = inspector.get_current_derivation_state(&product)?;
        Ok(Self { product, current_state, git })
    }

    fn derivation_commit<S: Into<String>>(
        &self,
        message: S,
        metadata: &DerivationMetadata,
    ) -> Result<Output, io::Error> {
        let real_message = message.into();
        let metadata_json = metadata.to_commit_message()?;
        let total = format!("{real_message}\n\n{metadata_json}");
        self.git.commit(&total)
    }

    fn run_derivation_until_conflict(&mut self) -> Result<(), DerivationError> {
        let feature_paths = FeatureMetadata::qualified_paths(&self.current_state.missing);
        let features = self
            .git
            .get_model()
            .assert_all::<ConcreteFeature>(&feature_paths)?;
        let mut new_state = self.current_state.clone();
        for feature in features {
            let out = self.git.merge(&feature)?;
            if out.status.success() {
                new_state.mark_as_completed(&feature.to_qualified_path());
            } else {
                self.git.abort_merge()?;
                break;
            }
        }
        self.current_state = new_state;
        Ok(())
    }
    
    pub fn get_current_state(&self) -> DerivationData {
        self.current_state.clone()
    }

    pub fn initialize_derivation(
        &mut self,
        features: &Vec<NodePath<ConcreteFeature>>,
    ) -> Result<DerivationData, DerivationError> {
        match self.current_state.get_state() {
            DerivationState::None => {
                let feature_metadata = FeatureMetadata::from_features(&features);
                let current_commit = self.git.get_last_commit(&self.product)?;
                let new_data = DerivationData::new(
                    feature_metadata,
                    current_commit.get_hash(),
                    Some(&self.current_state),
                );
                let payload = DerivationMetadata::new::<String>(None, Some(new_data.clone()));
                self.derivation_commit("Derivation start", &payload)?;
                self.current_state = new_data;
                Ok(self.current_state.clone())
            }
            DerivationState::InProgress => Err(DerivationError::DerivationInProgress),
        }
    }

    pub fn continue_derivation(&mut self) -> Result<DerivationData, DerivationError> {
        match self.current_state.get_state() {
            DerivationState::InProgress => {
                self.run_derivation_until_conflict()?;
                let metadata =
                    DerivationMetadata::new::<String>(None, Some(self.current_state.clone()));
                let message = match self.current_state.get_state() {
                    DerivationState::InProgress => "Derivation progress",
                    DerivationState::None => "Derivation finished",
                };
                self.derivation_commit(message, &metadata)?;
                Ok(self.current_state.clone())
            }
            DerivationState::None => Err(DerivationError::NoDerivationInProgress),
        }
    }
}
