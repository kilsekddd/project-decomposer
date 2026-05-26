use decomposer_core::{Budget, Category, Exchange, Phase, Session};

#[test]
fn session_serde_roundtrip() {
    let mut s = Session::new("a CLI that decomposes app ideas", Budget::default());
    s.transcript.push(Exchange {
        category: Category::Users,
        question: "Who's the primary user?".into(),
        answer: "Solo developers".into(),
    });
    s.phase = Phase::Ready;
    s.summary = Some("ok".into());

    let json = serde_json::to_string(&s).unwrap();
    let back: Session = serde_json::from_str(&json).unwrap();

    assert_eq!(back.idea, s.idea);
    assert_eq!(back.slug, "a-cli-that-decomposes-app-ideas");
    assert_eq!(back.transcript.len(), 1);
    assert_eq!(back.transcript[0].category, Category::Users);
    assert_eq!(back.phase, Phase::Ready);
}

#[test]
fn budget_helpers() {
    let mut s = Session::new("x", Budget { min: 2, max: 4 });
    assert!(!s.at_min());
    assert!(!s.at_max());
    for _ in 0..2 {
        s.transcript.push(Exchange {
            category: Category::Problem,
            question: "q".into(),
            answer: "a".into(),
        });
    }
    assert!(s.at_min());
    assert!(!s.at_max());
    for _ in 0..2 {
        s.transcript.push(Exchange {
            category: Category::Problem,
            question: "q".into(),
            answer: "a".into(),
        });
    }
    assert!(s.at_max());
}
