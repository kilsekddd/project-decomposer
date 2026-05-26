use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Problem,
    Users,
    Scope,
    NonGoals,
    DataModel,
    Interfaces,
    Stack,
    Constraints,
    Risks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Probing,
    Ready,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub min: usize,
    pub max: usize,
}

impl Default for Budget {
    fn default() -> Self {
        Self { min: 6, max: 15 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exchange {
    pub category: Category,
    pub question: String,
    pub answer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub idea: String,
    pub slug: String,
    pub budget: Budget,
    pub phase: Phase,
    pub transcript: Vec<Exchange>,
    pub summary: Option<String>,
}

impl Session {
    pub fn new(idea: impl Into<String>, budget: Budget) -> Self {
        let idea = idea.into();
        let slug = slug::slugify(&idea);
        Self {
            idea,
            slug,
            budget,
            phase: Phase::Probing,
            transcript: Vec::new(),
            summary: None,
        }
    }

    /// Re-derive the slug from a committed project name. Use when the
    /// interview's `idea` string was a vague description ("a CLI tool that
    /// summarizes git diffs") but a concrete project name was decided
    /// during the interview ("diffrep"). The output directory and manifest
    /// follow the new slug; the original `idea` is preserved for traceability.
    pub fn rename(&mut self, project_name: &str) {
        self.slug = slug::slugify(project_name);
    }

    pub fn turn(&self) -> usize {
        self.transcript.len()
    }

    pub fn at_max(&self) -> bool {
        self.transcript.len() >= self.budget.max
    }

    pub fn at_min(&self) -> bool {
        self.transcript.len() >= self.budget.min
    }
}
