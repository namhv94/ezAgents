use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SandboxError {
    #[error("Path traversal forbidden: '{0}' escapes the workspace boundary")]
    PathTraversal(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("File exceeds maximum allowed size: {0} bytes")]
    FileTooLarge(usize),
}

#[derive(Clone, Debug)]
pub struct SandboxPolicy {
    pub max_file_size_bytes: usize,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            max_file_size_bytes: 2_000_000,
        }
    }
}

pub struct ScopedSandbox {
    workspace_root: PathBuf,
    policy: SandboxPolicy,
}

impl ScopedSandbox {
    pub fn new<P: AsRef<Path>>(
        workspace_path: P,
        policy: Option<SandboxPolicy>,
    ) -> Result<Self, SandboxError> {
        let ws = workspace_path.as_ref();
        if !ws.exists() {
            fs::create_dir_all(ws)?;
        }
        let canonical_root = ws.canonicalize()?;
        Ok(Self {
            workspace_root: canonical_root,
            policy: policy.unwrap_or_default(),
        })
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    fn resolve_safe_path<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf, SandboxError> {
        let p = path.as_ref();
        let target = if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.workspace_root.join(p)
        };

        // Normalize lexically first
        let mut components = Vec::new();
        for comp in target.components() {
            match comp {
                std::path::Component::ParentDir => {
                    components.pop();
                }
                std::path::Component::CurDir => {}
                c => components.push(c),
            }
        }
        let normalized: PathBuf = components.into_iter().collect();

        // Enforce boundary containment
        if !normalized.starts_with(&self.workspace_root) {
            return Err(SandboxError::PathTraversal(p.display().to_string()));
        }
        Ok(normalized)
    }

    pub fn read_file<P: AsRef<Path>>(&self, rel_path: P) -> Result<String, SandboxError> {
        let safe = self.resolve_safe_path(rel_path)?;
        let content = fs::read_to_string(safe)?;
        Ok(content)
    }

    pub fn write_file<P: AsRef<Path>>(
        &self,
        rel_path: P,
        content: &str,
    ) -> Result<usize, SandboxError> {
        let bytes = content.as_bytes();
        if bytes.len() > self.policy.max_file_size_bytes {
            return Err(SandboxError::FileTooLarge(bytes.len()));
        }
        let safe = self.resolve_safe_path(rel_path)?;
        if let Some(parent) = safe.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(safe, bytes)?;
        Ok(bytes.len())
    }

    pub fn list_files(&self) -> Result<Vec<String>, SandboxError> {
        let mut results = Vec::new();
        self.collect_files(&self.workspace_root, &mut results)?;
        results.sort();
        Ok(results)
    }

    fn collect_files(&self, dir: &Path, results: &mut Vec<String>) -> Result<(), SandboxError> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    self.collect_files(&path, results)?;
                } else if path.is_file() {
                    if let Ok(rel) = path.strip_prefix(&self.workspace_root) {
                        results.push(rel.display().to_string());
                    }
                }
            }
        }
        Ok(())
    }
}
