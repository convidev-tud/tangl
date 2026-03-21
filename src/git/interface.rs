use crate::git::conflict::{MergeConflict, MergeStatistic, MergeSuccess};
use crate::git::error::{GitCommandError, GitError, GitWrongNodeTypeError};
use crate::model::*;
use crate::util::u8_to_string;
use serde::de::Unexpected::Str;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Output};

fn output_to_result(output: Output) -> Result<String, GitCommandError> {
    if output.status.success() {
        Ok(String::from_utf8(output.stdout).unwrap().trim().to_string())
    } else {
        Err(GitCommandError::new(
            String::from_utf8(output.stderr).unwrap().trim().to_string(),
        ))
    }
}

#[derive(Clone, Debug)]
pub enum GitPath {
    CurrentDirectory,
    CustomDirectory(PathBuf),
}

#[derive(Clone, Debug)]
pub(super) struct GitCLI {
    path: GitPath,
    colored: bool,
}
impl GitCLI {
    pub fn in_current_directory() -> Self {
        Self::new(GitPath::CurrentDirectory)
    }
    pub fn in_custom_directory(path: PathBuf) -> Self {
        Self::new(GitPath::CustomDirectory(path))
    }
    pub fn new(path: GitPath) -> Self {
        Self {
            path,
            colored: false,
        }
    }
    pub fn colored(&mut self, colored: bool) {
        self.colored = colored;
    }
    pub fn run(&self, args: Vec<&str>) -> io::Result<Output> {
        let mut base = Command::new("git");
        let mut arguments: Vec<String> = vec![];
        match self.path {
            GitPath::CurrentDirectory => {}
            GitPath::CustomDirectory(ref path) => {
                arguments.push(format!("--git-dir={}/.git", path.to_str().unwrap()));
                arguments.push(format!("--work-tree={}", path.to_str().unwrap()));
            }
        }
        if self.colored {
            arguments.push("-c".to_string());
            arguments.push("color.ui=always".to_string());
        }
        let mut transformed: Vec<&str> = arguments.iter().map(|s| s.as_str()).collect();
        transformed.extend(args);
        base.args(transformed).output()
    }
}

#[derive(Clone, Debug)]
pub struct GitInterface {
    model: TreeDataModel,
    raw_git_interface: GitCLI,
}
impl GitInterface {
    pub fn default() -> Self {
        Self::new(GitPath::CurrentDirectory)
    }

    pub fn in_directory(path: PathBuf) -> Self {
        Self::new(GitPath::CustomDirectory(path))
    }

    pub fn new(path: GitPath) -> Self {
        let raw_interface = GitCLI::new(path);
        let mut interface = Self {
            model: TreeDataModel::new(),
            raw_git_interface: raw_interface,
        };
        match interface.update_complete_model() {
            Ok(_) => interface,
            Err(e) => panic!("{:?}", e),
        }
    }

    pub fn colored_output(&mut self, color: bool) {
        self.raw_git_interface.colored(color);
    }

    fn update_complete_model(&mut self) -> Result<(), io::Error> {
        let branch_output = self.raw_git_interface.run(vec!["branch"])?;
        let all_branches: Vec<String> = u8_to_string(&branch_output.stdout)
            .split("\n")
            .map(|raw_string| raw_string.replace("*", ""))
            .collect();
        for branch in all_branches {
            if !branch.is_empty() {
                let mut path = QualifiedPath::from("");
                path.push(branch);
                self.model.insert_qualified_path(path, false);
            }
        }
        let tag_output = self.raw_git_interface.run(vec!["tag"])?;
        let all_tags: Vec<String> = u8_to_string(&tag_output.stdout)
            .split("\n")
            .map(|raw_string| raw_string.replace("*", ""))
            .collect();
        for tag in all_tags {
            if !tag.is_empty() {
                let mut path = QualifiedPath::from("");
                path.push(tag);
                self.model.insert_qualified_path(path, true);
            }
        }
        Ok(())
    }

    pub fn get_model(&self) -> &TreeDataModel {
        &self.model
    }

    fn get_current_branch(&self) -> Result<String, GitError> {
        let out = self
            .raw_git_interface
            .run(vec!["branch", "--show-current"])?;
        Ok(output_to_result(out)?)
    }

    pub fn get_current_qualified_path(&self) -> Result<QualifiedPath, GitError> {
        let mut base = QualifiedPath::from("");
        base.push(self.get_current_branch()?);
        Ok(base)
    }

    pub fn assert_current_node_path<T: HasBranch>(
        &self,
    ) -> Result<NodePath<T>, GitWrongNodeTypeError> {
        let current_qualified_path = self.get_current_qualified_path()?;
        match self.model.assert_path::<T>(&current_qualified_path) {
            Ok(path) => Ok(path),
            Err(error) => match error {
                ModelError::WrongNodeType(_) => {
                    let message =
                        format!("fatal: current branch is not of type '{}'", T::identifier());
                    Err(WrongNodeTypeError::new(message).into())
                }
                _ => unreachable!(),
            },
        }
    }

