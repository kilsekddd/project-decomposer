use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{provider::ArtifactKind, render::WrittenArtifact, Session};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestArtifact {
    pub kind: ArtifactKind,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub slug: String,
    pub idea: String,
    pub provider: String,
    pub model: String,
    pub created_at: DateTime<Utc>,
    pub session: Session,
    pub artifacts: Vec<ManifestArtifact>,
}

impl Manifest {
    pub fn build(
        session: &Session,
        provider: &str,
        model: &str,
        written: &[WrittenArtifact],
    ) -> Self {
        Self {
            version: 1,
            slug: session.slug.clone(),
            idea: session.idea.clone(),
            provider: provider.to_string(),
            model: model.to_string(),
            created_at: Utc::now(),
            session: session.clone(),
            artifacts: written
                .iter()
                .map(|a| ManifestArtifact {
                    kind: a.kind,
                    path: a.path.clone(),
                })
                .collect(),
        }
    }
}
