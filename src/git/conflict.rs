use crate::git::error::InvalidPathError;
use crate::git::interface::GitInterface;
use crate::logging::TanglLogger;
use crate::model::{AnyGitObject, IsGitObject, NodePath, NormalizedPath, ToNormalizedPath};
use colored::Colorize;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};


#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MergeResult {
    Base,
    Success,
    UpToDate,
    Conflict,
    Merging,
    Aborted,
}

impl Display for MergeResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Base => "(Base)".blue(),
            Self::Success => "(Ok)".green(),
            Self::UpToDate => "(Up To Date)".green(),
            Self::Conflict => "(Conflict)".red(),
            Self::Merging => "(Merging)".yellow(),
            Self::Aborted => "(Aborted)".red(),
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

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct MergeStatistic<T: IsGitObject> {
    path: NodePath<T>,
    stat: MergeResult,
}

impl<T: IsGitObject> MergeStatistic<T> {
    pub fn new(path: NodePath<T>, stat: MergeResult) -> Self {
        Self { path, stat }
    }
    pub fn from_normalized(stat: NormalizedMergeStatistic, git: &GitInterface) -> Result<Self, InvalidPathError> {
        let path = git.assert_path::<T>(stat.get_path())?;
        Ok(Self::new(path, stat.get_stat().clone()))
    }
    pub fn to_normalized(&self) -> NormalizedMergeStatistic {
        NormalizedMergeStatistic::new(self.path.to_normalized_path(), self.stat.clone())
    }
    pub fn get_path(&self) -> &NodePath<T> {
        &self.path
    }
    pub fn get_stat(&self) -> &MergeResult {
        &self.stat
    }
}

impl<T: IsGitObject> Display for MergeStatistic<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{} {}", self.get_stat(), self.get_path()).as_str())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MergeChainStatistic<B: IsGitObject, C: IsGitObject> {
    base: MergeStatistic<B>,
    chain: Vec<MergeStatistic<C>>,
    n_success: usize,
    n_up_to_date: usize,
    n_conflict: usize,
}

impl<B: IsGitObject, C: IsGitObject> MergeChainStatistic<B, C> {
    pub fn new(base: NodePath<B>) -> Self {
        Self {
            base: MergeStatistic::new(base, MergeResult::Base),
            chain: vec![],
            n_success: 0,
            n_up_to_date: 0,
            n_conflict: 0,
        }
    }

    fn add_to_internal_counters(&mut self, stat: &MergeStatistic<C>) {
        match stat.get_stat() {
            MergeResult::Success => self.n_success += 1,
            MergeResult::UpToDate => self.n_up_to_date += 1,
            MergeResult::Conflict => self.n_conflict += 1,
            _ => {}
        }
    }

