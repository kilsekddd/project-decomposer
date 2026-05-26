use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use decomposer_core::{
    engine, provider::anthropic::AnthropicClient, provider::openai::OpenAiClient, render,
    ArtifactKind, Budget, LlmClient, Manifest, Phase, Session,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Provider {
    Anthropic,
    Openai,
}

#[derive(Debug, Parser)]
#[command(
    name = "decomposer",
    version,
    about = "Interview-driven app decomposer. Quizzes you about an idea and emits PRD / Architecture / FILE_TREE / CLAUDE.md / TASKS.md."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// (default flow) A one-line description of the app you want to build.
    idea: Option<String>,

    /// Which LLM provider to use.
    #[arg(long, value_enum, default_value_t = Provider::Anthropic)]
    provider: Provider,

    /// Override the provider's default model.
    #[arg(long)]
    model: Option<String>,

    /// Minimum number of questions before the model may signal_ready.
    #[arg(long, default_value_t = 6)]
    min: usize,

    /// Hard cap on questions. The model is forced to wrap up at this point.
    #[arg(long, default_value_t = 15)]
    max: usize,

    /// Output directory. Defaults to ./decomposed/{slug}/.
    #[arg(long)]
    out: Option<PathBuf>,

    /// Resume from a previous manifest.json. If the session is already Done,
    /// re-renders artifacts. If still Probing, continues the interview.
    #[arg(long)]
    resume: Option<PathBuf>,

    /// Machine-readable mode: one JSON object per line on stdout, JSON
    /// answers expected on stdin.
    #[arg(long)]
    json: bool,
}

/// Subcommands used by external drivers (notably the v2 Claude Code skill)
/// that drive the LLM themselves but want to share decomposer's canonical
/// prompts and manifest layout.
#[derive(Debug, Subcommand)]
enum Command {
    /// Print a canonical prompt to stdout.
    Prompts {
        #[arg(value_enum)]
        kind: PromptKind,
    },

    /// Write the five artifacts + manifest.json from a transcript and a
    /// bodies map produced by an external LLM driver. Used by the v2 Claude
    /// Code plugin, which renders bodies via the host conversation rather
    /// than making API calls itself.
    WriteArtifacts {
        /// The user's one-line app idea. Recorded in the manifest for
        /// traceability. If `--name` is also set, the project name drives
        /// the slug + output dir; otherwise the slug is derived from idea.
        #[arg(long)]
        idea: String,

        /// Concrete project name committed during the interview (e.g.
        /// `diffrep`). When set, the slug + output dir come from this
        /// instead of the (often vague) `--idea` text.
        #[arg(long)]
        name: Option<String>,

        /// JSON file containing a transcript array, i.e.
        /// `[{"category":"problem","question":"...","answer":"..."}, ...]`.
        /// Categories are the same set used by the standalone interview
        /// (problem, users, scope, non_goals, data_model, interfaces,
        /// constraints, risks). Use `-` for stdin.
        #[arg(long)]
        transcript: PathBuf,

        /// JSON file mapping ArtifactKind → markdown body, i.e.
        /// `{"prd":"...", "architecture":"...", "file_tree":"...",
        /// "claude_md":"...", "tasks":"..."}`. Use `-` for stdin.
        #[arg(long)]
        bodies: PathBuf,

        /// Output directory. Defaults to ./decomposed/{slug}/.
        #[arg(long)]
        out: Option<PathBuf>,

        /// Optional readiness summary to record in the session. The
        /// standalone interview captures this from the model's `signal_ready`
        /// turn; the plugin path can omit it or pass through a short note.
        #[arg(long)]
        summary: Option<String>,

        /// Provider name to record in the manifest (informational — no API
        /// call is made by this subcommand). Defaults to "claude-code".
        #[arg(long, default_value = "claude-code")]
        provider: String,

        /// Model name to record in the manifest. Defaults to "claude-code".
        #[arg(long, default_value = "claude-code")]
        model: String,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PromptKind {
    Interviewer,
    Prd,
    Architecture,
    FileTree,
    ClaudeMd,
    Tasks,
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(1)
        }
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        return run_subcommand(cmd);
    }

    let client: Box<dyn LlmClient> = match cli.provider {
        Provider::Anthropic => Box::new(
            AnthropicClient::from_env(cli.model.clone())
                .context("anthropic client: set ANTHROPIC_API_KEY in env")?,
        ),
        Provider::Openai => Box::new(
            OpenAiClient::from_env(cli.model.clone())
                .context("openai client: set OPENAI_API_KEY in env")?,
        ),
    };

