//! Provider-agnostic system prompts and tool/function schemas.
//!
//! Keep these in one place so the Anthropic and OpenAI impls behave
//! identically up to their respective structured-output mechanisms.

#![allow(dead_code)] // wired up by provider impls

pub const INTERVIEWER_SYSTEM: &str = include_str!("interviewer.md");

pub const RENDER_PRD: &str = include_str!("render_prd.md");
pub const RENDER_ARCHITECTURE: &str = include_str!("render_architecture.md");
pub const RENDER_FILE_TREE: &str = include_str!("render_file_tree.md");
pub const RENDER_CLAUDE_MD: &str = include_str!("render_claude_md.md");
pub const RENDER_TASKS: &str = include_str!("render_tasks.md");
