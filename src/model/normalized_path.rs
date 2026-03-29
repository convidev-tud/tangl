use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::ops::{Add, Index};

const PATH_SEPARATOR: char = '/';
const VERSION_SEPARATOR: char = ':';

#[derive(Clone, Debug, Hash, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct NormalizedPath {
    path: Vec<String>,
}

impl From<String> for NormalizedPath {
    fn from(value: String) -> Self {
        let mut qualified_path = Self::new();
        qualified_path.push(value);
        qualified_path
    }
}

impl From<&String> for NormalizedPath {
    fn from(value: &String) -> Self {
        Self::from(value.to_string())
    }
}

impl From<&str> for NormalizedPath {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

impl From<Vec<String>> for NormalizedPath {
    fn from(value: Vec<String>) -> Self {
        let mut path = Self::new();
        for v in value {
            path.push(v);
        }
        path
    }
}

impl From<NormalizedPath> for String {
    fn from(value: NormalizedPath) -> Self {
        value.to_string()
    }
}

impl PartialEq for NormalizedPath {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl PartialEq<String> for NormalizedPath {
    fn eq(&self, other: &String) -> bool {
        self.to_string() == *other
    }

    fn ne(&self, other: &String) -> bool {
        self.to_string() != *other
    }
}

impl PartialEq<&str> for NormalizedPath {
    fn eq(&self, other: &&str) -> bool {
        self.to_string() == *other
    }

    fn ne(&self, other: &&str) -> bool {
        self.to_string() != *other
    }
}

impl Add for NormalizedPath {
    type Output = NormalizedPath;

    fn add(self, rhs: Self) -> Self::Output {
        let mut next_index = self.len();
        let mut new_path = self;
        if new_path.last_is(&NormalizedPath::from("")) && new_path.len() > 1 {
            new_path = new_path.strip_n_right(new_path.len() - 1);
        }
        for (i, part) in rhs.iter_segments().enumerate() {
            match part.as_str() {
                "." => {}
                ".." => {
                    if next_index > 0 {
                        next_index -= 1;
                        new_path = new_path.strip_n_right(next_index);
                    }
                }
                "" => {
                    if i == 0 && rhs.len() > 1 {
                        return rhs.clone();
                    } else if i == rhs.len() - 1 || new_path.is_empty() {
                        new_path.push(part.to_string())
                    }
                }
                _ => {
                    new_path.push(part.to_string());
                    next_index += 1;
                }
            }
        }
        new_path
    }
}

impl Display for NormalizedPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.path.join("/").as_str())
    }
}

impl Index<usize> for NormalizedPath {
    type Output = String;

    fn index(&self, index: usize) -> &Self::Output {
        &self.path[index]
    }
}

