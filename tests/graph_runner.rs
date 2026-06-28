// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the interaction graph and runner.

use praxis::{
    Agent, AgentId, EchoAgent, InteractionGraph, Message, MessageKind, Persona, Runner, Subject,
    Topology, Transcript,
};

#[test]
fn transcript_records_messages_in_order() {
    let mut transcript = Transcript::new();
    assert!(transcript.is_empty());

    let m1 = Message::new(
        AgentId::new("critic-a"),
        None,
        MessageKind::Critique,
        "first",
    );
    let m2 = Message::new(
        AgentId::new("critic-b"),
        None,
        MessageKind::Critique,
        "second",
    );

    transcript.push(m1);
    transcript.push(m2);

    assert_eq!(transcript.len(), 2);
    assert!(!transcript.is_empty());

    let texts: Vec<&str> = transcript.iter().map(|m| m.text()).collect();
    assert_eq!(texts, vec!["first", "second"]);
}

#[test]
fn parallel_topology_builds_graph_with_the_given_critics() {
    let critics = vec![
        AgentId::new("critic-a"),
        AgentId::new("critic-b"),
        AgentId::new("critic-c"),
    ];
    let graph: InteractionGraph = Topology::parallel(critics.clone()).into();

    let node_names: Vec<&str> = graph.critics().iter().map(|id| id.as_str()).collect();
    assert_eq!(node_names, vec!["critic-a", "critic-b", "critic-c"]);
}

#[test]
fn rounds_topology_builds_graph_with_critics_and_max_rounds() {
    let critics = vec![AgentId::new("a"), AgentId::new("b")];
    let graph = InteractionGraph::from(Topology::rounds(critics.clone(), 3));

    assert_eq!(graph.critics(), critics.as_slice());
    assert_eq!(graph.max_rounds(), Some(3));
}

#[test]
fn parallel_graph_has_no_round_cap() {
    // A parallel graph is a degenerate single-round run; max_rounds is None
    // to distinguish it from an explicit rounds topology.
    let graph = InteractionGraph::from(Topology::parallel(vec![AgentId::new("a")]));
    assert_eq!(graph.max_rounds(), None);
}

#[test]
fn runner_executes_parallel_topology_one_critique_per_critic() {
    // A parallel run: each critic receives the subject as a broadcast prompt
    // and produces one critique. Two critics -> two messages in the transcript.
    let graph = InteractionGraph::from(Topology::parallel(vec![
        AgentId::new("critic-a"),
        AgentId::new("critic-b"),
    ]));

    let mut runner = Runner::new(graph)
        .with_agent(EchoAgent::new(
            AgentId::new("critic-a"),
            Persona::new("Critic A"),
        ))
        .with_agent(EchoAgent::new(
            AgentId::new("critic-b"),
            Persona::new("Critic B"),
        ));

    let subject = Subject::from_markdown("# Roadmap\n\nShip the thing.", "roadmap.md");
    let transcript = runner.execute(&subject).expect("echo run never fails");

    assert_eq!(transcript.len(), 2);

    // Each critique is authored by a distinct critic, addressed back to the
    // prompt sender ("system"), and echoes the subject text (echo behavior).
    let senders: Vec<&str> = transcript.iter().map(|m| m.sender().as_str()).collect();
    assert_eq!(senders, vec!["critic-a", "critic-b"]);
    for msg in transcript.iter() {
        assert!(matches!(msg.kind(), MessageKind::Critique));
        assert_eq!(msg.recipient(), Some(&AgentId::new("system")));
        assert_eq!(msg.text(), "# Roadmap\n\nShip the thing.");
    }
}

#[test]
fn runner_preserves_critic_order_from_the_graph() {
    // The transcript must list critiques in the critics' declared order.
    // The report (item 4) and any deterministic diffing rely on this.
    let graph = InteractionGraph::from(Topology::parallel(vec![
        AgentId::new("zeta"),
        AgentId::new("alpha"),
        AgentId::new("middle"),
    ]));

    let mut runner = Runner::new(graph)
        .with_agent(EchoAgent::new(AgentId::new("zeta"), Persona::new("Z")))
        .with_agent(EchoAgent::new(AgentId::new("alpha"), Persona::new("A")))
        .with_agent(EchoAgent::new(AgentId::new("middle"), Persona::new("M")));

    let transcript = runner
        .execute(&Subject::from_markdown("doc", "d.md"))
        .expect("echo run never fails");

    let senders: Vec<&str> = transcript.iter().map(|m| m.sender().as_str()).collect();
    assert_eq!(senders, vec!["zeta", "alpha", "middle"]);
}

#[test]
fn runner_errors_when_a_critic_has_no_registered_agent() {
    // A graph node without a registered agent must surface as a structured
    // MissingAgent error, not a panic.
    let graph = InteractionGraph::from(Topology::parallel(vec![
        AgentId::new("present"),
        AgentId::new("absent"),
    ]));

    let mut runner =
        Runner::new(graph).with_agent(EchoAgent::new(AgentId::new("present"), Persona::new("P")));

    let result = runner.execute(&Subject::from_markdown("doc", "d.md"));
    let err = result.expect_err("should fail on missing agent");
    let rendered = format!("{err}");
    assert!(rendered.contains("absent"));
}