    fn subtract_from_internal_counters(&mut self, stat: &MergeStatistic<C>) {
        match stat.get_stat() {
            MergeResult::Success => self.n_success -= 1,
            MergeResult::UpToDate => self.n_up_to_date -= 1,
            MergeResult::Conflict => self.n_conflict -= 1,
            _ => {}
        }
    }
    pub fn push(&mut self, stat: MergeStatistic<C>) {
        self.add_to_internal_counters(&stat);
        self.chain.push(stat);
    }
    pub fn fill(&mut self, stats: Vec<MergeStatistic<C>>) {
        for stat in stats {
            self.chain.push(stat)
        }
    }
    pub fn fill_from_normalized(&mut self, stats: Vec<NormalizedMergeStatistic>, git: &GitInterface) -> Result<(), InvalidPathError> {
        for stat in stats {
            self.push(MergeStatistic::from_normalized(stat, git)?)
        };
        Ok(())
    }
    pub fn insert(&mut self, index: usize, stat: MergeStatistic<C>) {
        self.add_to_internal_counters(&stat);
        self.chain.insert(index, stat);
    }
    pub fn remove(&mut self, index: usize) -> MergeStatistic<C> {
        let statistic = self.chain.remove(index);
        self.subtract_from_internal_counters(&statistic);
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
    pub fn iter(&self) -> impl Iterator<Item = &MergeStatistic<C>> {
        self.chain.iter()
    }
    pub fn get_n_success(&self) -> usize {
        self.n_success
    }
    pub fn get_n_merges(&self) -> usize {
        self.n_success + self.n_conflict
    }
    pub fn get_n_up_to_date(&self) -> usize {
        self.n_up_to_date
    }
    pub fn all_up_to_date(&self) -> bool {
        if self.chain.is_empty() || self.chain.len() == 1 {
            true
        } else {
            self.n_up_to_date == self.chain.len() - 1
        }
    }
    pub fn len(&self) -> usize {
        self.chain.len()
    }
    pub fn is_empty(&self) -> bool {
        self.chain.is_empty()
    }
    pub fn get_n_conflict(&self) -> usize {
        self.n_conflict
    }
    pub fn contains_conflicts(&self) -> bool {
        self.n_conflict > 0
    }
    pub fn display_as_path(&self) -> String {
        self.chain.iter().map(|stat| stat.to_string()).join(" <- ")
    }
    pub fn display_as_list(&self) -> impl Iterator<Item = String> {
        self.chain.iter().map(|stat| match stat.get_stat() {
            MergeResult::Base => stat.to_string(),
            _ => format!(" <- {}", stat),
        })
    }
}


pub struct MergeChainStatistics<B: IsGitObject, T: IsGitObject> {
    statistics: Vec<MergeChainStatistic<B, T>>,
    total_successes: usize,
    total_conflicts: usize,
}

impl<B: IsGitObject, T: IsGitObject> MergeChainStatistics<B, T> {
    pub fn new() -> Self {
        Self {
            statistics: vec![],
            total_successes: 0,
            total_conflicts: 0,
        }
    }
    pub fn fill_from_iter<I: Iterator<Item = MergeChainStatistic<B, T>>>(&mut self, statistics: I) {
        for statistic in statistics {
            self.push(statistic);
        }
    }
    pub fn push(&mut self, statistic: MergeChainStatistic<B, T>) {
        self.total_successes += statistic.n_success;
        self.total_conflicts += statistic.n_conflict;
        self.statistics.push(statistic);
    }
    pub fn iter_all(&self) -> impl Iterator<Item = &MergeChainStatistic<B, T>> {
        self.statistics.iter()
    }
    pub fn iter_conflicts(&self) -> impl Iterator<Item = &MergeChainStatistic<B, T>> {
        self.statistics.iter().filter(|s| s.contains_conflicts())
    }
    pub fn n_ok(&self) -> usize {
        self.total_successes
    }
    pub fn n_conflicts(&self) -> usize {
        self.total_conflicts
    }
}

impl<B: IsGitObject, T: IsGitObject> FromIterator<MergeChainStatistic<B, T>> for MergeChainStatistics<B, T> {
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
            Self::Simple => {
                match statistic {
                    MergeResult::Base => 0,
                    MergeResult::UpToDate => 1,
                    MergeResult::Success => 0,
                    MergeResult::Conflict => -1,
                    MergeResult::Merging => 0,
                    MergeResult::Aborted => -10,
                }
            }
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
        Self { statistics: vec![], weights }
    }
    pub fn push(&mut self, statistic: MergeStatistic<T>) {
        self.statistics.push(statistic);
    }
    pub fn accumulate_weights(&self) -> i32 {
        let mut sum = 0;
        for s in &self.statistics {
            sum += self.weights.get_weight(s.get_stat())
        };
        sum
    }
    pub fn get_lowest(&self) -> &MergeStatistic<T> {
        self.statistics.iter().min_by(|a, b| {
            self.weights.get_weight(a.get_stat()).cmp(&self.weights.get_weight(b.get_stat()))
        }).unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct ConflictChecker<'a> {
    interface: &'a GitInterface,
}

impl<'a> ConflictChecker<'a> {
    pub fn new(interface: &'a GitInterface) -> Self {
        Self { interface }
    }