impl NormalizedPath {
    pub fn new() -> Self {
        Self {
            path: Vec::new(),
        }
    }
    pub fn to_git_branch(&self) -> String {
        let trimmed_path = self.trim_whitespaces();
        let path = trimmed_path.path;
        match path.len() {
            1 => path[0].to_string(),
            _ => {
                let mut prefix = path[..path.len() - 1]
                    .iter()
                    .map(|x| "_".to_string() + x)
                    .collect::<Vec<_>>();
                prefix.push(path[path.len() - 1].to_string());
                prefix.join("/")
            }
        }
    }
    pub fn push<S: Into<String>>(&mut self, path: S) {
        let qualified_str = path.into().replace("_", "");
        for split in qualified_str.trim().split(PATH_SEPARATOR) {
            self.path.push(split.to_lowercase());
        }
    }
    pub fn get_version_appendix(&self) -> Option<String> {
        let last = self.last()?;
        if last.contains(VERSION_SEPARATOR) {
            Some(last.split(VERSION_SEPARATOR).collect::<Vec<_>>()[1].to_string())
        } else { None }
    }
    pub fn set_version_appendix<S: Into<String>>(&mut self, version_appendix: Option<S>) {
        if let Some(version) = version_appendix {
            let version = version.into();
            let last = self.path.pop().unwrap();
            if self.get_version_appendix().is_some() {
                let split = last.split(VERSION_SEPARATOR).collect::<Vec<&str>>();
                self.push(format!("{}:{version}", split[0]));
            } else {
                self.push(format!("{last}:{version}"));
            }
        } else {
            if self.get_version_appendix().is_some() {
                let last = self.path.pop().unwrap();
                let split = last.split(VERSION_SEPARATOR).collect::<Vec<&str>>();
                self.push(format!("{}", split[0]));
            }
        }
    }
    pub fn strip_n(&self, n_left: usize, n_right: usize) -> NormalizedPath {
        NormalizedPath::from(self.path[n_left..n_right].to_vec())
    }
    pub fn strip_n_left(&self, n: usize) -> NormalizedPath {
        self.strip_n(n, self.path.len())
    }
    pub fn strip_version(&self) -> NormalizedPath {
        let mut new = self.clone(); 
        new.set_version_appendix::<String>(None);
        new
    }
    pub fn strip_n_right(&self, n: usize) -> NormalizedPath {
        self.strip_n(0, n)
    }
    pub fn trim_whitespaces(&self) -> NormalizedPath {
        let mut new_path = self.path.clone();
        match new_path.first() {
            Some(value) => {
                if value == "" {
                    new_path.remove(0);
                }
            }
            None => {}
        }
        match new_path.last() {
            Some(value) => {
                if value == "" {
                    new_path.remove(new_path.len() - 1);
                }
            }
            None => {}
        }
        NormalizedPath::from(new_path)
    }
    pub fn replace<S: Into<String>>(&self, index: usize, value: S) -> NormalizedPath {
        let mut new_path = self.path.clone();
        new_path.insert(index, value.into());
        NormalizedPath::from(new_path)
    }
    pub fn first(&self) -> Option<NormalizedPath> {
        Some(NormalizedPath::from(self.path.first()?.clone()))
    }
    pub fn last(&self) -> Option<&String> {
        self.path.last()
    }
    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = NormalizedPath> {
        self.iter_segments()
            .map(|s| NormalizedPath::from(s.clone()))
    }
    pub fn iter_segments(&self) -> impl Iterator<Item = &String> {
        self.path.iter()
    }
    pub fn get(&self, index: usize) -> Option<NormalizedPath> {
        Some(NormalizedPath::from(self.path.get(index)?.clone()))
    }
    pub fn starts_with(&self, prefix: &NormalizedPath) -> bool {
        self.to_string().starts_with(&prefix.to_string())
    }
    pub fn last_is(&self, suffix: &NormalizedPath) -> bool {
        self.last() == suffix.last()
    }
    pub fn len(&self) -> usize {
        self.path.len()
    }
    pub fn as_dir(&self) -> NormalizedPath {
        let mut new_path = self.path.clone();
        new_path.push("".to_string());
        NormalizedPath::from(new_path)
    }
    pub fn as_absolute(&self) -> NormalizedPath {
        let mut new_path = self.path.clone();
        new_path.insert(0, "".to_string());
        NormalizedPath::from(new_path)
    }
    pub fn is_dir(&self) -> bool {
        self.path.len() > 1 && self.last().unwrap() == ""
    }
    pub fn is_absolute(&self) -> bool {
        self.path.len() > 0 && self.first().unwrap() == ""
    }
    pub fn formatted(&self, colored: bool) -> String {
        let base = if colored {
            self.to_string().blue().to_string()
        } else {
            self.to_string()
        };
        base
    }
}

pub trait ToNormalizedPath {
    fn to_normalized_path(&self) -> NormalizedPath;
}

pub trait ToNormalizedPaths {
    fn to_normalized_paths(&self) -> Vec<NormalizedPath>;
}

