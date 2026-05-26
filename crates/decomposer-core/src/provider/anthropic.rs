use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use super::{ArtifactKind, LlmClient, TurnAction};
use crate::prompts;
use crate::session::Category;
use crate::{Error, Result, Session};

pub const DEFAULT_MODEL: &str = "claude-opus-4-7";
const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";
const TURN_MAX_TOKENS: u32 = 1024;
const RENDER_MAX_TOKENS: u32 = 4096;

pub struct AnthropicClient {
    api_key: String,
    model: String,
    http: reqwest::Client,
}

impl AnthropicClient {
    pub fn new(api_key: impl Into<String>, model: Option<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            http: reqwest::Client::new(),
        }
    }

    pub fn from_env(model: Option<String>) -> Result<Self> {
        let key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| Error::MissingCredential("ANTHROPIC_API_KEY"))?;
        Ok(Self::new(key, model))
    }

    async fn post(&self, body: Value) -> Result<MessagesResponse> {
        let resp = self
            .http
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Protocol(format!(
                "anthropic HTTP {status}: {text}"
            )));
        }
        serde_json::from_str(&text).map_err(Error::from)
    }
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        #[allow(dead_code)]
        id: String,
        name: String,
        input: Value,
    },
}

#[async_trait]
impl LlmClient for AnthropicClient {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn next_turn(&self, session: &Session, must_finish: bool) -> Result<TurnAction> {
        let user_content = format_turn_prompt(session, must_finish);

        let body = json!({
            "model": self.model,
            "max_tokens": TURN_MAX_TOKENS,
            "system": [{
                "type": "text",
                "text": prompts::INTERVIEWER_SYSTEM,
                "cache_control": {"type": "ephemeral"},
            }],
            "tools": tool_schemas(),
            "tool_choice": {"type": "any"},
            "messages": [{
                "role": "user",
                "content": user_content,
            }],
        });

        let resp = self.post(body).await?;
        parse_turn_response(resp, must_finish)
    }

    async fn render(
        &self,
        session: &Session,
        kind: ArtifactKind,
        prior: &[(ArtifactKind, &str)],
    ) -> Result<String> {
        let system_prompt = render_system_prompt(kind);
        let user_content = format_render_prompt(session, prior);

        let body = json!({
            "model": self.model,
            "max_tokens": RENDER_MAX_TOKENS,
            "system": system_prompt,
            "messages": [{
                "role": "user",
                "content": user_content,
            }],
        });

        let resp = self.post(body).await?;
        extract_text(&resp.content).ok_or_else(|| {
            Error::Protocol(format!(
                "no text block in render response (stop_reason={:?})",
                resp.stop_reason
            ))
        })
    }
}

fn tool_schemas() -> Value {
    json!([
        {
            "name": "ask_next_question",
            "description": "Ask the next interview question.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "enum": [
                            "problem", "users", "scope", "non_goals",
                            "data_model", "interfaces", "constraints", "risks"
                        ],
                    },
                    "question": {"type": "string", "description": "One question, under 25 words."},
                    "rationale": {"type": "string", "description": "Why this question is the right next step."},
                },
                "required": ["category", "question", "rationale"],
            },
        },
        {
            "name": "signal_ready",
            "description": "Signal that the interview has enough information to write the artifacts.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "summary": {
                        "type": "string",
                        "description": "3-5 sentence synthesis of the project for downstream renderers.",
                    },
                },
                "required": ["summary"],
            },
        }
    ])
}

