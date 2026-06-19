// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the interaction graph and runner.

use praxis::{
    AgentId, EchoAgent, InteractionGraph, Message, MessageKind, Persona, Runner, Subject, Topology,
    Transcript,
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
