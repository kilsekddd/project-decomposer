//! Core engine for project-decomposer.
//!
//! Public surface:
//! - [`Session`], [`Exchange`], [`Category`], [`Phase`], [`Budget`]
//! - [`engine::run_quiz`] drives the quiz loop against any [`LlmClient`]
//! - [`provider::LlmClient`] trait + provider impls
//! - [`render::render_all`] turns a completed session into the five artifacts
//! - [`Manifest`] is the persisted on-disk record

pub mod engine;
pub mod manifest;
pub mod provider;
pub mod render;
pub mod session;

mod prompts;

pub use manifest::Manifest;
pub use provider::{ArtifactKind, LlmClient, TurnAction};
pub use session::{Budget, Category, Exchange, Phase, Session};

/// System prompt for the interview phase. Exposed so external drivers (e.g.
/// the v2 Claude Code skill, where the parent Claude conversation is the LLM)
/// can reuse the canonical prompt without re-deriving it.
pub fn interviewer_prompt() -> &'static str {
    prompts::INTERVIEWER_SYSTEM
}

/// Render-phase system prompt for a given artifact kind.
pub fn render_prompt(kind: ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::Prd => prompts::RENDER_PRD,
        ArtifactKind::Architecture => prompts::RENDER_ARCHITECTURE,
        ArtifactKind::FileTree => prompts::RENDER_FILE_TREE,
        ArtifactKind::ClaudeMd => prompts::RENDER_CLAUDE_MD,
        ArtifactKind::Tasks => prompts::RENDER_TASKS,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("provider returned malformed response: {0}")]
    Protocol(String),
    #[error("missing credential: {0}")]
    MissingCredential(&'static str),
    #[error("budget violated: {0}")]
    Budget(String),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