    let mut session = if let Some(path) = cli.resume.as_ref() {
        let bytes = std::fs::read(path)
            .with_context(|| format!("reading manifest at {}", path.display()))?;
        let manifest: Manifest = serde_json::from_slice(&bytes)?;
        manifest.session
    } else {
        let idea = cli.idea.clone().ok_or_else(|| anyhow!("missing idea"))?;
        Session::new(
            idea,
            Budget {
                min: cli.min,
                max: cli.max,
            },
        )
    };

    let out_dir = cli
        .out
        .clone()
        .unwrap_or_else(|| PathBuf::from("decomposed").join(&session.slug));

    let resumed_complete = cli.resume.is_some()
        && matches!(session.phase, Phase::Ready | Phase::Done);

    if !resumed_complete {
        drive_interview(&mut session, client.as_ref(), cli.json).await?;
    } else if !cli.json {
        eprintln!(
            "resuming completed session ({} turns) — re-rendering artifacts.",
            session.transcript.len()
        );
    }

    let bodies = engine::render_all(&session, client.as_ref()).await?;
    session.phase = Phase::Done;
    let (manifest_path, _written) =
        render::write_artifacts(&out_dir, &session, client.name(), client.model(), &bodies)?;

    if cli.json {
        let done = serde_json::json!({
            "type": "done",
            "manifest_path": manifest_path,
        });
        println!("{done}");
    } else {
        println!("\nwrote artifacts to {}", out_dir.display());
        println!("manifest: {}", manifest_path.display());
    }
    Ok(())
}

/// Dispatch for the no-LLM subcommands used by the v2 Claude Code plugin.
/// These intentionally never construct an [`LlmClient`] — that's the whole
/// point (no `ANTHROPIC_API_KEY` required on the plugin path).
fn run_subcommand(cmd: Command) -> Result<()> {
    match cmd {
        Command::Prompts { kind } => {
            let body = match kind {
                PromptKind::Interviewer => decomposer_core::interviewer_prompt(),
                PromptKind::Prd => decomposer_core::render_prompt(ArtifactKind::Prd),
                PromptKind::Architecture => {
                    decomposer_core::render_prompt(ArtifactKind::Architecture)
                }
                PromptKind::FileTree => decomposer_core::render_prompt(ArtifactKind::FileTree),
                PromptKind::ClaudeMd => decomposer_core::render_prompt(ArtifactKind::ClaudeMd),
                PromptKind::Tasks => decomposer_core::render_prompt(ArtifactKind::Tasks),
            };
            print!("{body}");
            Ok(())
        }
        Command::WriteArtifacts {
            idea,
            name,
            transcript,
            bodies,
            out,
            summary,
            provider,
            model,
        } => {
            let exchanges: Vec<decomposer_core::Exchange> = read_json(&transcript)
                .with_context(|| format!("reading transcript from {transcript:?}"))?;
            let bodies_map: HashMap<ArtifactKind, String> =
                read_json(&bodies).with_context(|| format!("reading bodies from {bodies:?}"))?;

            let mut ordered = Vec::with_capacity(ArtifactKind::ALL.len());
            for kind in ArtifactKind::ALL {
                let body = bodies_map
                    .get(&kind)
                    .ok_or_else(|| anyhow!("bodies JSON missing artifact: {}", kind.filename()))?;
                ordered.push((kind, strip_outer_fence(body)));
            }

            let mut session = Session::new(idea, Budget::default());
            if let Some(name) = name.as_deref() {
                session.rename(name);
            }
            session.transcript = exchanges;
            session.phase = Phase::Done;
            session.summary = summary;

            let out_dir = out.unwrap_or_else(|| PathBuf::from("decomposed").join(&session.slug));
            let (manifest_path, _written) =
                render::write_artifacts(&out_dir, &session, &provider, &model, &ordered)?;

            let done = serde_json::json!({
                "type": "done",
                "manifest_path": manifest_path,
            });
            println!("{done}");
            Ok(())
        }
    }
}

/// Strip a single outer ```markdown / ``` wrapper if present.
///
/// The plugin path renders bodies via the host Claude conversation, which is
/// prone to wrapping markdown output in an outer fence for display. Bodies
/// produced by the v1 Anthropic/OpenAI providers don't go through this
/// function — it's CLI-side defensive cleanup for `write-artifacts` only.
fn strip_outer_fence(body: &str) -> String {
    let trimmed = body.trim();
    let Some(first_line_end) = trimmed.find('\n') else {
        return body.to_string();
    };
    if !trimmed.ends_with("```") {
        return body.to_string();
    }
    let first_line = &trimmed[..first_line_end];
    // Accept `\`\`\`` or `\`\`\`<lang>` (no internal whitespace) as the opening
    // fence; anything else (e.g. inline code that happens to start a paragraph)
    // is left untouched.
    if !first_line.starts_with("```") || first_line[3..].chars().any(char::is_whitespace) {
        return body.to_string();
    }
    let inner = &trimmed[first_line_end + 1..trimmed.len() - 3];
    let mut out = inner.trim_end().to_string();
    out.push('\n');
    out
}