impl ToNormalizedPath for String {
    fn to_normalized_path(&self) -> NormalizedPath {
        NormalizedPath::from(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalized_path_from_qualified() {
        assert_eq!(NormalizedPath::from("foo/bar").path, vec!["foo", "bar"]);
        assert_eq!(
            NormalizedPath::from("/foo/bar").path,
            vec!["", "foo", "bar"]
        );
        assert_eq!(NormalizedPath::from("/foo/bar").to_string(), "/foo/bar");
        assert_eq!(NormalizedPath::from("foo/").path, vec!["foo", ""]);
        assert_eq!(NormalizedPath::from("/").path, vec!["", ""]);
    }

    #[test]
    fn test_normalized_path_from_git_branch() {
        assert_eq!(NormalizedPath::from("_foo/bar").path, vec!["foo", "bar"]);
        assert_eq!(
            NormalizedPath::from("_foo/bar".to_string()).path,
            vec!["foo", "bar"]
        );
        assert_eq!(
            NormalizedPath::from("_foo/_bar/baz").path,
            vec!["foo", "bar", "baz"]
        );
    }

    #[test]
    fn test_normalized_path_to_git_object() {
        assert_eq!(NormalizedPath::from("foo/bar").to_git_branch(), "_foo/bar");
        assert_eq!(NormalizedPath::from("/foo/bar").to_git_branch(), "_foo/bar");
        assert_eq!(
            NormalizedPath::from("/foo/bar/c:abc").to_git_branch(),
            "abc"
        );
        assert_eq!(
            NormalizedPath::from("/foo/bar/t:1.0.0").to_git_branch(),
            "_foo/_bar/1.0.0"
        );
    }

    #[test]
    fn test_normalized_path_add_empty() {
        let l = NormalizedPath::new();
        let r = NormalizedPath::from("foo/bar");
        assert_eq!(l + r, NormalizedPath::from("foo/bar"));

        let l = NormalizedPath::new();
        let r = NormalizedPath::from("/foo/bar");
        assert_eq!(l + r, NormalizedPath::from("/foo/bar"));
    }

    #[test]
    fn test_normalized_path_add_absolute() {
        let l = NormalizedPath::from("foo");
        let r = NormalizedPath::from("bar/baz");
        assert_eq!(l + r, NormalizedPath::from("foo/bar/baz"));

        let l = NormalizedPath::from("");
        let r = NormalizedPath::from("bar/baz");
        assert_eq!((l + r).path, vec!["", "bar", "baz"]);

        let l = NormalizedPath::from("foo/");
        let r = NormalizedPath::from("bar/baz");
        assert_eq!(l + r, NormalizedPath::from("foo/bar/baz"));
    }

    #[test]
    fn test_normalized_path_add_relative() {
        let l = NormalizedPath::from("foo");
        let r = NormalizedPath::from("..");
        assert_eq!(l + r, NormalizedPath::new());

        let l = NormalizedPath::from("foo");
        let r = NormalizedPath::from("./bar");
        assert_eq!(l + r, NormalizedPath::from("foo/bar"));

        let l = NormalizedPath::from("foo");
        let r = NormalizedPath::from("./");
        assert_eq!(l + r, NormalizedPath::from("foo/"));

        let l = NormalizedPath::from("foo");
        let r = NormalizedPath::from("../bar");
        assert_eq!(l + r, NormalizedPath::from("bar"));

        let l = NormalizedPath::from("foo/bar");
        let r = NormalizedPath::from("../baz");
        assert_eq!(l + r, NormalizedPath::from("foo/baz"));

        let l = NormalizedPath::from("foo/bar");
        let r = NormalizedPath::from("../../baz");
        assert_eq!(l + r, NormalizedPath::from("baz"));

        let l = NormalizedPath::from("foo/bar");
        let r = NormalizedPath::from("../../../../../../baz");
        assert_eq!(l + r, NormalizedPath::from("baz"));

        let l = NormalizedPath::from("foo/bar");
        let r = NormalizedPath::from("baz/../baz/../baz/../baz");
        assert_eq!(l + r, NormalizedPath::from("foo/bar/baz"));

        let l = NormalizedPath::from("foo/bar");
        let r = NormalizedPath::from("../baz/../baz/../baz");
        assert_eq!(l + r, NormalizedPath::from("foo/baz"));
    }

    #[test]
    fn test_normalized_path_add_whitespaces() {
        let l = NormalizedPath::from("foo");
        let r = NormalizedPath::from("");
        assert_eq!(l + r, NormalizedPath::from("foo/"));

        let l = NormalizedPath::from("foo");
        let r = NormalizedPath::from("/bar/baz");
        assert_eq!(l + r, NormalizedPath::from("/bar/baz"));
    }

    #[test]
    fn test_normalized_path_trim() {
        let path = NormalizedPath::from("foo/bar");
        assert_eq!(path.strip_n(0, path.len() - 1).path, vec!["foo"]);
    }

    #[test]
    fn test_normalized_path_as_absolute() {
        let path = NormalizedPath::from("foo/bar");
        let absolute = path.as_absolute();
        assert!(absolute.is_absolute());
        assert_eq!(absolute, "/foo/bar");
    }
}
