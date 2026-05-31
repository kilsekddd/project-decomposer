//! Provider-agnostic system prompts and tool/function schemas.
//!
//! Keep these in one place so the Anthropic and OpenAI impls behave
//! identically up to their respective structured-output mechanisms.
//!
//! Single source of truth: the prompt `.md` files physically live in the
//! Claude Code plugin (`plugin/decompose/skills/decompose/prompts/`) so the
//! plugin can read them directly with no native binary on its path. The
//! standalone CLI compiles the same files in via `include_str!` across the
//! repo tree, so there's exactly one copy and the two products can't drift.
//! Note: this cross-tree `include_str!` works for `cargo build` and
//! `cargo install --git`/`--path` (the whole repo is present); it would NOT
//! survive `cargo publish` to crates.io (out-of-crate file), which this
//! workspace does not do.

#![allow(dead_code)] // wired up by provider impls

pub const INTERVIEWER_SYSTEM: &str =
    include_str!("../../../../plugin/decompose/skills/decompose/prompts/interviewer.md");

pub const RENDER_PRD: &str =
    include_str!("../../../../plugin/decompose/skills/decompose/prompts/render_prd.md");
pub const RENDER_ARCHITECTURE: &str =
    include_str!("../../../../plugin/decompose/skills/decompose/prompts/render_architecture.md");
pub const RENDER_FILE_TREE: &str =
    include_str!("../../../../plugin/decompose/skills/decompose/prompts/render_file_tree.md");
pub const RENDER_CLAUDE_MD: &str =
    include_str!("../../../../plugin/decompose/skills/decompose/prompts/render_claude_md.md");
pub const RENDER_TASKS: &str =
    include_str!("../../../../plugin/decompose/skills/decompose/prompts/render_tasks.md");
