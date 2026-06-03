use decomposer_core::{render, ArtifactKind, Budget, Category, Exchange, Manifest, Phase, Session};

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

#[test]
fn write_artifacts_emits_manifest_v2_with_agents_md() {
    let mut s = Session::new("a tiny app", Budget::default());
    s.summary = Some("ready".into());
    s.phase = Phase::Done;

    let out_dir = std::env::temp_dir().join(format!(
        "decomposer-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let bodies = vec![
        (ArtifactKind::Prd, "# PRD\n".to_string()),
        (ArtifactKind::Architecture, "# Architecture\n".to_string()),
        (ArtifactKind::FileTree, "```tree\n.\n```\n".to_string()),
        (ArtifactKind::ClaudeMd, "# Guidance\n".to_string()),
        (ArtifactKind::Tasks, "# Tasks\n".to_string()),
    ];

    let (manifest_path, written) =
        render::write_artifacts(&out_dir, &s, "test-provider", "test-model", &bodies).unwrap();
    let manifest: Manifest =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();

    assert_eq!(manifest.version, 2);
    assert_eq!(manifest.artifacts.len(), 6);
    assert!(written.iter().any(|a| a.kind == ArtifactKind::AgentsMd));
    assert_eq!(
        std::fs::read_to_string(out_dir.join("CLAUDE.md")).unwrap(),
        std::fs::read_to_string(out_dir.join("AGENTS.md")).unwrap()
    );
    assert!(manifest
        .artifacts
        .iter()
        .any(|a| a.kind == ArtifactKind::AgentsMd && a.path.ends_with("AGENTS.md")));

    std::fs::remove_dir_all(out_dir).unwrap();
}
