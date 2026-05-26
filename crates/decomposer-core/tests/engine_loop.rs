mod common;

use common::MockClient;
use decomposer_core::engine::{self, Event};
use decomposer_core::provider::{ArtifactKind, TurnAction};
use decomposer_core::{Budget, Category, Phase, Session};

fn ask(cat: Category, q: &str) -> TurnAction {
    TurnAction::Ask {
        category: cat,
        question: q.into(),
        rationale: "because".into(),
    }
}

fn ready(s: &str) -> TurnAction {
    TurnAction::Ready {
        summary: s.into(),
        project_name: None,
    }
}

fn ready_named(s: &str, name: &str) -> TurnAction {
    TurnAction::Ready {
        summary: s.into(),
        project_name: Some(name.into()),
    }
}

async fn answer_loop(session: &mut Session, client: &MockClient) -> Vec<String> {
    let mut questions = Vec::new();
    loop {
        match engine::next_event(session, client).await.unwrap() {
            Event::Question {
                category,
                question,
                ..
            } => {
                questions.push(question.clone());
                engine::record_answer(session, category, question, "an answer".into());
            }
            Event::Done { .. } => break,
        }
    }
    questions
}

#[tokio::test]
async fn happy_path_min_questions_then_ready() {
    let mut session = Session::new("a test idea", Budget { min: 3, max: 10 });
    let mock = MockClient::new(vec![
        ask(Category::Problem, "Q1?"),
        ask(Category::Users, "Q2?"),
        ask(Category::Scope, "Q3?"),
        ready("looks good"),
    ]);

    let qs = answer_loop(&mut session, &mock).await;
    assert_eq!(qs.len(), 3);
    assert_eq!(session.phase, Phase::Ready);
    assert_eq!(session.summary.as_deref(), Some("looks good"));
}

#[tokio::test]
async fn ready_before_min_is_rejected() {
    let mut session = Session::new("idea", Budget { min: 4, max: 10 });
    let mock = MockClient::new(vec![
        ask(Category::Problem, "Q1?"),
        ready("too soon"),
    ]);

    engine::next_event(&mut session, &mock).await.unwrap();
    engine::record_answer(&mut session, Category::Problem, "Q1?".into(), "a".into());

    let err = engine::next_event(&mut session, &mock).await.unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("min is 4"), "got: {msg}");
    assert_eq!(session.phase, Phase::Probing);
    assert!(session.summary.is_none());
}

#[tokio::test]
async fn ask_past_max_is_rejected() {
    let mut session = Session::new("idea", Budget { min: 1, max: 2 });
    let mock = MockClient::new(vec![
        ask(Category::Problem, "Q1?"),
        ask(Category::Users, "Q2?"),
        ask(Category::Scope, "Q3?"),
    ]);

    engine::next_event(&mut session, &mock).await.unwrap();
    engine::record_answer(&mut session, Category::Problem, "Q1?".into(), "a".into());
    engine::next_event(&mut session, &mock).await.unwrap();
    engine::record_answer(&mut session, Category::Users, "Q2?".into(), "a".into());

    let err = engine::next_event(&mut session, &mock).await.unwrap_err();
    assert!(format!("{err}").contains("past the hard cap"));
}

#[tokio::test]
async fn forced_finish_at_max_accepted() {
    let mut session = Session::new("idea", Budget { min: 1, max: 2 });
    let mock = MockClient::new(vec![
        ask(Category::Problem, "Q1?"),
        ask(Category::Users, "Q2?"),
        ready("done at cap"),
    ]);

    engine::next_event(&mut session, &mock).await.unwrap();
    engine::record_answer(&mut session, Category::Problem, "Q1?".into(), "a".into());
    engine::next_event(&mut session, &mock).await.unwrap();
    engine::record_answer(&mut session, Category::Users, "Q2?".into(), "a".into());

    let ev = engine::next_event(&mut session, &mock).await.unwrap();
    assert!(matches!(ev, Event::Done { .. }));
    assert_eq!(session.phase, Phase::Ready);
}

#[tokio::test]
async fn resume_from_ready_short_circuits_interview() {
    let mut session = Session::new("idea", Budget { min: 1, max: 5 });
    session.transcript.push(decomposer_core::Exchange {
        category: Category::Problem,
        question: "Q1?".into(),
        answer: "A1".into(),
    });
    session.phase = Phase::Ready;
    session.summary = Some("already done".into());

    // Empty script — would error if any next_turn call leaked through.
    let mock = MockClient::new(vec![]);
    let qs = answer_loop(&mut session, &mock).await;
    assert!(qs.is_empty());
    assert_eq!(session.phase, Phase::Ready);
}

#[tokio::test]
async fn resume_from_done_short_circuits_interview() {
    let mut session = Session::new("idea", Budget { min: 1, max: 5 });
    session.transcript.push(decomposer_core::Exchange {
        category: Category::Problem,
        question: "Q1?".into(),
        answer: "A1".into(),
    });
    session.phase = Phase::Done;

    let mock = MockClient::new(vec![]);
    let qs = answer_loop(&mut session, &mock).await;
    assert!(qs.is_empty());
    assert_eq!(session.phase, Phase::Done);
}

#[tokio::test]
async fn render_all_returns_five_kinds() {
    let mut session = Session::new("a useful tool", Budget { min: 1, max: 2 });
    let mock = MockClient::new(vec![ask(Category::Problem, "Q1?"), ready("ok")]);

    engine::next_event(&mut session, &mock).await.unwrap();
    engine::record_answer(&mut session, Category::Problem, "Q1?".into(), "a".into());
    engine::next_event(&mut session, &mock).await.unwrap();

    let bodies = engine::render_all(&session, &mock).await.unwrap();
    let kinds: Vec<ArtifactKind> = bodies.iter().map(|(k, _)| *k).collect();
    assert_eq!(kinds, ArtifactKind::ALL.to_vec());
    for (_, body) in &bodies {
        assert!(body.contains("a-useful-tool"));
    }
}

#[tokio::test]
async fn ready_with_project_name_renames_session() {
    let mut session = Session::new(
        "a CLI tool that summarizes git diffs",
        Budget { min: 1, max: 5 },
    );
    assert_eq!(session.slug, "a-cli-tool-that-summarizes-git-diffs");

    let mock = MockClient::new(vec![
        ask(Category::Stack, "Q1?"),
        ready_named("user committed diffrep as the binary name", "diffrep"),
    ]);

    engine::next_event(&mut session, &mock).await.unwrap();
    engine::record_answer(&mut session, Category::Stack, "Q1?".into(), "diffrep".into());
    engine::next_event(&mut session, &mock).await.unwrap();

    assert_eq!(
        session.slug, "diffrep",
        "slug must be re-derived from the committed project_name"
    );
    assert_eq!(
        session.idea, "a CLI tool that summarizes git diffs",
        "original idea string must be preserved for traceability"
    );
    assert!(matches!(session.phase, Phase::Ready));
}
