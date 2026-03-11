use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportScope {
    Full,
    PrefixOnly,
    GameOnly,
    LibsOnly,
}

impl ExportScope {
    pub const ALL: [ExportScope; 4] = [
        ExportScope::Full,
        ExportScope::PrefixOnly,
        ExportScope::GameOnly,
        ExportScope::LibsOnly,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ExportScope::Full => "Pełna paczka",
            ExportScope::PrefixOnly => "Tylko prefix",
            ExportScope::GameOnly => "Tylko gra",
            ExportScope::LibsOnly => "Biblioteki / prefix",
        }
    }
}

impl Default for ExportScope {
    fn default() -> Self {
        ExportScope::Full
    }
}

impl fmt::Display for ExportScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportAudit {
    pub label: String,
    pub sha256: String,
}

impl ExportAudit {
    pub fn new(label: impl Into<String>, sha256: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            sha256: sha256.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExportArtifact {
    pub installer_path: PathBuf,
    pub audits: Vec<ExportAudit>,
    pub scope: ExportScope,
    pub dry_run: bool,
    pub source_path: PathBuf,
    pub prefix_path: PathBuf,
}
