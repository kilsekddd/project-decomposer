use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use super::{ArtifactKind, LlmClient, TurnAction};
use crate::prompts;
use crate::provider::anthropic::{format_render_prompt, format_turn_prompt};
use crate::session::Category;
use crate::{Error, Result, Session};

pub const DEFAULT_MODEL: &str = "gpt-5";
const API_URL: &str = "https://api.openai.com/v1/chat/completions";

pub struct OpenAiClient {
    api_key: String,
    model: String,
    http: reqwest::Client,
}

impl OpenAiClient {
    pub fn new(api_key: impl Into<String>, model: Option<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            http: reqwest::Client::new(),
        }
    }

    pub fn from_env(model: Option<String>) -> Result<Self> {
        let key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| Error::MissingCredential("OPENAI_API_KEY"))?;
        Ok(Self::new(key, model))
    }

    async fn post(&self, body: Value) -> Result<ChatResponse> {
        let resp = self
            .http
            .post(API_URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Protocol(format!("openai HTTP {status}: {text}")));
        }
        serde_json::from_str(&text).map_err(Error::from)
    }
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ToolCall>,
}

#[derive(Deserialize)]
struct ToolCall {
    function: ToolCallFunction,
}

#[derive(Deserialize)]
struct ToolCallFunction {
    name: String,
    arguments: String,
}

#[async_trait]
impl LlmClient for OpenAiClient {
    fn name(&self) -> &'static str {
        "openai"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn next_turn(&self, session: &Session, must_finish: bool) -> Result<TurnAction> {
        let body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": prompts::INTERVIEWER_SYSTEM},
                {"role": "user", "content": format_turn_prompt(session, must_finish)},
            ],
            "tools": tool_schemas(),
            "tool_choice": "required",
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
        let body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": format_render_prompt(session, prior)},
            ],
        });

        let resp = self.post(body).await?;
        resp.choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| Error::Protocol("openai render returned no content".into()))
    }
}

fn tool_schemas() -> Value {
    let category_enum = json!([
        "problem", "users", "scope", "non_goals",
        "data_model", "interfaces", "constraints", "risks"
    ]);
    json!([
        {
            "type": "function",
            "function": {
                "name": "ask_next_question",
                "description": "Ask the next interview question.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "category": {"type": "string", "enum": category_enum},
                        "question": {"type": "string"},
                        "rationale": {"type": "string"},
                    },
                    "required": ["category", "question", "rationale"],
                },
            },
        },
        {
            "type": "function",
            "function": {
                "name": "signal_ready",
                "description": "Signal that the interview has enough information to write the artifacts.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "summary": {"type": "string"},
                    },
                    "required": ["summary"],
                },
            },
        }
    ])
}

fn parse_turn_response(resp: ChatResponse, must_finish: bool) -> Result<TurnAction> {
    let choice = resp
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| Error::Protocol("openai response had no choices".into()))?;
    let call = choice
        .message
        .tool_calls
        .into_iter()
        .next()
        .ok_or_else(|| {
            Error::Protocol(format!(
                "openai returned no tool_calls (must_finish={must_finish}, content={:?})",
                choice.message.content
            ))
        })?;
    let args: Value = serde_json::from_str(&call.function.arguments)
        .map_err(|e| Error::Protocol(format!("bad tool arguments: {e}")))?;
    match call.function.name.as_str() {
        "ask_next_question" => {
            let category: Category =
                serde_json::from_value(args.get("category").cloned().unwrap_or(Value::Null))
                    .map_err(|e| Error::Protocol(format!("bad category: {e}")))?;
            let question = args
                .get("question")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Protocol("missing question".into()))?
                .to_string();
            let rationale = args
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
            let summary = args
                .get("summary")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Protocol("missing summary".into()))?
                .to_string();
            Ok(TurnAction::Ready { summary })
        }
        other => Err(Error::Protocol(format!("unknown tool {other}"))),
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
