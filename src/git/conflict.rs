use crate::git::error::GitError;
use crate::git::interface::GitInterface;
use crate::logging::TanglLogger;
use crate::model::{AnyHasBranch, NodePath, QualifiedPath, ToQualifiedPath};
use colored::Colorize;
use itertools::Itertools;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeSuccess {
    path: QualifiedPath,
}
impl MergeSuccess {
    pub fn new(path: QualifiedPath) -> Self {
        Self { path }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergePending {
    path: QualifiedPath,
}
impl MergePending {
    pub fn new(path: QualifiedPath) -> Self {
        Self { path }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeConflict {
    path: QualifiedPath,
}
impl MergeConflict {
    pub fn new(path: QualifiedPath) -> Self {
        Self { path }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeStatistic {
    Base(QualifiedPath),
    Success(MergeSuccess),
    Conflict(MergeConflict),
    Merging(MergePending),
    Skipped(QualifiedPath),
}

impl MergeStatistic {
    pub fn get_path(&self) -> &QualifiedPath {
        match self {
            MergeStatistic::Base(path) | MergeStatistic::Skipped(path) => path,
            MergeStatistic::Success(success) => &success.path,
            MergeStatistic::Merging(pending) => &pending.path,
            MergeStatistic::Conflict(conflict) => &conflict.path,
        }
    }
}

impl Display for MergeStatistic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value: String = match self {
            Self::Base(path) => path.to_string().blue().to_string(),
            Self::Success(success) => {
                format!("{} {}", success.path.to_string().green(), "(Ok)".green())
            }
            Self::Conflict(conflict) => {
                format!("{} {}", conflict.path.to_string().red(), "(Conflict)".red())
            }
            Self::Merging(pending) => {
                format!(
                    "{} {}",
                    pending.path.to_string().yellow(),
                    "(Merging)".yellow()
                )
            }
            Self::Skipped(path) => path.to_string().normal().strikethrough().to_string(),
        };
        f.write_str(value.as_str())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MergeChainStatistic {
    chain: Vec<MergeStatistic>,
    n_success: usize,
    n_conflict: usize,
}

impl From<&MergeChainStatistic> for Vec<QualifiedPath> {
    fn from(value: &MergeChainStatistic) -> Self {
        value
            .get_chain()
            .iter()
            .map(|p| p.get_path().clone())
            .collect()
    }
}

impl From<Vec<MergeStatistic>> for MergeChainStatistic {
    fn from(value: Vec<MergeStatistic>) -> Self {
        let mut new = Self::new();
        for v in value {
            new.push(v)
        }
        new
    }
}

impl MergeChainStatistic {
    pub fn new() -> Self {
        Self {
            chain: vec![],
            n_success: 0,
            n_conflict: 0,
        }
    }

    pub fn push(&mut self, stat: MergeStatistic) {
        match &stat {
            MergeStatistic::Success(_) => self.n_success += 1,
            MergeStatistic::Conflict(_) => self.n_conflict += 1,
            _ => {}
        }
        self.chain.push(stat);
    }

    pub fn insert(&mut self, index: usize, stat: MergeStatistic) {
        match &stat {
            MergeStatistic::Success(_) => self.n_success += 1,
            MergeStatistic::Conflict(_) => self.n_conflict += 1,
            _ => {}
        }
        self.chain.insert(index, stat);
    }

    pub fn remove(&mut self, index: usize) -> MergeStatistic {
        let statistic = self.chain.remove(index);
        match &statistic {
            MergeStatistic::Success(_) => self.n_success -= 1,
            MergeStatistic::Conflict(_) => self.n_conflict -= 1,
            _ => {}
        }
        statistic
    }

    pub fn get(&self, index: usize) -> Option<&MergeStatistic> {
        self.chain.get(index)
    }

    pub fn replace(&mut self, index: usize, stat: MergeStatistic) {
        self.remove(index);
        self.insert(index, stat);
    }

    pub fn get_chain(&self) -> &Vec<MergeStatistic> {
        &self.chain
    }
    pub fn get_n_success(&self) -> usize {
        self.n_success
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
}

pub struct MergeChainStatistics {
    statistics: Vec<MergeChainStatistic>,
    total_successes: usize,
    total_conflicts: usize,
}

impl MergeChainStatistics {
    pub fn new() -> Self {
        Self {
            statistics: vec![],
            total_successes: 0,
            total_conflicts: 0,
        }
    }
    pub fn fill_from_iter<T: Iterator<Item = MergeChainStatistic>>(&mut self, statistics: T) {
        for statistic in statistics {
            self.push(statistic);
        }
    }
    pub fn push(&mut self, statistic: MergeChainStatistic) {
        self.total_successes += statistic.n_success;
        self.total_conflicts += statistic.n_conflict;
        self.statistics.push(statistic);
    }
    pub fn iter_all(&self) -> impl Iterator<Item = &MergeChainStatistic> {
        self.statistics.iter()
    }
    pub fn iter_conflicts(&self) -> impl Iterator<Item = &MergeChainStatistic> {
        self.statistics.iter().filter(|s| s.contains_conflicts())
    }
    pub fn n_ok(&self) -> usize {
        self.total_successes
    }
    pub fn n_conflicts(&self) -> usize {
        self.total_conflicts
    }
}

impl FromIterator<MergeChainStatistic> for MergeChainStatistics {
    fn from_iter<T: IntoIterator<Item = MergeChainStatistic>>(iter: T) -> Self {
        let mut new = Self::new();
        new.fill_from_iter(iter.into_iter());
        new
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

    pub fn check_k_permutations(
        &self,
        paths: &Vec<NodePath<AnyHasBranch>>,
        k: usize,
    ) -> impl Iterator<Item = Result<MergeChainStatistic, GitError>> {
        let iterator = paths
            .iter()
            .permutations(k)
            .map(|perm| self.check_chain(&perm));
        iterator
    }

    pub fn check_permutations_against_base(
        &self,
        targets: &Vec<NodePath<AnyHasBranch>>,
        base: &NodePath<AnyHasBranch>,
        k: usize,
    ) -> impl Iterator<Item = Result<MergeChainStatistic, GitError>> {
        let iterator = targets.iter().permutations(k).map(|target| {
            let mut to_check: Vec<&NodePath<AnyHasBranch>> = vec![];
            to_check.push(base);
            to_check.extend(target);
            self.check_chain(&to_check)
        });
        iterator
    }

    pub fn check_by_order(
        &self,
        paths: &Vec<NodePath<AnyHasBranch>>,
    ) -> Result<MergeChainStatistic, GitError> {
        let chain: Vec<&NodePath<AnyHasBranch>> = paths.iter().collect();
        self.check_chain(&chain)
    }

    pub fn check_n_against_permutations(
        &self,
        n: &'a Vec<NodePath<AnyHasBranch>>,
        against: &'a Vec<NodePath<AnyHasBranch>>,
        k: &'a usize,
    ) -> impl Iterator<Item = Result<MergeChainStatistic, GitError>> {
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
                                self.check_chain(&dereferenced)
                            })
                            .collect::<Vec<_>>()
                    })
                    .flatten()
            })
            .flatten();
        iterator
    }

    pub fn clean_up(&mut self) {}

    fn check_chain(
        &self,
        chain: &Vec<&NodePath<AnyHasBranch>>,
    ) -> Result<MergeChainStatistic, GitError> {
        if chain.len() < 2 {
            panic!("Chain has to contain at least 2 paths")
        }
        let mut chain_statistic = MergeChainStatistic::new();
        let current_path = self
            .interface
            .assert_current_node_path::<AnyHasBranch>()
            .unwrap();
        let base = chain[0];
        chain_statistic.push(MergeStatistic::Base(base.to_qualified_path()));
        self.interface.checkout(base)?;
        let temporary = QualifiedPath::from("tmp");
        self.interface.create_branch_no_mut(&temporary)?;
        self.interface.checkout_raw(&temporary)?;
        let mut skip = false;
        for path in chain[1..].iter() {
            if skip {
                chain_statistic.push(MergeStatistic::Skipped(path.to_qualified_path()));
            } else {
                let (statistic, _) = self.interface.merge(path)?;
                match &statistic {
                    MergeStatistic::Conflict(_) => {
                        self.interface.abort_merge()?;
                        skip = true;
                    }
                    _ => {}
                }
                chain_statistic.push(statistic);
            }
        }
        self.interface.checkout(&current_path)?;
        self.interface.delete_branch_no_mut(&temporary)?;
        Ok(chain_statistic)
    }
}

#[derive(Debug, Clone)]
pub struct Conflict2DMatrix {
    matrix: HashMap<QualifiedPath, HashMap<QualifiedPath, i32>>,
    all_keys: Vec<QualifiedPath>,
}

impl Conflict2DMatrix {
    pub fn initialize(paths: &Vec<QualifiedPath>) -> Self {
        let mut matrix: HashMap<QualifiedPath, HashMap<QualifiedPath, i32>> = HashMap::new();
        for combinations in paths.iter().combinations(2) {
            let l = combinations[0];
            let r = combinations[1];

            if matrix.contains_key(&l) {
                matrix.get_mut(l).unwrap().insert(r.clone(), 0);
            } else {
                let mut map: HashMap<QualifiedPath, i32> = HashMap::new();
                map.insert(r.clone(), 0);
                matrix.insert(l.clone(), map);
            }

            if matrix.contains_key(&r) {
                matrix.get_mut(r).unwrap().insert(l.clone(), 0);
            } else {
                let mut map: HashMap<QualifiedPath, i32> = HashMap::new();
                map.insert(l.clone(), 0);
                matrix.insert(r.clone(), map);
            }
        }
        Self {
            matrix,
            all_keys: paths.clone(),
        }
    }

    pub fn insert(&mut self, statistic: &MergeChainStatistic) {
        let base = statistic.get_chain().get(0).unwrap();
        let reference = statistic.get_chain().get(1).unwrap();
        match reference {
            MergeStatistic::Conflict(conflict) => {
                self.matrix
                    .get_mut(base.get_path())
                    .unwrap()
                    .insert(conflict.path.clone(), -1);
            }
            _ => {}
        }
    }

    pub fn predict_conflicts(&self, order: &Vec<QualifiedPath>) -> MergeChainStatistic {
        let base = order.get(0).unwrap().clone();
        let mut final_path = vec![(base, 0)];
        for path in order[1..].iter() {
            let voters = final_path.iter().map(|(k, _)| k.clone()).collect();
            let votes = self.calculate_votes(&voters, &vec![path.clone()]);
            let vote = votes.get(&path).unwrap();
            final_path.push((path.clone(), *vote));
        }
        self.statistics_from_votes(&final_path)
    }

    pub fn calculate_best_path_greedy(&self, base_path: &QualifiedPath) -> MergeChainStatistic {
        let mut missing = self.all_keys.clone();
        let start = base_path.clone();
        missing.retain(|k| k != base_path);
        let mut final_path = vec![(start, 0)];
        while missing.len() > 0 {
            let voters = final_path.iter().map(|(k, _)| k.clone()).collect();
            let votes = Self::reverse_votes(&self.calculate_votes(&voters, &missing));
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
        element: &QualifiedPath,
        missing: &Vec<QualifiedPath>,
    ) -> i32 {
        let table = &self.matrix[element];
        table
            .iter()
            .filter_map(|(k, v)| if missing.contains(k) { Some(*v) } else { None })
            .sum::<i32>()
    }

    fn calculate_votes(
        &self,
        voters: &Vec<QualifiedPath>,
        targets: &Vec<QualifiedPath>,
    ) -> HashMap<QualifiedPath, i32> {
        let mut votes: HashMap<QualifiedPath, i32> = HashMap::new();
        for candidate in targets.iter() {
            let mut vote = 0;
            for p in voters.iter() {
                vote += self.matrix[p].get(candidate).unwrap();
            }
            votes.insert(candidate.clone(), vote);
        }
        votes
    }

    fn reverse_votes(votes: &HashMap<QualifiedPath, i32>) -> HashMap<i32, Vec<QualifiedPath>> {
        let mut reversed: HashMap<i32, Vec<QualifiedPath>> = HashMap::new();
        for (path, vote) in votes.iter() {
            if reversed.contains_key(vote) {
                reversed.get_mut(vote).unwrap().push(path.clone());
            } else {
                reversed.insert(*vote, vec![path.clone()]);
            }
        }
        reversed
    }

    fn statistics_from_votes(&self, votes: &Vec<(QualifiedPath, i32)>) -> MergeChainStatistic {
        let mut chain_statistic = MergeChainStatistic::new();
        for (index, (path, vote)) in votes.iter().enumerate() {
            let statistic = if index == 0 {
                MergeStatistic::Base(path.clone())
            } else {
                match vote {
                    0 => {
                        let success = MergeSuccess::new(path.clone());
                        MergeStatistic::Success(success)
                    }
                    _ => {
                        let conflict = MergeConflict::new(path.clone());
                        MergeStatistic::Conflict(conflict)
                    }
                }
            };
            chain_statistic.push(statistic);
        }
        chain_statistic
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
        paths: &Vec<NodePath<AnyHasBranch>>,
        base: &NodePath<AnyHasBranch>,
    ) -> Result<Conflict2DMatrix, GitError> {
        let mut all = vec![base];
        all.extend(paths);
        let mut matrix =
            Conflict2DMatrix::initialize(&all.iter().map(|p| p.to_qualified_path()).collect());

        let mut conflicting_with_base: Vec<QualifiedPath> = vec![];
        self.logger.debug("Checking against base pairwise");
        for s in self.checker.check_permutations_against_base(paths, base, 1) {
            let result = s?;
            self.logger.debug(result.display_as_path());
            if result.contains_conflicts() {
                conflicting_with_base.push(result.get_chain().get(1).unwrap().get_path().clone());
            }
        }
        let to_test_with_base: Vec<NodePath<AnyHasBranch>> = paths
            .iter()
            .filter(|path| !conflicting_with_base.contains(&path.to_qualified_path()))
            .cloned()
            .collect();
        let _to_test_without_base: Vec<NodePath<AnyHasBranch>> = paths
            .iter()
            .filter(|path| conflicting_with_base.contains(&path.to_qualified_path()))
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
            matrix.insert(&result);
        }
        self.logger.debug("Checking conflicting without base");
        // TODO
        self.checker.clean_up();
        Ok(matrix)
    }
}
