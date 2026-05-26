//! Filesystem-side of artifact generation.
//!
//! The LLM produces the markdown bodies (see [`engine::render_all`]); this
//! module is responsible for writing them to disk alongside the manifest.

use std::path::{Path, PathBuf};

use crate::{provider::ArtifactKind, Manifest, Result, Session};

pub struct WrittenArtifact {
    pub kind: ArtifactKind,
    pub path: PathBuf,
}

pub fn write_artifacts(
    out_dir: &Path,
    session: &Session,
    provider: &str,
    model: &str,
    bodies: &[(ArtifactKind, String)],
) -> Result<(PathBuf, Vec<WrittenArtifact>)> {
    std::fs::create_dir_all(out_dir)?;

    let mut written = Vec::with_capacity(bodies.len());
    for (kind, body) in bodies {
        let path = out_dir.join(kind.filename());
        std::fs::write(&path, body)?;
        written.push(WrittenArtifact { kind: *kind, path });
    }

    let manifest = Manifest::build(session, provider, model, &written);
    let manifest_path = out_dir.join("manifest.json");
    std::fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

    Ok((manifest_path, written))
}
