use crate::git::conflict::*;
use crate::git::error::*;
use crate::model::*;
use std::io;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Output};

fn output_to_result(output: Output, command: &Vec<&str>) -> Result<String, GitCommandError> {
    let stdout = String::from_utf8(output.stdout).unwrap().trim().to_string();
    let stderr = String::from_utf8(output.stderr).unwrap().trim().to_string();
    let message = format!("{}\n{}", stdout, stderr).trim().to_string();
    if output.status.success() {
        Ok(message)
    } else {
        let code = output.status.code().unwrap();
        let git_command = command.join(" ");
        let error = format!(
            "fatal: Command 'git {}' returned with exit code {}:\n",
            git_command, code
        );
        Err(GitCommandError::new(message, error))
    }
}

fn status_to_result(status: ExitStatus, command: &Vec<&str>) -> Result<(), GitCommandError> {
    if status.success() {
        Ok(())
    } else {
        let code = status.code().unwrap();
        let git_command = command.join(" ");
        let error = format!(
            "fatal: Command 'git {}' returned with exit code {}:\n",
            git_command, code
        );
        Err(GitCommandError::new("", error))
    }
}

fn make_commit_message_with_metadata<S: Into<String>>(
    message: S,
    metadata: Option<&CommitMetadataContainer>,
) -> String {
    let message = message.into();
    if let Some(metadata) = metadata {
        let meta = metadata.get_metadata();
        format!("{message}\n\n{meta}")
    } else {
        message
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
    pub fn prepare_command(&self, args: &Vec<&str>) -> Vec<String> {
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
        arguments.extend(args.into_iter().map(|arg| arg.to_string()));
        arguments
    }
    pub fn run_attached(&self, args: &Vec<&str>) -> io::Result<Output> {
        let mut base = Command::new("git");
        let arguments = self.prepare_command(args);
        base.args(arguments).output()
    }
    pub fn run_detached(&self, args: &Vec<&str>) -> io::Result<ExitStatus> {
        let mut base = Command::new("git");
        let arguments = self.prepare_command(args);
        base.args(arguments).status()
    }
}

#[derive(Debug)]
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
        let branch_command = vec!["branch", "--format=%(refname:short) %(objectname)"];
        let branch_output = self.raw_git_interface.run_attached(&branch_command)?;
        let all_branches: Vec<(String, String)> = String::from_utf8(branch_output.stdout)
            .unwrap()
            .trim()
            .split("\n")
            .map(|raw_string| {
                let split = raw_string.split(" ").collect::<Vec<&str>>();
                (split[0].to_string(), split[1].to_string())
            })
            .collect();
        for (branch, commit) in all_branches {
            if !branch.is_empty() {
                self.model.insert_git_branch(branch, commit);
            }
        }
        let tag_command = vec!["tag"];
        let tag_output = self.raw_git_interface.run_attached(&tag_command)?;
        let all_tags: Vec<String> = String::from_utf8(tag_output.stdout)
            .unwrap()
            .trim()
            .split("\n")
            .map(|raw_string| raw_string.replace("*", ""))
            .collect();
        for tag in all_tags {
            if !tag.is_empty() {
                self.model.insert_tag(tag);
            }
        }
        Ok(())
    }

    fn get_model(&self) -> &TreeDataModel {
        &self.model
    }

    fn update_head_commit<T: IsGitObject>(&self, path: &NodePath<T>) -> Result<(), GitError> {
        let commit = self.get_commit(&path)?;
        let node = path.get_node();
        node.borrow_mut()
            .update_head_commit(commit.get_hash().clone());
        Ok(())
    }

    fn get_current_branch(&self) -> Result<String, GitError> {
        let command = vec!["branch", "--show-current"];
        let out = self.raw_git_interface.run_attached(&command)?;
        Ok(output_to_result(out, &command)?)
    }

    pub fn get_current_normalized_path(&self) -> Result<NormalizedPath, GitError> {
        let mut base = NormalizedPath::from("");
        base.push(self.get_current_branch()?);
        Ok(base)
    }

    pub fn get_virtual_root(&self) -> NodePath<VirtualRoot> {
        self.get_model().get_virtual_root()
    }

    pub fn assert_path<T: SymbolicNodeType>(
        &self,
        path: &NormalizedPath,
    ) -> Result<NodePath<T>, PathAssertionError> {
        let node_path = self.get_model().assert_path::<T>(path)?;
        Ok(node_path)
    }

    pub fn assert_paths<T: SymbolicNodeType>(
        &self,
        paths: &Vec<NormalizedPath>,
    ) -> Result<Vec<NodePath<T>>, PathAssertionError> {
        let mut vec = vec![];
        for path in paths {
            vec.push(self.assert_path(path)?);
        }
        Ok(vec)
    }

    pub fn assert_current_node_path<T: IsGitObject>(
        &self,
    ) -> Result<NodePath<T>, PathAssertionError> {
        let current_qualified_path = self.get_current_normalized_path()?;
        match self.model.assert_path::<T>(&current_qualified_path) {
            Ok(path) => Ok(path),
            Err(error) => match error {
                ModelError::WrongNodeType(_) => {
                    let message =
                        format!("fatal: current branch is not of type '{}'", T::identifier());
                    Err(WrongNodeTypeError::new(message).into())
                }
                _ => Err(error.into()),
            },
        }
    }

    pub fn get_current_area(&self) -> Result<NodePath<ConcreteArea>, GitError> {
        let current_qualified_path = self.get_current_normalized_path()?;
        let qualified_path = NormalizedPath::from(&current_qualified_path[1]);
        Ok(self.model.get_area(&qualified_path).unwrap())
    }

    // all git commands
    pub fn initialize_repo(&self) -> Result<String, GitError> {
        let command = vec!["init", "--initial-branch=main"];
        let out = self.raw_git_interface.run_attached(&command)?;
        Ok(output_to_result(out, &command)?)
    }

    pub fn status(&self) -> Result<String, GitError> {
        let command = vec!["status"];
        let out = self.raw_git_interface.run_attached(&command)?;
        Ok(output_to_result(out, &command)?)
    }

    pub(super) fn checkout_raw(&self, path: &NormalizedPath) -> Result<String, GitError> {
        let branch = path.to_git_branch();
        let command = vec!["checkout", branch.as_str()];
        let out = self.raw_git_interface.run_attached(&command)?;
        Ok(output_to_result(out, &command)?)
    }

    pub fn checkout<T: IsGitObject>(&self, path: &NodePath<T>) -> Result<String, GitError> {
        self.checkout_raw(&path.to_normalized_path())
    }

    fn create_branch_no_mut(&self, path: &NormalizedPath) -> Result<String, GitError> {
        let branch = path.to_git_branch();
        let command = vec!["branch", branch.as_str()];
        Ok(output_to_result(
            self.raw_git_interface.run_attached(&command)?,
            &command,
        )?)
    }

    pub fn create_branch<T: SymbolicNodeType>(
        &self,
        path: &NormalizedPath,
    ) -> Result<NodePath<T>, PathAssertionError> {
        let current = self.assert_current_node_path::<AnyGitObject>()?;
        let commit = self.get_commit(&current)?;
        let node_type = self
            .get_model()
            .insert_git_branch(path.to_git_branch(), commit.get_hash().get_full_hash());
        if !T::is_compatible(&node_type) {
            let message = format!(
                "fatal: Expected to create branch of type '{}', but it would be of type '{}'",
                T::identifier(),
                node_type.get_type_name(),
            );
            return Err(WrongNodeTypeError::new(message).into());
        }
        self.create_branch_no_mut(path)?;
        Ok(self.model.get_node_path(&path).unwrap())
    }

    pub(super) fn delete_branch_no_mut(&self, path: &NormalizedPath) -> Result<String, GitError> {
        let branch = path.to_git_branch();
        let command = vec!["branch", "-D", branch.as_str()];
        let out = self.raw_git_interface.run_attached(&command)?;
        Ok(output_to_result(out, &command)?)
    }

    pub fn delete_branch<T: IsGitObject>(&mut self, path: NodePath<T>) -> Result<String, GitError> {
        self.delete_branch_no_mut(&path.to_normalized_path())
    }

    pub fn merge<B: IsGitObject, T: IsGitObject>(
        &self,
        path: NodePath<T>,
    ) -> Result<(MergeChainStatistic<B, T>, String), PathAssertionError> {
        let current = self.assert_current_node_path::<B>()?;
        let object = path.get_qualified_object();
        let command = vec!["merge", object.as_str()];
        let out = self.raw_git_interface.run_attached(&command)?;
        let result = output_to_result(out, &command);
        let (status, response) = match result {
            Ok(output) => {
                let status = if output.contains("Already up to date.") {
                    MergeStatistic::new(path, MergeResult::UpToDate)
                } else {
                    self.update_head_commit(&current)?;
                    MergeStatistic::new(path, MergeResult::Success)
                };
                (status, output)
            }
            Err(error) => {
                let output = error.get_git_output();
                let status = if output.contains("CONFLICT") {
                    MergeStatistic::new(path, MergeResult::Conflict)
                } else {
                    MergeStatistic::new(path, MergeResult::Error(output.to_string()))
                };
                (status, output.to_string())
            }
        };
        let mut chain = MergeChainStatistic::new(current);
        chain.push(status);
        Ok((chain, response))
    }

    pub fn cherry_pick<B: IsGitObject, T: IsGitObject>(
        &self,
        path: NodePath<T>
    ) -> Result<(MergeChainStatistic<B, T>, String), PathAssertionError> {
        let current = self.assert_current_node_path::<B>()?;
        let object = path.get_qualified_object();
        let command = vec!["cherry-pick", object.as_str()];
        let out = self.raw_git_interface.run_attached(&command)?;
        let result = output_to_result(out, &command);
        let (status, response) = match result {
            Ok(output) => {
                let status = MergeStatistic::new(path, MergeResult::Success);
                self.update_head_commit(&current)?;
                (status, output)
            }
            Err(error) => {
                let output = error.get_git_output();
                let status = if output.contains("CONFLICT") {
                    MergeStatistic::new(path, MergeResult::Conflict)
                } else if output.contains("If you wish to commit it anyway") {
                    MergeStatistic::new(path, MergeResult::UpToDate)
                } else {
                    MergeStatistic::new(path, MergeResult::Error(output.to_string()))
                };
                (status, output.to_string())
            }
        };
        let mut chain = MergeChainStatistic::new(current);
        chain.push(status);
        Ok((chain, response))
    }

    pub fn abort_merge(&self) -> Result<String, GitError> {
        let command = vec!["merge", "--abort"];
        let out = self.raw_git_interface.run_attached(&command)?;
        Ok(output_to_result(out, &command)?)
    }

    pub fn abort_cherry_pick(&self) -> Result<String, GitError> {
        let command = vec!["cherry-pick", "--abort"];
        let out = self.raw_git_interface.run_attached(&command)?;
        Ok(output_to_result(out, &command)?)
    }

    pub fn pending_merge(&self) -> Result<bool, GitError> {
        let status = self.status()?;
        Ok(status.contains("merg"))
    }

    // pub fn create_tag(&mut self, tag: &QualifiedPath) -> Result<NodePath<Tag>, GitError> {
    //     let current_branch = self.get_current_qualified_path()?;
    //     let tagged = current_branch + tag.clone();
    //     let out = self
    //         .raw_git_interface
    //         .run(vec!["tag", tagged.to_git_branch().as_str()])?;
    // }
    //
    // pub fn delete_tag(&self, tag: NodePath<Tag>) -> Result<Output, GitError> {
    //     let current_branch = self.get_current_qualified_path()?;
    //     let tagged = current_branch + tag.clone();
    //     Ok(self
    //         .raw_git_interface
    //         .run(vec!["tag", "-d", tagged.to_git_branch().as_str()])?)
    // }

    pub fn get_commit_from_hash(&self, hash: &CommitHash) -> Result<Commit, GitError> {
        let command = vec!["log", "--format=%B", "-n 1", hash.get_full_hash()];
        let out = self.raw_git_interface.run_attached(&command)?;
        let message = output_to_result(out, &command)?;
        Ok(Commit::new(hash.clone(), message))
    }

    pub fn iter_commit_history<T: IsGitObject>(
        &self,
        path: &NodePath<T>,
        n: i32,
    ) -> Result<CommitIterator, GitError> {
        let n_str = n.to_string();
        let object = path.to_normalized_path().to_git_branch();
        let command = vec!["log", "-n", n_str.as_str(), "--format=%H", object.as_str()];
        let out = self.raw_git_interface.run_attached(&command)?;
        let raw_hashes = output_to_result(out, &command)?.trim().to_string();
        let all_hashes = raw_hashes
            .split("\n")
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        Ok(CommitIterator::new(all_hashes, &self))
    }

    pub fn get_commit<T: IsGitObject>(&self, branch: &NodePath<T>) -> Result<Commit, GitError> {
        let iterator = self.iter_commit_history(&branch, 1)?;
        let mut commits: Vec<Commit> = vec![];
        for commit in iterator {
            commits.push(commit?);
        }
        Ok(commits[0].clone())
    }

    pub fn get_files_managed_by_branch<T: IsGitObject>(
        &self,
        branch: &NodePath<T>,
    ) -> Result<Vec<String>, GitError> {
        let branch = branch.to_normalized_path().to_git_branch();
        let command = vec!["ls-tree", "-r", "--name-only", branch.as_str()];
        let out = self.raw_git_interface.run_attached(&command)?;
        let message = output_to_result(out, &command)?;
        Ok(message.split("\n").map(|e| e.to_string()).collect())
    }

    pub fn get_files_changed_by_commit<S: Into<String>>(
        &self,
        commit: S,
    ) -> Result<Vec<String>, GitError> {
        let commit_str = commit.into();
        let command = vec![
            "diff-tree",
            "--no-commit-id",
            "--name-only",
            commit_str.as_str(),
            "-r",
        ];
        let out = self.raw_git_interface.run_attached(&command)?;
        let message = output_to_result(out, &command)?;
        Ok(message.split("\n").map(|e| e.to_string()).collect())
    }

    pub fn commit<S: Into<String>, T: IsGitObject>(
        &self,
        message: S,
        metadata: Option<&CommitMetadataContainer>,
        allow_empty: bool,
        attached: bool,
    ) -> Result<String, PathAssertionError> {
        let current = self.assert_current_node_path::<T>()?;
        let commit_message = make_commit_message_with_metadata(message, metadata);
        let command = if allow_empty {
            vec!["commit", "--allow-empty", "-m", commit_message.as_str()]
        } else {
            vec!["commit", "-m", commit_message.as_str()]
        };
        let response = if attached {
            let out = self.raw_git_interface.run_attached(&command)?;
            output_to_result(out, &command)?
        } else {
            let out = self.raw_git_interface.run_detached(&command)?;
            status_to_result(out, &command)?;
            "".to_string()
        };
        self.update_head_commit(&current)?;
        Ok(response)
    }

    pub fn reset_hard(&self, commit: &CommitHash) -> Result<String, GitError> {
        let command = vec!["reset", "--hard", commit.get_full_hash()];
        let out = self.raw_git_interface.run_attached(&command)?;
        Ok(output_to_result(out, &command)?)
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
        git.run_attached(&vec!["init", "--initial-branch=main"])?;
        let mut file = path.clone();
        file.push("file1");
        fs::write(file.clone(), "")?;
        let out = git.run_attached(&vec!["add", file.to_str().unwrap()])?;
        let out = git.run_attached(&vec!["commit", "-m", "initial commit"])?;
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
            git.run_attached(&vec!["branch", branch])?;
        }
        Ok(())
    }

    pub fn populate_with_products(path: PathBuf) -> Result<(), GitError> {
        let git = GitCLI::in_custom_directory(PathBuf::from(path));
        let branches = vec!["_main/_product/myprod"];
        for branch in branches {
            git.run_attached(&vec!["branch", branch])?;
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
        let current = interface.get_current_normalized_path().unwrap();
        assert_eq!(current, "/main")
    }
}