fn parse_turn_response(resp: MessagesResponse, must_finish: bool) -> Result<TurnAction> {
    for block in resp.content.iter() {
        if let ContentBlock::ToolUse { name, input, .. } = block {
            return match name.as_str() {
                "ask_next_question" => {
                    let category: Category = serde_json::from_value(
                        input.get("category").cloned().unwrap_or(Value::Null),
                    )
                    .map_err(|e| Error::Protocol(format!("bad category: {e}")))?;
                    let question = input
                        .get("question")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| Error::Protocol("missing question".into()))?
                        .to_string();
                    let rationale = input
                        .get("rationale")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    Ok(TurnAction::Ask {
                        category,
                        question,
                        rationale,
                    })
                }
                "signal_ready" => {
                    let summary = input
                        .get("summary")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| Error::Protocol("missing summary".into()))?
                        .to_string();
                    Ok(TurnAction::Ready { summary })
                }
                other => Err(Error::Protocol(format!("unknown tool {other}"))),
            };
        }
    }
    let texts: Vec<&str> = resp
        .content
        .iter()
        .filter_map(|b| match b {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    Err(Error::Protocol(format!(
        "no tool_use in response (must_finish={must_finish}, text={:?})",
        texts.join(" | ")
    )))
}

fn extract_text(content: &[ContentBlock]) -> Option<String> {
    let mut out = String::new();
    for block in content {
        if let ContentBlock::Text { text } = block {
            out.push_str(text);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn render_system_prompt(kind: ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::Prd => prompts::RENDER_PRD,
        ArtifactKind::Architecture => prompts::RENDER_ARCHITECTURE,
        ArtifactKind::FileTree => prompts::RENDER_FILE_TREE,
        ArtifactKind::ClaudeMd => prompts::RENDER_CLAUDE_MD,
        ArtifactKind::Tasks => prompts::RENDER_TASKS,
    }
}

pub(crate) fn format_turn_prompt(session: &Session, must_finish: bool) -> String {
    let mut s = String::new();
    s.push_str("Project idea: ");
    s.push_str(&session.idea);
    s.push_str("\n\n");
    if session.transcript.is_empty() {
        s.push_str("This is the start of the interview. Ask the first question.\n");
    } else {
        s.push_str("Interview so far:\n\n");
        for (i, ex) in session.transcript.iter().enumerate() {
            let cat = category_str(ex.category);
            s.push_str(&format!("Q{} ({cat}): {}\n", i + 1, ex.question));
            s.push_str(&format!("A{}: {}\n\n", i + 1, ex.answer));
        }
    }
    s.push_str(&format!(
        "Budget: {} of {} questions used.\n",
        session.transcript.len(),
        session.budget.max
    ));
    if must_finish {
        s.push_str("You have reached the question budget. Call signal_ready now.\n");
    } else if session.transcript.len() >= session.budget.min {
        s.push_str("Minimum budget reached; you may signal_ready when satisfied.\n");
    }
    s
}

pub(crate) fn format_render_prompt(
    session: &Session,
    prior: &[(ArtifactKind, &str)],
) -> String {
    let mut s = String::new();
    s.push_str("Project idea: ");
    s.push_str(&session.idea);
    s.push_str("\n\nInterview transcript:\n\n");
    for (i, ex) in session.transcript.iter().enumerate() {
        let cat = category_str(ex.category);
        s.push_str(&format!("Q{} ({cat}): {}\n", i + 1, ex.question));
        s.push_str(&format!("A{}: {}\n\n", i + 1, ex.answer));
    }
    if let Some(sum) = &session.summary {
        s.push_str("Interviewer's wrap-up summary:\n");
        s.push_str(sum);
        s.push('\n');
    }
    if !prior.is_empty() {
        s.push_str(
            "\nAlready-finalized artifacts (treat as canonical — do not contradict their \
             names, scope decisions, or non-goals):\n\n",
        );
        for (kind, body) in prior {
            s.push_str(&format!("--- BEGIN {} ---\n", kind.filename()));
            s.push_str(body);
            if !body.ends_with('\n') {
                s.push('\n');
            }
            s.push_str(&format!("--- END {} ---\n\n", kind.filename()));
        }
    }
    s
}

fn category_str(c: Category) -> &'static str {
    match c {
        Category::Problem => "problem",
        Category::Users => "users",
        Category::Scope => "scope",
        Category::NonGoals => "non_goals",
        Category::DataModel => "data_model",
        Category::Interfaces => "interfaces",
        Category::Stack => "stack",
        Category::Constraints => "constraints",
        Category::Risks => "risks",
    }
}

