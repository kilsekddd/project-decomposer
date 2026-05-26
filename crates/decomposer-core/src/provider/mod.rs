use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{session::Category, Result, Session};

pub mod anthropic;
pub mod openai;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TurnAction {
    Ask {
        category: Category,
        question: String,
        rationale: String,
    },
    Ready {
        summary: String,
        /// Concrete project name (binary / crate / mod-id) when the
        /// developer committed one during the interview. Used by the
        /// engine to re-slug the session before render. `None` if naming
        /// was deferred to the architect.
        #[serde(default)]
        project_name: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Prd,
    Architecture,
    FileTree,
    ClaudeMd,
    Tasks,
}

impl ArtifactKind {
    pub fn filename(self) -> &'static str {
        match self {
            ArtifactKind::Prd => "PRD.md",
            ArtifactKind::Architecture => "ARCHITECTURE.md",
            ArtifactKind::FileTree => "FILE_TREE.md",
            ArtifactKind::ClaudeMd => "CLAUDE.md",
            ArtifactKind::Tasks => "TASKS.md",
        }
    }

    pub const ALL: [ArtifactKind; 5] = [
        ArtifactKind::Prd,
        ArtifactKind::Architecture,
        ArtifactKind::FileTree,
        ArtifactKind::ClaudeMd,
        ArtifactKind::Tasks,
    ];
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    fn name(&self) -> &'static str;
    fn model(&self) -> &str;

    async fn next_turn(&self, session: &Session, must_finish: bool) -> Result<TurnAction>;

    /// Render one artifact. `prior` carries already-rendered artifacts so the
    /// model can stay consistent with them (e.g. PRD names, non-goals).
    async fn render(
        &self,
        session: &Session,
        kind: ArtifactKind,
        prior: &[(ArtifactKind, &str)],
    ) -> Result<String>;
}
