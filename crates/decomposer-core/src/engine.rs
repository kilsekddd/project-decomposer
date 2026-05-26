use crate::{
    provider::{ArtifactKind, LlmClient, TurnAction},
    session::Phase,
    Error, Result, Session,
};

/// Driver-facing event emitted on each loop iteration.
///
/// The CLI translates these into either TTY prompts or `--json` lines.
#[derive(Debug)]
pub enum Event {
    Question {
        turn: usize,
        of_max: usize,
        category: crate::session::Category,
        question: String,
    },
    Done {
        summary: String,
    },
}

/// One step of the quiz loop. The CLI:
///   1. calls `next_event` to get a Question or Done,
///   2. on Question, collects the user's answer and calls `record_answer`,
///   3. repeats until Done.
pub async fn next_event(session: &mut Session, client: &dyn LlmClient) -> Result<Event> {
    if matches!(session.phase, Phase::Ready | Phase::Done) {
        return Ok(Event::Done {
            summary: session.summary.clone().unwrap_or_default(),
        });
    }

    let must_finish = session.at_max();
    let action = client.next_turn(session, must_finish).await?;

    match action {
        TurnAction::Ask { .. } if must_finish => Err(Error::Budget(
            "model attempted to keep asking past the hard cap".into(),
        )),
        TurnAction::Ask {
            category, question, ..
        } => Ok(Event::Question {
            turn: session.turn() + 1,
            of_max: session.budget.max,
            category,
            question,
        }),
        TurnAction::Ready { summary } if session.at_min() || must_finish => {
            session.phase = Phase::Ready;
            session.summary = Some(summary.clone());
            Ok(Event::Done { summary })
        }
        TurnAction::Ready { .. } => Err(Error::Budget(format!(
            "model signalled ready at turn {} but min is {}",
            session.turn(),
            session.budget.min
        ))),
    }
}

pub fn record_answer(
    session: &mut Session,
    category: crate::session::Category,
    question: String,
    answer: String,
) {
    session.transcript.push(crate::session::Exchange {
        category,
        question,
        answer,
    });
}

/// Render all five artifacts in three stages:
///   1. PRD alone — establishes names, scope, non-goals.
///   2. ARCHITECTURE with PRD as context — resolves any stack/language
///      ambiguity the PRD left open.
///   3. FILE_TREE, CLAUDE.md, TASKS in parallel with both PRD and
///      ARCHITECTURE as context, so they inherit the same concrete decisions.
pub async fn render_all(
    session: &Session,
    client: &dyn LlmClient,
) -> Result<Vec<(ArtifactKind, String)>> {
    let prd = client.render(session, ArtifactKind::Prd, &[]).await?;

    let prd_only = [(ArtifactKind::Prd, prd.as_str())];
    let arch = client
        .render(session, ArtifactKind::Architecture, &prd_only)
        .await?;

    let prior = [
        (ArtifactKind::Prd, prd.as_str()),
        (ArtifactKind::Architecture, arch.as_str()),
    ];
    let (tree, claude_md, tasks) = tokio::join!(
        client.render(session, ArtifactKind::FileTree, &prior),
        client.render(session, ArtifactKind::ClaudeMd, &prior),
        client.render(session, ArtifactKind::Tasks, &prior),
    );
    Ok(vec![
        (ArtifactKind::Prd, prd),
        (ArtifactKind::Architecture, arch),
        (ArtifactKind::FileTree, tree?),
        (ArtifactKind::ClaudeMd, claude_md?),
        (ArtifactKind::Tasks, tasks?),
    ])
}
