//! The project and branch a session works in, read through gix.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Workspace {
    /// The main worktree's directory name, else the directory's own.
    pub project: String,
    /// The checked-out branch; empty on a detached head or outside a repository.
    pub branch: String,
}

impl Workspace {
    pub fn locate(dir: &str) -> Self {
        let Ok(repo) = gix::discover_with_environment_overrides(dir) else {
            return Self {
                project: basename(dir).to_owned(),
                branch: String::new(),
            };
        };
        let common = std::fs::canonicalize(repo.common_dir())
            .unwrap_or_else(|_| repo.common_dir().to_owned());
        let project = common.parent().map_or_else(
            || "/".to_owned(),
            |parent| basename(&parent.to_string_lossy()).to_owned(),
        );
        let branch = repo
            .head_name()
            .ok()
            .flatten()
            .and_then(|name| {
                name.as_bstr()
                    .strip_prefix(b"refs/heads/")
                    .map(|branch| String::from_utf8_lossy(branch).into_owned())
            })
            .unwrap_or_default();
        Self { project, branch }
    }
}

/// The last path component, as `basename` gives it.
fn basename(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return if path.is_empty() { "" } else { "/" };
    }
    trimmed.rsplit('/').next().unwrap_or(trimmed)
}

#[cfg(test)]
mod tests {
    use super::basename;

    #[test]
    fn basename_follows_the_shell() {
        assert_eq!(basename("/a/b/"), "b");
        assert_eq!(basename("/"), "/");
        assert_eq!(basename("."), ".");
        assert_eq!(basename(""), "");
    }
}
