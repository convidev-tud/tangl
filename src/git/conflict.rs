use crate::git::error::PathAssertionError;
use crate::git::interface::GitInterface;
use crate::logging::TanglLogger;
use crate::model::{
    AnyGitObject, IsGitObject, NodePath, NormalizedPath, Temporary, ToNormalizedPath,
    ToNormalizedPaths,
};
use colored::Colorize;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::iter::once;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MergeResult {
    Base,
    Success,
    UpToDate,
    Conflict,
    Merging,
    Aborted,
    Error(String),
}

impl Display for MergeResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Base => "".normal(),
            Self::Success => "(Ok)".green(),
            Self::UpToDate => "(Up To Date)".green(),
            Self::Conflict => "(Conflict)".red(),
            Self::Merging => "(Merging)".yellow(),
            Self::Aborted => "(Aborted)".red(),
            Self::Error(reason) => format!("(Error: {reason})").red(),
        };
        f.write_str(value.to_string().as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NormalizedMergeStatistic {
    path: NormalizedPath,
    stat: MergeResult,
}

impl NormalizedMergeStatistic {
    pub fn new(path: NormalizedPath, stat: MergeResult) -> Self {
        Self { path, stat }
    }
    pub fn get_path(&self) -> &NormalizedPath {
        &self.path
    }
    pub fn get_stat(&self) -> &MergeResult {
        &self.stat
    }
}

impl ToNormalizedPaths for Vec<NormalizedMergeStatistic> {
    fn to_normalized_paths(&self) -> Vec<NormalizedPath> {
        self.iter().map(|s| s.get_path().clone()).collect()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct MergeStatistic<T: IsGitObject> {
    path: NodePath<T>,
    stat: MergeResult,
}

impl<T: IsGitObject> Display for MergeStatistic<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let stat = self.get_stat().to_string();
        if !stat.is_empty() {
            f.write_str(
                format!("{} {stat}", self.get_path().formatted_with_version(true),).as_str(),
            )
        } else {
            f.write_str(self.get_path().formatted_with_version(true).as_str())
        }
    }
}

impl<T: IsGitObject> MergeStatistic<T> {
    pub fn new(path: NodePath<T>, stat: MergeResult) -> Self {
        Self { path, stat }
    }
    pub fn from_normalized(
        stat: NormalizedMergeStatistic,
        git: &GitInterface,
    ) -> Result<Self, PathAssertionError> {
        let path = git.assert_path::<T>(stat.get_path())?;
        Ok(Self::new(path, stat.get_stat().clone()))
    }
    pub fn to_normalized(&self) -> NormalizedMergeStatistic {
        NormalizedMergeStatistic::new(
            self.path.to_normalized_path_with_version(),
            self.stat.clone(),
        )
    }
    pub fn get_path(&self) -> &NodePath<T> {
        &self.path
    }
    pub fn get_stat(&self) -> &MergeResult {
        &self.stat
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MergeChainStatistic<B: IsGitObject, C: IsGitObject> {
    base: MergeStatistic<B>,
    chain: Vec<MergeStatistic<C>>,
}

impl<B: IsGitObject, C: IsGitObject> MergeChainStatistic<B, C> {
    pub fn new(base: NodePath<B>) -> Self {
        Self {
            base: MergeStatistic::new(base, MergeResult::Base),
            chain: vec![],
        }
    }
    pub fn push(&mut self, stat: MergeStatistic<C>) {
        self.chain.push(stat);
    }
    pub fn fill(&mut self, stats: Vec<MergeStatistic<C>>) {
        for stat in stats {
            self.chain.push(stat)
        }
    }
    pub fn fill_from_normalized(
        &mut self,
        stats: Vec<NormalizedMergeStatistic>,
        git: &GitInterface,
    ) -> Result<(), PathAssertionError> {
        for stat in stats {
            self.push(MergeStatistic::from_normalized(stat, git)?)
        }
        Ok(())
    }
    pub fn to_normalized(&self) -> Vec<NormalizedMergeStatistic> {
        self.iter_chain().map(|s| s.to_normalized()).collect()
    }
    pub fn insert(&mut self, index: usize, stat: MergeStatistic<C>) {
        self.chain.insert(index, stat);
    }
    pub fn remove(&mut self, index: usize) -> MergeStatistic<C> {
        let statistic = self.chain.remove(index);
        statistic
    }
    pub fn get(&self, index: usize) -> Option<&MergeStatistic<C>> {
        self.chain.get(index)
    }
    pub fn get_base(&self) -> &MergeStatistic<B> {
        &self.base
    }
    pub fn replace(&mut self, index: usize, stat: MergeStatistic<C>) {
        self.remove(index);
        self.insert(index, stat);
    }
    pub fn get_chain(&self) -> &Vec<MergeStatistic<C>> {
        &self.chain
    }
    pub fn iter_chain(&self) -> impl Iterator<Item = &MergeStatistic<C>> {
        self.chain.iter()
    }
    pub fn get_n_success(&self) -> usize {
        let success: Vec<&MergeStatistic<C>> = self
            .iter_chain()
            .filter(|s| s.get_stat() == &MergeResult::Success)
            .collect();
        success.len()
    }
    pub fn get_n_conflict(&self) -> usize {
        let all: Vec<&MergeStatistic<C>> = self
            .iter_chain()
            .filter(|s| s.get_stat() == &MergeResult::Conflict)
            .collect();
        all.len()
    }
    pub fn get_n_merges(&self) -> usize {
        let all: Vec<&MergeStatistic<C>> = self
            .iter_chain()
            .filter(|s| match s.get_stat() {
                MergeResult::Success | MergeResult::Conflict | MergeResult::Merging => true,
                _ => false,
            })
            .collect();
        all.len()
    }
    pub fn get_n_up_to_date(&self) -> usize {
        let all: Vec<&MergeStatistic<C>> = self
            .iter_chain()
            .filter(|s| s.get_stat() == &MergeResult::UpToDate)
            .collect();
        all.len()
    }
    pub fn get_n_errors(&self) -> usize {
        let all: Vec<&MergeStatistic<C>> = self
            .iter_chain()
            .filter(|s| {
                match s.get_stat() {
                    MergeResult::Error(_) => true,
                    _ => false,
                }
            })
            .collect();
        all.len()
    }
    pub fn all_up_to_date(&self) -> bool {
        if self.chain.is_empty() || self.chain.len() == 1 {
            true
        } else {
            self.get_n_up_to_date() == self.chain.len()
        }
    }
    pub fn len(&self) -> usize {
        self.chain.len()
    }
    pub fn is_empty(&self) -> bool {
        self.chain.is_empty()
    }
    pub fn contains_conflicts(&self) -> bool {
        self.get_n_conflict() > 0
    }
    pub fn contains_up_to_date(&self) -> bool {
        self.get_n_up_to_date() > 0
    }
    pub fn contains_errors(&self) -> bool {
        self.get_n_errors() > 0
    }
    pub fn display_as_path(&self) -> String {
        vec![&self.base]
            .iter()
            .map(|m| m.to_string())
            .chain(self.chain.iter().map(|stat| stat.to_string()))
            .join(" <- ")
    }
    pub fn display_as_list(&self) -> impl Iterator<Item = String> {
        once(&self.base)
            .map(|m| m.to_string())
            .chain(self.chain.iter().map(|stat| format!(" <- {}", stat)))
    }
}

pub struct MergeChainStatistics<B: IsGitObject, T: IsGitObject> {
    statistics: Vec<MergeChainStatistic<B, T>>,
    total_successes: usize,
    total_conflicts: usize,
    total_errors: usize,
}

impl<B: IsGitObject, T: IsGitObject> MergeChainStatistics<B, T> {
    pub fn new() -> Self {
        Self {
            statistics: vec![],
            total_successes: 0,
            total_conflicts: 0,
            total_errors: 0,
        }
    }
    pub fn fill_from_iter<I: Iterator<Item = MergeChainStatistic<B, T>>>(&mut self, statistics: I) {
        for statistic in statistics {
            self.push(statistic);
        }
    }
    pub fn push(&mut self, statistic: MergeChainStatistic<B, T>) {
        self.total_successes += statistic.get_n_success();
        self.total_conflicts += statistic.get_n_conflict();
        self.total_errors += statistic.get_n_errors();
        self.statistics.push(statistic);
    }
    pub fn iter_all(&self) -> impl Iterator<Item = &MergeChainStatistic<B, T>> {
        self.statistics.iter()
    }
    pub fn iter_conflicts(&self) -> impl Iterator<Item = &MergeChainStatistic<B, T>> {
        self.statistics.iter().filter(|s| s.contains_conflicts())
    }
    pub fn iter_errors(&self) -> impl Iterator<Item = &MergeChainStatistic<B, T>> {
        self.statistics.iter().filter(|s| s.contains_errors())
    }
    pub fn n_ok(&self) -> usize {
        self.total_successes
    }
    pub fn n_conflicts(&self) -> usize {
        self.total_conflicts
    }
    pub fn n_errors(&self) -> usize {
        self.total_errors
    }
}

impl<B: IsGitObject, T: IsGitObject> FromIterator<MergeChainStatistic<B, T>>
    for MergeChainStatistics<B, T>
{
    fn from_iter<I: IntoIterator<Item = MergeChainStatistic<B, T>>>(iter: I) -> Self {
        let mut new = Self::new();
        new.fill_from_iter(iter.into_iter());
        new
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MergeStatisticWeight {
    Simple,
}

impl MergeStatisticWeight {
    pub fn get_weight(&self, statistic: &MergeResult) -> i32 {
        match self {
            Self::Simple => match statistic {
                MergeResult::Base => 0,
                MergeResult::UpToDate => 1,
                MergeResult::Success => 0,
                MergeResult::Conflict => -1,
                MergeResult::Merging => 0,
                MergeResult::Aborted => -10,
                MergeResult::Error(_) => -20,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MergeStatistics<T: IsGitObject> {
    statistics: Vec<MergeStatistic<T>>,
    weights: MergeStatisticWeight,
}

impl<T: IsGitObject> PartialOrd for MergeStatistics<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let my_weights = self.accumulate_weights();
        let their_weights = other.accumulate_weights();
        Some(my_weights.cmp(&their_weights))
    }
}

impl<T: IsGitObject> Ord for MergeStatistics<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl<T: IsGitObject> MergeStatistics<T> {
    pub fn new(weights: MergeStatisticWeight) -> Self {
        Self {
            statistics: vec![],
            weights,
        }
    }
    pub fn push(&mut self, statistic: MergeStatistic<T>) {
        self.statistics.push(statistic);
    }
    pub fn accumulate_weights(&self) -> i32 {
        let mut sum = 0;
        for s in &self.statistics {
            sum += self.weights.get_weight(s.get_stat())
        }
        sum
    }
    pub fn get_lowest(&self) -> &MergeStatistic<T> {
        self.statistics
            .iter()
            .min_by(|a, b| {
                self.weights
                    .get_weight(a.get_stat())
                    .cmp(&self.weights.get_weight(b.get_stat()))
            })
            .unwrap()
    }
}

#[derive(Debug, Clone)]
pub enum CheckMode {
    Merge,
    CherryPick,
}

#[derive(Debug, Clone)]
pub struct ConflictChecker<'a> {
    git: &'a GitInterface,
    mode: CheckMode,
}

impl<'a> ConflictChecker<'a> {
    pub fn new(
        git: &'a GitInterface,
        mode: CheckMode,
    ) -> Self {
        Self { git, mode }
    }

    pub fn check_k_permutations<T: IsGitObject>(
        &self,
        paths: &Vec<NodePath<T>>,
        k: usize,
    ) -> impl Iterator<Item = Result<MergeChainStatistic<T, T>, PathAssertionError>> {
        let iterator = paths.iter().permutations(k).map(|perm| {
            let base = perm[0];
            self.check_chain(&base, &perm[1..].to_vec())
        });
        iterator
    }

    pub fn check_permutations_against_base<B: IsGitObject, T: IsGitObject>(
        &self,
        targets: &Vec<NodePath<T>>,
        base: &NodePath<B>,
        k: usize,
    ) -> impl Iterator<Item = Result<MergeChainStatistic<B, T>, PathAssertionError>> {
        let iterator = targets
            .iter()
            .permutations(k)
            .map(|target| self.check_chain(base, &target));
        iterator
    }

    pub fn check_by_order<T: IsGitObject>(
        &self,
        paths: &Vec<NodePath<T>>,
    ) -> Result<MergeChainStatistic<T, T>, PathAssertionError> {
        let chain: Vec<&NodePath<T>> = paths.iter().collect();
        let base = chain[0];
        self.check_chain(base, &chain[1..].to_vec())
    }

    pub fn check_n_against_permutations<T: IsGitObject>(
        &self,
        n: &'a Vec<NodePath<T>>,
        against: &'a Vec<NodePath<T>>,
        k: &'a usize,
    ) -> impl Iterator<Item = Result<MergeChainStatistic<T, T>, PathAssertionError>> {
        // I don't know why, but k has to be borrowed here
        let iterator = n
            .iter()
            .map(|path| {
                against
                    .iter()
                    .combinations(*k)
                    .map(|mut combination| {
                        combination.push(path);
                        combination
                            .iter()
                            .permutations(*k + 1)
                            .map(|permutations| {
                                let dereferenced = permutations
                                    .iter()
                                    .map(|permutation| **permutation)
                                    .collect::<Vec<_>>();
                                self.check_chain(dereferenced[0], &dereferenced[1..].to_vec())
                            })
                            .collect::<Vec<_>>()
                    })
                    .flatten()
            })
            .flatten();
        iterator
    }

    pub fn clean_up(&mut self) {}

    fn check_chain<B: IsGitObject, C: IsGitObject>(
        &self,
        base: &NodePath<B>,
        chain: &Vec<&NodePath<C>>,
    ) -> Result<MergeChainStatistic<B, C>, PathAssertionError> {
        if chain.len() < 1 {
            panic!("Chain has to contain at least 1 path")
        }
        let mut chain_statistic = MergeChainStatistic::new(base.clone());
        let current_path = self.git.assert_current_node_path::<AnyGitObject>()?;
        self.git.checkout(base)?;
        let area = self.git.get_current_area()?;
        let temporary = area.to_normalized_path() + NormalizedPath::from("tmp");
        self.git.create_branch::<Temporary>(&temporary)?;
        self.git.checkout_raw(&temporary)?;
        let mut skip = false;
        for path in chain.to_vec().into_iter() {
            if skip {
                chain_statistic.push(MergeStatistic::new(path.clone(), MergeResult::Aborted));
            } else {
                let statistic = match self.mode {
                    CheckMode::Merge => {
                        let (statistic, _) = self.git.merge::<Temporary, C>(path.clone())?;
                        if statistic.contains_conflicts() {
                            self.git.abort_merge()?;
                            skip = true;
                        }
                        statistic
                    }
                    CheckMode::CherryPick => {
                        let (statistic, _) = self.git.cherry_pick::<Temporary, C>(path.clone())?;
                        if statistic.contains_conflicts() || statistic.contains_up_to_date() {
                            self.git.abort_cherry_pick()?;
                            skip = true;
                        }
                        statistic
                    }
                };
                chain_statistic.push(statistic.get(0).unwrap().clone());
            }
        }
        self.git.checkout(&current_path)?;
        self.git.delete_branch_no_mut(&temporary)?;
        Ok(chain_statistic)
    }
}

#[derive(Debug, Clone)]
pub struct Conflict2DMatrix {
    matrix: HashMap<
        NodePath<AnyGitObject>,
        HashMap<NodePath<AnyGitObject>, MergeStatistic<AnyGitObject>>,
    >,
}

impl Display for Conflict2DMatrix {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = String::new();
        for (k, v) in &self.matrix {
            result += format!("Key: {}\n", k.formatted_with_version(true)).as_str();
            for (_, value) in v {
                result += format!("  Value: {}\n", value).as_str();
            }
        }
        f.write_str(result.as_str())
    }
}

impl Conflict2DMatrix {
    pub fn new(statistics: &MergeChainStatistics<AnyGitObject, AnyGitObject>) -> Self {
        let mut matrix: HashMap<
            NodePath<AnyGitObject>,
            HashMap<NodePath<AnyGitObject>, MergeStatistic<AnyGitObject>>,
        > = HashMap::new();
        for chain in statistics.iter_all() {
            if chain.len() > 1 {
                panic!("Matrix only supports 2 dimensions")
            }
            let base = chain.get_base();
            let second = chain.get(0).unwrap();
            if !matrix.contains_key(base.get_path()) {
                matrix.insert(base.get_path().clone(), HashMap::new());
            }
            matrix
                .get_mut(base.get_path())
                .unwrap()
                .insert(second.get_path().clone(), second.clone());
        }
        Self { matrix }
    }

    pub fn predict_conflicts<B: IsGitObject, C: IsGitObject>(
        &self,
        base: &NodePath<B>,
        order: &Vec<NodePath<C>>,
    ) -> Option<MergeChainStatistic<B, C>> {
        let mut final_path = vec![(
            base.try_convert_to()?,
            MergeStatistics::new(MergeStatisticWeight::Simple),
        )];
        for path in order.iter() {
            let converted = path.try_convert_to()?;
            let voters = final_path.iter().map(|(k, _)| k.clone()).collect();
            let votes = self.calculate_votes(&voters, &vec![converted.clone()]);
            let vote = votes.get(&converted).unwrap();
            final_path.push((converted, vote.clone()));
        }
        self.statistics_from_votes(&final_path)
    }

    pub fn estimate_best_path<B: IsGitObject, C: IsGitObject>(
        &self,
        base_path: &NodePath<B>,
    ) -> Option<MergeChainStatistic<B, C>> {
        let mut missing: Vec<NodePath<AnyGitObject>> = self.matrix.keys().cloned().collect();
        let start = base_path.try_convert_to()?;
        missing.retain(|k| k != &start);
        let mut final_path = vec![(start, MergeStatistics::new(MergeStatisticWeight::Simple))];
        while missing.len() > 0 {
            let voters = final_path.iter().map(|(k, _)| k.clone()).collect();
            let votes = Self::reverse_votes(self.calculate_votes(&voters, &missing));
            let max_vote = votes.keys().max().unwrap();
            let max_candidates = &votes[&max_vote];
            let winner = match max_candidates.len() {
                0 => {
                    panic!("Empty candidates should not be possible")
                }
                1 => max_candidates[0].clone(),
                _ => {
                    let start = max_candidates[0].clone();
                    let compatibility = self.calculate_forward_compatibility(&start, &missing);
                    let mut highest_compatible = (start, compatibility);
                    for candidate in max_candidates[1..].iter() {
                        let compatibility =
                            self.calculate_forward_compatibility(&candidate, &missing);
                        if compatibility > highest_compatible.1 {
                            highest_compatible = (candidate.clone(), compatibility);
                        }
                    }
                    highest_compatible.0
                }
            };
            let index: usize = missing
                .iter()
                .enumerate()
                .find_map(|(index, e)| if e == &winner { Some(index) } else { None })
                .unwrap();
            missing.remove(index);
            final_path.push((winner, max_vote.clone()));
        }
        self.statistics_from_votes(&final_path)
    }

    fn calculate_forward_compatibility(
        &self,
        element: &NodePath<AnyGitObject>,
        missing: &Vec<NodePath<AnyGitObject>>,
    ) -> MergeStatistics<AnyGitObject> {
        let table = &self.matrix[element];
        let mut statistics = MergeStatistics::new(MergeStatisticWeight::Simple);
        for statistic in table.iter().filter_map(|(k, v)| {
            if missing.contains(k) {
                Some(v.clone())
            } else {
                None
            }
        }) {
            statistics.push(statistic)
        }
        statistics
    }

    fn calculate_votes(
        &self,
        voters: &Vec<NodePath<AnyGitObject>>,
        targets: &Vec<NodePath<AnyGitObject>>,
    ) -> HashMap<NodePath<AnyGitObject>, MergeStatistics<AnyGitObject>> {
        let mut votes: HashMap<NodePath<AnyGitObject>, MergeStatistics<AnyGitObject>> =
            HashMap::new();
        for candidate in targets.iter() {
            let mut statistics = MergeStatistics::new(MergeStatisticWeight::Simple);
            for p in voters.iter() {
                let statistic = self.matrix[p].get(candidate).unwrap();
                statistics.push(statistic.clone());
            }
            votes.insert(candidate.clone(), statistics);
        }
        votes
    }

    fn reverse_votes(
        votes: HashMap<NodePath<AnyGitObject>, MergeStatistics<AnyGitObject>>,
    ) -> HashMap<MergeStatistics<AnyGitObject>, Vec<NodePath<AnyGitObject>>> {
        let mut reversed: HashMap<MergeStatistics<AnyGitObject>, Vec<NodePath<AnyGitObject>>> =
            HashMap::new();
        for (path, vote) in votes.iter() {
            if reversed.contains_key(vote) {
                reversed.get_mut(vote).unwrap().push(path.clone());
            } else {
                reversed.insert(vote.clone(), vec![path.clone()]);
            }
        }
        reversed
    }

    fn statistics_from_votes<B: IsGitObject, C: IsGitObject>(
        &self,
        votes: &Vec<(NodePath<AnyGitObject>, MergeStatistics<AnyGitObject>)>,
    ) -> Option<MergeChainStatistic<B, C>> {
        let mut chain_statistic = MergeChainStatistic::new(votes[0].0.try_convert_to()?);
        for (index, (_, vote)) in votes.iter().enumerate() {
            if index != 0 {
                let lowest = vote.get_lowest();
                chain_statistic.push(MergeStatistic::new(
                    lowest.get_path().try_convert_to()?,
                    lowest.get_stat().clone(),
                ));
            };
        }
        Some(chain_statistic)
    }
}

pub struct ConflictAnalyzer<'a> {
    checker: ConflictChecker<'a>,
    logger: &'a TanglLogger,
}

impl<'a> ConflictAnalyzer<'a> {
    pub fn new(checker: ConflictChecker<'a>, logger: &'a TanglLogger) -> Self {
        Self { checker, logger }
    }

    pub fn calculate_2d_heuristics_matrix_with_merge_base(
        &mut self,
        paths: &Vec<NodePath<AnyGitObject>>,
        base: &NodePath<AnyGitObject>,
    ) -> Result<Conflict2DMatrix, PathAssertionError> {
        let mut statistics = MergeChainStatistics::new();

        let mut conflicting_with_base: Vec<NodePath<AnyGitObject>> = vec![];
        self.logger.debug("Checking against base pairwise");
        for s in self.checker.check_permutations_against_base(paths, base, 1) {
            let result = s?;
            self.logger.debug(result.display_as_path());
            statistics.push(result.clone());
            if result.contains_conflicts() {
                conflicting_with_base
                    .push(result.get(0).unwrap().get_path().clone());
            }
        }
        let to_test_with_base: Vec<NodePath<AnyGitObject>> = paths
            .iter()
            .filter(|path| !conflicting_with_base.contains(&path))
            .cloned()
            .collect();
        let to_test_without_base: Vec<NodePath<AnyGitObject>> = paths
            .iter()
            .filter(|path| conflicting_with_base.contains(&path))
            .cloned()
            .collect();
        let to_test_against_without_base: Vec<NodePath<AnyGitObject>> = paths
            .iter()
            .filter(|path| !to_test_without_base.contains(&path))
            .cloned()
            .collect();

        self.logger.debug("Checking successful against base");
        for with_base in self
            .checker
            .check_permutations_against_base(&to_test_with_base, &base, 2)
        {
            let result = with_base?;
            self.logger.debug(result.display_as_path());
            let second = result.get(0).unwrap();
            let third = result.get(1).unwrap();
            let mut new = MergeChainStatistic::new(second.get_path().clone());
            new.push(third.clone());
            statistics.push(new);
        }
        self.logger.debug("Checking conflicting without base");
        for without_base in self
            .checker
            .check_n_against_permutations(&to_test_without_base, &to_test_against_without_base, &1)
        {
            let result = without_base?;
            statistics.push(result);
        }
        self.checker.clean_up();
        let matrix = Conflict2DMatrix::new(&statistics);
        Ok(matrix)
    }
}
