use std::collections::VecDeque;
use std::sync::Mutex;

use async_trait::async_trait;
use decomposer_core::provider::{ArtifactKind, LlmClient, TurnAction};
use decomposer_core::{Error, Result, Session};

/// Replays a scripted sequence of `TurnAction`s for `next_turn`.
/// For `render`, returns a synthetic string per artifact kind.
pub struct MockClient {
    pub script: Mutex<VecDeque<TurnAction>>,
    pub name: &'static str,
    pub model: String,
}

impl MockClient {
    pub fn new(script: Vec<TurnAction>) -> Self {
        Self {
            script: Mutex::new(VecDeque::from(script)),
            name: "mock",
            model: "mock-1".to_string(),
        }
    }
}

#[async_trait]
impl LlmClient for MockClient {
    fn name(&self) -> &'static str {
        self.name
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn next_turn(&self, _session: &Session, _must_finish: bool) -> Result<TurnAction> {
        self.script
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| Error::Other("mock script exhausted".into()))
    }

    async fn render(
        &self,
        session: &Session,
        kind: ArtifactKind,
        _prior: &[(ArtifactKind, &str)],
    ) -> Result<String> {
        Ok(format!(
            "# {:?} for {}\n\n(turns: {})\n",
            kind,
            session.slug,
            session.transcript.len()
        ))
    }
}