fn read_json<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Result<T> {
    let bytes = if path.as_os_str() == "-" {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf)?;
        buf
    } else {
        std::fs::read(path)?
    };
    Ok(serde_json::from_slice(&bytes)?)
}

async fn drive_interview(
    session: &mut Session,
    client: &dyn LlmClient,
    json_mode: bool,
) -> Result<()> {
    use std::io::BufRead;

    let stdin = std::io::stdin();
    let mut stdin_lock = stdin.lock();
    let mut editor: Option<rustyline::DefaultEditor> = if json_mode {
        None
    } else {
        Some(rustyline::DefaultEditor::new().context("rustyline init")?)
    };

    loop {
        let event = engine::next_event(session, client).await?;
        match event {
            engine::Event::Question {
                turn,
                of_max,
                category,
                question,
            } => {
                if json_mode {
                    let q = serde_json::json!({
                        "type": "question",
                        "turn": turn,
                        "of_max": of_max,
                        "category": category,
                        "question": question,
                    });
                    println!("{q}");
                    let mut line = String::new();
                    let n = stdin_lock.read_line(&mut line)?;
                    if n == 0 {
                        return Err(anyhow!("stdin closed before answer"));
                    }
                    let v: serde_json::Value = serde_json::from_str(line.trim())
                        .context("expected JSON answer on stdin")?;
                    let answer = v
                        .get("text")
                        .and_then(|t| t.as_str())
                        .ok_or_else(|| anyhow!("answer JSON missing 'text' field"))?
                        .to_string();
                    engine::record_answer(session, category, question, answer);
                } else {
                    prompt_tty(
                        editor.as_mut().unwrap(),
                        session,
                        turn,
                        of_max,
                        category,
                        question,
                    )?;
                }
            }
            engine::Event::Done { .. } => return Ok(()),
        }
    }
}

/// TTY answer collection for a single question.
///
/// Returns when the answer has been committed (pushed to the transcript) OR
/// when the user revised a prior answer via `/back`. In the `/back` case the
/// current question is discarded; the outer loop will fetch a fresh next
/// question from the model on its next iteration.
fn prompt_tty(
    rl: &mut rustyline::DefaultEditor,
    session: &mut Session,
    initial_turn: usize,
    of_max: usize,
    initial_category: decomposer_core::Category,
    initial_question: String,
) -> Result<()> {
    use rustyline::error::ReadlineError;

    let mut turn = initial_turn;
    let mut category = initial_category;
    let mut question = initial_question;
    let mut prefill: Option<String> = None;

    loop {
        println!("\n[{turn}/{of_max}] ({category:?}) {question}");
        println!("  (/back to revise previous answer, /help for commands)");

        let read = match prefill.take() {
            Some(text) => rl.readline_with_initial("> ", (&text, "")),
            None => rl.readline("> "),
        };

        let line = match read {
            Ok(l) => l,
            Err(ReadlineError::Eof) => return Err(anyhow!("stdin closed before answer")),
            Err(ReadlineError::Interrupted) => return Err(anyhow!("interview canceled")),
            Err(e) => return Err(e.into()),
        };
        let trimmed = line.trim();

        if trimmed.is_empty() {
            eprintln!("  (answer cannot be empty — type something, or /back to revise)");
            continue;
        }

        match trimmed {
            "/help" | "/?" => {
                eprintln!(
                    "  commands:\n    /back   discard the previous answer and re-ask it (pre-fills with the old text)\n    /help   show this list"
                );
                continue;
            }
            "/back" => {
                if let Some(prev) = session.transcript.pop() {
                    eprintln!("  (rewound — edit the previous answer below)");
                    category = prev.category;
                    question = prev.question;
                    turn = turn.saturating_sub(1).max(1);
                    prefill = Some(prev.answer);
                    continue;
                } else {
                    eprintln!("  (nothing to undo — you're on the first question)");
                    continue;
                }
            }
            _ => {}
        }

        engine::record_answer(session, category, question, trimmed.to_string());
        return Ok(());
    }
}