    pub fn check_k_permutations<T: IsGitObject>(
        &self,
        paths: &Vec<NodePath<T>>,
        k: usize,
    ) -> impl Iterator<Item = Result<MergeChainStatistic<T, T>, InvalidPathError>> {
        let iterator = paths
            .iter()
            .permutations(k)
            .map(|perm| {
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
    ) -> impl Iterator<Item = Result<MergeChainStatistic<B, T>, InvalidPathError>> {
        let iterator = targets.iter().permutations(k).map(|target| {
            self.check_chain(base, &target)
        });
        iterator
    }

    pub fn check_by_order<T: IsGitObject>(
        &self,
        paths: &Vec<NodePath<T>>,
    ) -> Result<MergeChainStatistic<T, T>, InvalidPathError> {
        let chain: Vec<&NodePath<T>> = paths.iter().collect();
        let base = chain[0];
        self.check_chain(base, &chain[1..].to_vec())
    }

    pub fn check_n_against_permutations<T: IsGitObject>(
        &self,
        n: &'a Vec<NodePath<T>>,
        against: &'a Vec<NodePath<T>>,
        k: &'a usize,
    ) -> impl Iterator<Item = Result<MergeChainStatistic<T, T>, InvalidPathError>> {
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
    ) -> Result<MergeChainStatistic<B, C>, InvalidPathError> {
        if chain.len() < 1 {
            panic!("Chain has to contain at least 1 path")
        }
        let mut chain_statistic = MergeChainStatistic::new(base.clone());
        let current_path = self
            .interface
            .assert_current_node_path::<AnyGitObject>()?;
        self.interface.checkout(base)?;
        let temporary = NormalizedPath::from("tmp");
        self.interface.create_branch_no_mut(&temporary)?;
        self.interface.checkout_raw(&temporary)?;
        let mut skip = false;
        for path in chain[1..].to_vec().into_iter() {
            if skip {
                chain_statistic.push(MergeStatistic::new(path.clone(), MergeResult::Aborted));
            } else {
                let (statistic, _) = self.interface.merge::<B, C>(path.clone())?;
                if statistic.contains_conflicts() {
                    self.interface.abort_merge()?;
                    skip = true;
                }
                chain_statistic.push(statistic.get(1).unwrap().clone());
            }
        }
        self.interface.checkout(&current_path)?;
        self.interface.delete_branch_no_mut(&temporary)?;
        Ok(chain_statistic)
    }
}

#[derive(Debug, Clone)]
pub struct Conflict2DMatrix<T: IsGitObject> {
    matrix: HashMap<NodePath<T>, HashMap<NodePath<T>, MergeStatistic<T>>>,
}

impl<T: IsGitObject> Conflict2DMatrix<T> {
    pub fn new(statistics: &MergeChainStatistics<T, T>) -> Self {
        let mut matrix: HashMap<NodePath<T>, HashMap<NodePath<T>, MergeStatistic<T>>> =
            HashMap::new();
        for chain in statistics.iter_all() {
            if chain.len() > 2 {
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

    pub fn predict_conflicts<B: IsGitObject>(&self, base: &NodePath<B>, order: &Vec<NodePath<T>>) -> Option<MergeChainStatistic<B, T>> {
        let mut final_path = vec![(base.try_convert_to()?, MergeStatistics::new(MergeStatisticWeight::Simple))];
        for path in order[1..].iter() {
            let voters = final_path.iter().map(|(k, _)| k.clone()).collect();
            let votes = self.calculate_votes(&voters, &vec![path.clone()]);
            let vote = votes.get(&path).unwrap();
            final_path.push((path.clone(), vote.clone()));
        }
        self.statistics_from_votes(&final_path)
    }

    pub fn estimate_best_path<B: IsGitObject>(&self, base_path: &NodePath<B>) -> Option<MergeChainStatistic<B, T>> {
        let mut missing: Vec<NodePath<T>> = self.matrix.keys().cloned().collect();
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
        element: &NodePath<T>,
        missing: &Vec<NodePath<T>>,
    ) -> MergeStatistics<T> {
        let table = &self.matrix[element];
        let mut statistics = MergeStatistics::new(MergeStatisticWeight::Simple);
        for statistic in table
            .iter()
            .filter_map(|(k, v)| if missing.contains(k) { Some(v.clone()) } else { None }) {
            statistics.push(statistic)
        };
        statistics
    }

    fn calculate_votes(
        &self,
        voters: &Vec<NodePath<T>>,
        targets: &Vec<NodePath<T>>,
    ) -> HashMap<NodePath<T>, MergeStatistics<T>> {
        let mut votes: HashMap<NodePath<T>, MergeStatistics<T>> = HashMap::new();
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

    fn reverse_votes(votes: HashMap<NodePath<T>, MergeStatistics<T>>) -> HashMap<MergeStatistics<T>, Vec<NodePath<T>>> {
        let mut reversed: HashMap<MergeStatistics<T>, Vec<NodePath<T>>> = HashMap::new();
        for (path, vote) in votes.iter() {
            if reversed.contains_key(vote) {
                reversed.get_mut(vote).unwrap().push(path.clone());
            } else {
                reversed.insert(vote.clone(), vec![path.clone()]);
            }
        }
        reversed
    }

    fn statistics_from_votes<B: IsGitObject>(&self, votes: &Vec<(NodePath<T>, MergeStatistics<T>)>) -> Option<MergeChainStatistic<B, T>> {
        let mut chain_statistic = MergeChainStatistic::new(votes[0].0.try_convert_to()?);
        for (index, (_, vote)) in votes.iter().enumerate() {
            if index != 0 {
                chain_statistic.push(vote.get_lowest().clone());
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

    pub fn calculate_2d_heuristics_matrix_with_merge_base<C: IsGitObject>(
        &mut self,
        paths: &Vec<NodePath<C>>,
        base: &NodePath<C>,
    ) -> Result<Conflict2DMatrix<C>, InvalidPathError> {
        let mut statistics = MergeChainStatistics::new();

        let mut conflicting_with_base: Vec<NodePath<C>> = vec![];
        self.logger.debug("Checking against base pairwise");
        for s in self.checker.check_permutations_against_base(paths, base, 1) {
            let result = s?;
            self.logger.debug(result.display_as_path());
            statistics.push(result.clone());
            if result.contains_conflicts() {
                conflicting_with_base.push(result.get_chain().get(1).unwrap().get_path().clone());
            }
        }
        let to_test_with_base: Vec<NodePath<C>> = paths
            .iter()
            .filter(|path| !conflicting_with_base.contains(&path))
            .cloned()
            .collect();
        let _to_test_without_base: Vec<NodePath<C>> = paths
            .iter()
            .filter(|path| conflicting_with_base.contains(&path))
            .cloned()
            .collect();

        self.logger.debug("Checking successful against base");
        for with_base in self
            .checker
            .check_permutations_against_base(&to_test_with_base, &base, 2)
        {
            let mut result = with_base?;
            self.logger.debug(result.display_as_path());
            result.remove(0);
            let second = result.remove(0);
            let new = MergeStatistic::new(second.get_path().clone(), MergeResult::Base);
            result.insert(0, new);
            statistics.push(result);
        }
        self.logger.debug("Checking conflicting without base");
        // TODO
        self.checker.clean_up();
        let matrix = Conflict2DMatrix::new(&statistics);
        Ok(matrix)
    }
}