    pub fn get_current_area(&self) -> Result<NodePath<ConcreteArea>, GitError> {
        let current_qualified_path = self.get_current_qualified_path()?;
        let qualified_path = QualifiedPath::from(&current_qualified_path[1]);
        Ok(self.model.get_area(&qualified_path).unwrap())
    }

    // all git commands
    pub fn initialize_repo(&self) -> Result<String, GitError> {
        let out = self
            .raw_git_interface
            .run(vec!["init", "--initial-branch=main"])?;
        Ok(output_to_result(out)?)
    }

    pub fn status(&self) -> Result<String, GitError> {
        Ok(output_to_result(
            self.raw_git_interface.run(vec!["status"])?,
        )?)
    }

    pub(super) fn checkout_raw(&self, path: &QualifiedPath) -> Result<String, GitError> {
        let out = self
            .raw_git_interface
            .run(vec!["checkout", path.to_git_branch().as_str()])?;
        Ok(output_to_result(out)?)
    }

    pub fn checkout<T: HasBranch>(&self, path: &NodePath<T>) -> Result<String, GitError> {
        self.checkout_raw(&path.to_qualified_path())
    }

    pub(super) fn create_branch_no_mut(&self, path: &QualifiedPath) -> Result<String, GitError> {
        let branch = path.to_git_branch();
        let commands = vec!["branch", branch.as_str()];
        Ok(output_to_result(self.raw_git_interface.run(commands)?)?)
    }

    pub fn create_branch<T: SymbolicNodeType>(
        &mut self,
        path: &QualifiedPath,
    ) -> Result<NodePath<T>, GitWrongNodeTypeError> {
        let node_type = self.model.insert_qualified_path(path.clone(), false);
        if !T::is_compatible(&node_type) {
            let message = format!(
                "Expected to create branch of type '{}', but it would be of type '{}'",
                T::identifier(),
                node_type.get_type_name(),
            );
            return Err(WrongNodeTypeError::new(message).into());
        }
        self.create_branch_no_mut(path)?;
        Ok(self.model.get_node_path(&path).unwrap())
    }

    pub(super) fn delete_branch_no_mut(&self, path: &QualifiedPath) -> Result<String, GitError> {
        let branch = path.to_git_branch();
        let commands = vec!["branch", "-D", branch.as_str()];
        let out = self.raw_git_interface.run(commands)?;
        Ok(output_to_result(out)?)
    }

    pub fn delete_branch<T: HasBranch>(&mut self, path: NodePath<T>) -> Result<String, GitError> {
        self.delete_branch_no_mut(&path.to_qualified_path())
    }

    pub fn merge<T: HasBranch>(
        &self,
        path: &NodePath<T>,
    ) -> Result<(MergeStatistic, String), io::Error> {
        let out = self.raw_git_interface.run(vec![
            "merge",
            path.to_qualified_path().to_git_branch().as_str(),
        ])?;
        let result = if out.status.success() {
            let response = String::from_utf8(out.stdout).unwrap();
            let success = MergeSuccess::new(path.to_qualified_path());
            (MergeStatistic::Success(success), response)
        } else {
            let response = String::from_utf8(out.stderr).unwrap();
            let conflict = MergeConflict::new(path.to_qualified_path());
            (MergeStatistic::Conflict(conflict), response)
        };
        Ok(result)
    }

    pub fn abort_merge(&self) -> Result<String, GitError> {
        Ok(output_to_result(
            self.raw_git_interface.run(vec!["merge", "--abort"])?,
        )?)
    }

    pub fn create_tag(&self, tag: &QualifiedPath) -> Result<NodePath<Tag>, GitError> {
        let current_branch = self.get_current_qualified_path()?;
        let tagged = current_branch + tag.clone();
        let out = self
            .raw_git_interface
            .run(vec!["tag", tagged.to_git_branch().as_str()])?;
    }

    pub fn delete_tag(&self, tag: NodePath<Tag>) -> Result<Output, GitError> {
        let current_branch = self.get_current_qualified_path()?;
        let tagged = current_branch + tag.clone();
        Ok(self
            .raw_git_interface
            .run(vec!["tag", "-d", tagged.to_git_branch().as_str()])?)
    }

    pub fn get_commit_from_hash<S: Into<String>>(&self, hash: S) -> Result<Commit, GitError> {
        let h = hash.into();
        let out = self
            .raw_git_interface
            .run(vec!["log", "--format=%B", "-n 1", h.as_str()])?;
        let message = output_to_result(out)?;
        Ok(Commit::new(h, message))
    }

    pub fn iter_commit_history<T: HasBranch>(
        &self,
        branch: &NodePath<T>,
        n: i32,
    ) -> Result<CommitIterator, GitError> {
        let out = self.raw_git_interface.run(vec![
            "log",
            "n",
            n.to_string().as_str(),
            "--format=%H",
            branch.to_qualified_path().to_git_branch().as_str(),
        ])?;
        let raw_hashes = output_to_result(out)?.trim().to_string();
        let all_hashes = raw_hashes
            .split("\n")
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        Ok(CommitIterator::new(all_hashes, &self))
    }