#[test]
fn runner_executes_rounds_topology_critique_then_rebuttal() {
    // Two critics, two rounds: round 1 produces a critique each; round 2 each
    // critic receives the other's critique and rebuts it. Then max_rounds is
    // reached and the run stops.
    let graph = InteractionGraph::from(Topology::rounds(
        vec![AgentId::new("critic-a"), AgentId::new("critic-b")],
        2,
    ));
    let mut runner = Runner::new(graph)
        .with_agent(EchoAgent::new(AgentId::new("critic-a"), Persona::new("A")))
        .with_agent(EchoAgent::new(AgentId::new("critic-b"), Persona::new("B")));

    let transcript = runner
        .execute(&Subject::from_markdown("the claim", "c.md"))
        .expect("echo run never fails");

    let messages: Vec<_> = transcript.iter().collect();
    assert_eq!(messages.len(), 4, "2 critiques + 2 rebuttals");

    // Round 1: critiques in critic order.
    assert_eq!(messages[0].sender(), &AgentId::new("critic-a"));
    assert!(matches!(messages[0].kind(), MessageKind::Critique));
    assert_eq!(messages[1].sender(), &AgentId::new("critic-b"));
    assert!(matches!(messages[1].kind(), MessageKind::Critique));

    // Round 2: each critic rebuts the other's critique, addressed to that critic.
    assert_eq!(messages[2].sender(), &AgentId::new("critic-a"));
    assert_eq!(messages[2].recipient(), Some(&AgentId::new("critic-b")));
    assert!(matches!(messages[2].kind(), MessageKind::Rebuttal));
    assert_eq!(messages[3].sender(), &AgentId::new("critic-b"));
    assert_eq!(messages[3].recipient(), Some(&AgentId::new("critic-a")));
    assert!(matches!(messages[3].kind(), MessageKind::Rebuttal));
}

#[test]
fn runner_stops_early_when_a_round_produces_no_rebuttals() {
    // Echo always rebuts, so it never converges. Use a conceding agent that
    // produces Concessions (not Rebuttals) in response to a critique: round 2
    // then produces zero rebuttals, so the run must stop before max_rounds.
    let graph = InteractionGraph::from(Topology::rounds(
        vec![AgentId::new("a"), AgentId::new("b")],
        5, // generously high; convergence should stop after round 2
    ));
    let mut runner = Runner::new(graph)
        .with_agent(ConcedingAgent::new(AgentId::new("a")))
        .with_agent(ConcedingAgent::new(AgentId::new("b")));

    let transcript = runner
        .execute(&Subject::from_markdown("claim", "c.md"))
        .expect("conceding run never fails");

    let messages: Vec<_> = transcript.iter().collect();
    // Round 1: 2 critiques. Round 2: 2 concessions (no rebuttals) -> stop.
    assert_eq!(
        messages.len(),
        4,
        "should stop after round 2 despite max_rounds=5"
    );
    assert!(matches!(messages[0].kind(), MessageKind::Critique));
    assert!(matches!(messages[1].kind(), MessageKind::Critique));
    assert!(matches!(messages[2].kind(), MessageKind::Concession));
    assert!(matches!(messages[3].kind(), MessageKind::Concession));
}

#[test]
fn runner_runs_to_max_rounds_when_rebuttals_keep_coming() {
    // Echo always rebuts (never converges), so it must run all the way to
    // max_rounds. Two critics, three rounds: 2 critiques + 2 rebuttals + 2
    // rebuttals-of-rebuttals = 6 messages.
    let graph = InteractionGraph::from(Topology::rounds(
        vec![AgentId::new("a"), AgentId::new("b")],
        3,
    ));
    let mut runner = Runner::new(graph)
        .with_agent(EchoAgent::new(AgentId::new("a"), Persona::new("A")))
        .with_agent(EchoAgent::new(AgentId::new("b"), Persona::new("B")));

    let transcript = runner
        .execute(&Subject::from_markdown("claim", "c.md"))
        .expect("echo run never fails");

    assert_eq!(transcript.len(), 6, "2 per round x 3 rounds");
    // Round boundaries: critiques (0..2), rebuttals (2..4), rebuttals (4..6).
    for msg in transcript.iter().take(2) {
        assert!(matches!(msg.kind(), MessageKind::Critique));
    }
    for msg in transcript.iter().skip(2) {
        assert!(matches!(msg.kind(), MessageKind::Rebuttal));
    }
}

/// A test-only agent that concedes (instead of rebutting) when it hears a
/// critique. Used to exercise the rounds convergence early-stop.
struct ConcedingAgent {
    id: AgentId,
    persona: Persona,
}

impl ConcedingAgent {
    fn new(id: AgentId) -> Self {
        Self {
            id: id.clone(),
            persona: Persona::new("Conceding"),
        }
    }
}

impl Agent for ConcedingAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn persona(&self) -> &Persona {
        &self.persona
    }

    fn respond(&mut self, msg: &Message) -> Result<Message, praxis::PraxisError> {
        use praxis::MessageKind as K;
        let kind = match msg.kind() {
            K::Prompt => K::Critique,
            K::Critique => K::Concession,
            other => other,
        };
        Ok(Message::new(
            self.id.clone(),
            Some(msg.sender().clone()),
            kind,
            msg.text().to_owned(),
        ))
    }
}
