use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Serialize, Clone, Debug)]
pub struct VerificationResult {
    pub name: String,
    pub passed: bool,
    pub details: String,
    pub duration_ms: f64,
}

pub enum VerifierCheck {
    FileExists(String),
    ContentMatch(String, String),
    CommandRun(Vec<String>, i32),
}

pub struct DeterministicVerifier {
    workspace_path: PathBuf,
    checks: Vec<(String, VerifierCheck)>,
}

impl DeterministicVerifier {
    pub fn new<P: AsRef<Path>>(workspace_path: P) -> Self {
        Self {
            workspace_path: workspace_path.as_ref().to_path_buf(),
            checks: Vec::new(),
        }
    }

    pub fn add_file_exists_check(&mut self, rel_path: &str) {
        self.checks.push((
            format!("FileExists:{}", rel_path),
            VerifierCheck::FileExists(rel_path.to_string()),
        ));
    }

    pub fn add_content_match_check(&mut self, rel_path: &str, substring: &str) {
        self.checks.push((
            format!("ContentMatch:{}", rel_path),
            VerifierCheck::ContentMatch(rel_path.to_string(), substring.to_string()),
        ));
    }

    pub fn add_command_check(&mut self, cmd: Vec<String>, expected_code: i32) {
        let label = format!("Command:{}", cmd.first().cloned().unwrap_or_default());
        self.checks
            .push((label, VerifierCheck::CommandRun(cmd, expected_code)));
    }

    pub fn run_all(&self) -> Vec<VerificationResult> {
        let mut results = Vec::new();
        for (name, check) in &self.checks {
            let start = Instant::now();
            let (passed, details) = match check {
                VerifierCheck::FileExists(rel) => {
                    let target = self.workspace_path.join(rel);
                    let exists = target.exists();
                    (
                        exists,
                        if exists {
                            format!("File '{}' exists", rel)
                        } else {
                            format!("File '{}' missing", rel)
                        },
                    )
                }
                VerifierCheck::ContentMatch(rel, needle) => {
                    let target = self.workspace_path.join(rel);
                    if !target.is_file() {
                        (false, format!("File '{}' does not exist", rel))
                    } else {
                        match std::fs::read_to_string(&target) {
                            Ok(text) => {
                                let found = text.contains(needle);
                                (
                                    found,
                                    if found {
                                        format!("Substring '{}' found in '{}'", needle, rel)
                                    } else {
                                        format!("Substring '{}' NOT found in '{}'", needle, rel)
                                    },
                                )
                            }
                            Err(e) => (false, format!("Failed to read file: {}", e)),
                        }
                    }
                }
                VerifierCheck::CommandRun(cmd_args, expected) => {
                    if cmd_args.is_empty() {
                        (false, "Empty command".to_string())
                    } else {
                        let program = &cmd_args[0];
                        let args = &cmd_args[1..];
                        match Command::new(program)
                            .args(args)
                            .current_dir(&self.workspace_path)
                            .output()
                        {
                            Ok(output) => {
                                let code = output.status.code().unwrap_or(-1);
                                let passed = code == *expected;
                                (
                                    passed,
                                    format!("Exit code: {} (expected {})", code, expected),
                                )
                            }
                            Err(e) => (false, format!("Failed to execute command: {}", e)),
                        }
                    }
                }
            };
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            results.push(VerificationResult {
                name: name.clone(),
                passed,
                details,
                duration_ms: elapsed_ms,
            });
        }
        results
    }
}