    pub fn get_last_commit<T: HasBranch>(&self, branch: &NodePath<T>) -> Result<Commit, GitError> {
        let iterator = self.iter_commit_history(&branch, 1)?;
        let mut commits: Vec<Commit> = vec![];
        for commit in iterator {
            commits.push(commit?);
        }
        Ok(commits[0].clone())
    }

    pub fn get_files_managed_by_branch<T: HasBranch>(
        &self,
        branch: &NodePath<T>,
    ) -> Result<Vec<String>, GitError> {
        let out = self.raw_git_interface.run(vec![
            "ls-tree",
            "-r",
            "--name-only",
            branch.to_qualified_path().to_git_branch().as_str(),
        ])?;
        let message = output_to_result(out)?;
        Ok(message.split("\n").map(|e| e.to_string()).collect())
    }

    pub fn get_files_changed_by_commit<S: Into<String>>(
        &self,
        commit: S,
    ) -> Result<Vec<String>, GitError> {
        let out = self.raw_git_interface.run(vec![
            "diff-tree",
            "--no-commit-id",
            "--name-only",
            commit.into().as_str(),
            "-r",
        ])?;
        let message = output_to_result(out)?;
        Ok(message.split("\n").map(|e| e.to_string()).collect())
    }

    pub fn commit<S: Into<String>>(&self, message: S) -> Result<String, GitError> {
        let message_string = message.into();
        let out = self
            .raw_git_interface
            .run(vec!["commit", "-m", message_string.as_str()])?;
        Ok(output_to_result(out)?)
    }

    pub fn empty_commit<S: Into<String>>(&self, message: S) -> Result<String, GitError> {
        let message_string = message.into();
        let out = self.raw_git_interface.run(vec![
            "commit",
            "--allow-empty",
            "-m",
            message_string.as_str(),
        ])?;
        Ok(output_to_result(out)?)
    }

    pub fn interactive_commit(&self) -> Result<String, GitError> {
        Ok(output_to_result(
            self.raw_git_interface.run(vec!["commit"])?,
        )?)
    }

    pub fn cherry_pick(&self, commit: &str) -> Result<String, GitError> {
        Ok(output_to_result(
            self.raw_git_interface.run(vec!["cherry-pick", commit])?,
        )?)
    }

    pub fn reset_hard(&self, commit: &str) -> Result<String, GitError> {
        let out = self
            .raw_git_interface
            .run(vec!["reset", "--hard", commit])?;
        Ok(output_to_result(out)?)
    }
}

#[cfg(test)]
pub mod test_utils {
    use crate::git::error::GitError;
    use crate::git::interface::GitCLI;
    use std::fs;
    use std::path::PathBuf;

    pub fn prepare_empty_git_repo(path: PathBuf) -> Result<(), GitError> {
        let git = GitCLI::in_custom_directory(path.clone());
        git.run(vec!["init", "--initial-branch=main"])?;
        let mut file = path.clone();
        file.push("file1");
        fs::write(file.clone(), "")?;
        let out = git.run(vec!["add", file.to_str().unwrap()])?;
        let out = git.run(vec!["commit", "-m", "initial commit"])?;
        Ok(())
    }

    pub fn populate_with_features(path: PathBuf) -> Result<(), GitError> {
        let git = GitCLI::in_custom_directory(PathBuf::from(path));
        let branches = vec![
            "_main/_feature/root",
            "_main/_feature/_root/foo",
            "_main/_feature/_root/bar",
            "_main/_feature/_root/baz",
        ];
        for branch in branches {
            git.run(vec!["branch", branch])?;
        }
        Ok(())
    }

    pub fn populate_with_products(path: PathBuf) -> Result<(), GitError> {
        let git = GitCLI::in_custom_directory(PathBuf::from(path));
        let branches = vec!["_main/_product/myprod"];
        for branch in branches {
            git.run(vec!["branch", branch])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::interface::test_utils::{populate_with_features, prepare_empty_git_repo};
    use tempfile::TempDir;

    #[test]
    fn interface_populate_model() {
        let path = TempDir::new().unwrap();
        let path_buf = PathBuf::from(path.path());
        prepare_empty_git_repo(path_buf.clone()).unwrap();
        populate_with_features(path_buf.clone()).unwrap();
        let interface = GitInterface::new(GitPath::CustomDirectory(path_buf));
        let paths = interface.get_model().get_qualified_paths_with_branches();
        assert_eq!(
            paths,
            &vec![
                "/main/feature/root/bar",
                "/main/feature/root/baz",
                "/main/feature/root/foo",
                "/main/feature/root",
                "/main",
            ]
        );
    }

    #[test]
    fn interface_get_current_branch_absolute() {
        let path = TempDir::new().unwrap();
        let path_buf = PathBuf::from(path.path());
        prepare_empty_git_repo(path_buf.clone()).unwrap();
        populate_with_features(path_buf.clone()).unwrap();
        let interface = GitInterface::new(GitPath::CustomDirectory(path_buf));
        let current = interface.get_current_qualified_path().unwrap();
        assert_eq!(current, "/main")
    }
}
