//! Integration tests for PBFT-lite Byzantine consensus.

use octo_engine::agent::collaboration::{
    ByzantineProposal, CollaborationMessage, ConsensusPhase, ConsensusVote, PhaseAdvanceResult,
};

fn make_vote(agent_id: &str, approve: bool, phase: ConsensusPhase) -> ConsensusVote {
    ConsensusVote {
        agent_id: agent_id.to_string(),
        approve,
        phase,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

// ─── Quorum threshold tests ───

#[test]
fn quorum_threshold_n1() {
    let p = ByzantineProposal::new("p".into(), "l".into(), "a".into(), "d".into(), 1);
    assert_eq!(p.quorum_threshold(), 1); // f=0, 2*0+1=1
}

#[test]
fn quorum_threshold_n2() {
    let p = ByzantineProposal::new("p".into(), "l".into(), "a".into(), "d".into(), 2);
    assert_eq!(p.quorum_threshold(), 1); // f=0, 2*0+1=1
}

#[test]
fn quorum_threshold_n3() {
    let p = ByzantineProposal::new("p".into(), "l".into(), "a".into(), "d".into(), 3);
    assert_eq!(p.quorum_threshold(), 1); // f=0, 2*0+1=1
}

#[test]
fn quorum_threshold_n4() {
    let p = ByzantineProposal::new("p".into(), "l".into(), "a".into(), "d".into(), 4);
    assert_eq!(p.quorum_threshold(), 3); // f=1, 2*1+1=3
}

#[test]
fn quorum_threshold_n7() {
    let p = ByzantineProposal::new("p".into(), "l".into(), "a".into(), "d".into(), 7);
    assert_eq!(p.quorum_threshold(), 5); // f=2, 2*2+1=5
}

#[test]
fn quorum_threshold_n10() {
    let p = ByzantineProposal::new("p".into(), "l".into(), "a".into(), "d".into(), 10);
    assert_eq!(p.quorum_threshold(), 7); // f=3, 2*3+1=7
}

// ─── Phase transition tests ───

#[test]
fn full_phase_transition_happy_path() {
    // 4 agents: quorum = 3
    let mut p = ByzantineProposal::new(
        "p1".into(),
        "leader".into(),
        "deploy".into(),
        "Deploy v2".into(),
        4,
    );
    assert_eq!(p.phase, ConsensusPhase::PrePrepare);

    // Prepare votes: 3 approvals reach quorum
    let r1 = p.add_prepare_vote(make_vote("a1", true, ConsensusPhase::Prepare));
    assert_eq!(r1, PhaseAdvanceResult::QuorumNotReached);
    assert_eq!(p.phase, ConsensusPhase::Prepare);

    let r2 = p.add_prepare_vote(make_vote("a2", true, ConsensusPhase::Prepare));
    assert_eq!(r2, PhaseAdvanceResult::QuorumNotReached);

    let r3 = p.add_prepare_vote(make_vote("a3", true, ConsensusPhase::Prepare));
    assert_eq!(r3, PhaseAdvanceResult::Advanced(ConsensusPhase::Commit));
    assert_eq!(p.phase, ConsensusPhase::Commit);

    // Commit votes: 3 approvals reach quorum
    let r4 = p.add_commit_vote(make_vote("a1", true, ConsensusPhase::Commit));
    assert_eq!(r4, PhaseAdvanceResult::QuorumNotReached);

    let r5 = p.add_commit_vote(make_vote("a2", true, ConsensusPhase::Commit));
    assert_eq!(r5, PhaseAdvanceResult::QuorumNotReached);

    let r6 = p.add_commit_vote(make_vote("a3", true, ConsensusPhase::Commit));
    assert_eq!(r6, PhaseAdvanceResult::Advanced(ConsensusPhase::Finalized));
    assert_eq!(p.phase, ConsensusPhase::Finalized);
    assert!(p.is_finalized());
    assert!(p.finalized_at.is_some());
}

#[test]
fn single_agent_reaches_quorum_immediately() {
    // N=1: quorum=1, a single approve suffices
    let mut p = ByzantineProposal::new(
        "p1".into(),
        "solo".into(),
        "act".into(),
        "desc".into(),
        1,
    );

    let r = p.add_prepare_vote(make_vote("solo", true, ConsensusPhase::Prepare));
    assert_eq!(r, PhaseAdvanceResult::Advanced(ConsensusPhase::Commit));

    let r = p.add_commit_vote(make_vote("solo", true, ConsensusPhase::Commit));
    assert_eq!(r, PhaseAdvanceResult::Advanced(ConsensusPhase::Finalized));
    assert!(p.is_finalized());
}

// ─── Insufficient votes / quorum not reached ───

#[test]
fn quorum_not_reached_with_insufficient_votes() {
    // 7 agents: quorum = 5
    let mut p = ByzantineProposal::new(
        "p1".into(),
        "leader".into(),
        "act".into(),
        "desc".into(),
        7,
    );

    // Only 4 approvals — not enough
    for i in 0..4 {
        let _ = p.add_prepare_vote(make_vote(&format!("a{}", i), true, ConsensusPhase::Prepare));
    }
    assert_eq!(p.phase, ConsensusPhase::Prepare);

    let r = p.try_advance();
    assert_eq!(r, PhaseAdvanceResult::QuorumNotReached);
}

// ─── Duplicate vote rejection ───

#[test]
fn duplicate_prepare_vote_rejected() {
    let mut p = ByzantineProposal::new(
        "p1".into(),
        "leader".into(),
        "act".into(),
        "desc".into(),
        4,
    );

    let r1 = p.add_prepare_vote(make_vote("a1", true, ConsensusPhase::Prepare));
    assert_eq!(r1, PhaseAdvanceResult::QuorumNotReached);

    let r2 = p.add_prepare_vote(make_vote("a1", true, ConsensusPhase::Prepare));
    match r2 {
        PhaseAdvanceResult::Failed(msg) => {
            assert!(msg.contains("already voted"));
        }
        other => panic!("Expected Failed, got {:?}", other),
    }

    // Only 1 vote should be recorded
    assert_eq!(p.prepare_votes.len(), 1);
}

#[test]
fn duplicate_commit_vote_rejected() {
    // Get to commit phase first (N=1 for simplicity)
    let mut p = ByzantineProposal::new(
        "p1".into(),
        "leader".into(),
        "act".into(),
        "desc".into(),
        4,
    );

    // Reach commit with 3 approvals
    for i in 0..3 {
        p.add_prepare_vote(make_vote(&format!("a{}", i), true, ConsensusPhase::Prepare));
    }
    assert_eq!(p.phase, ConsensusPhase::Commit);

    let r1 = p.add_commit_vote(make_vote("a1", true, ConsensusPhase::Commit));
    assert_eq!(r1, PhaseAdvanceResult::QuorumNotReached);

    let r2 = p.add_commit_vote(make_vote("a1", true, ConsensusPhase::Commit));
    match r2 {
        PhaseAdvanceResult::Failed(msg) => {
            assert!(msg.contains("already voted"));
        }
        other => panic!("Expected Failed, got {:?}", other),
    }

    assert_eq!(p.commit_votes.len(), 1);
}

// ─── Mixed votes ───

#[test]
fn mixed_votes_quorum_requires_approvals_only() {
    // 4 agents, quorum = 3
    let mut p = ByzantineProposal::new(
        "p1".into(),
        "leader".into(),
        "act".into(),
        "desc".into(),
        4,
    );

    // 2 approve, 1 reject — not enough approvals
    p.add_prepare_vote(make_vote("a1", true, ConsensusPhase::Prepare));
    p.add_prepare_vote(make_vote("a2", false, ConsensusPhase::Prepare));
    let r = p.add_prepare_vote(make_vote("a3", true, ConsensusPhase::Prepare));
    assert_eq!(r, PhaseAdvanceResult::QuorumNotReached);
    assert_eq!(p.phase, ConsensusPhase::Prepare);

    // Third approval reaches quorum
    let r = p.add_prepare_vote(make_vote("a4", true, ConsensusPhase::Prepare));
    assert_eq!(r, PhaseAdvanceResult::Advanced(ConsensusPhase::Commit));
}

// ─── Failed consensus from too many rejections ───

#[test]
fn too_many_rejections_cause_failure() {
    // 4 agents, quorum = 3, so max allowed rejections = 4 - 3 = 1
    // If 2 reject, it is impossible to reach quorum
    let mut p = ByzantineProposal::new(
        "p1".into(),
        "leader".into(),
        "act".into(),
        "desc".into(),
        4,
    );

    p.add_prepare_vote(make_vote("a1", false, ConsensusPhase::Prepare));
    let r = p.add_prepare_vote(make_vote("a2", false, ConsensusPhase::Prepare));
    assert_eq!(r, PhaseAdvanceResult::Advanced(ConsensusPhase::Failed));
    assert_eq!(p.phase, ConsensusPhase::Failed);
    assert!(p.is_finalized());
    assert!(p.finalized_at.is_some());
}

#[test]
fn commit_phase_failure_from_rejections() {
    // 4 agents, quorum = 3
    let mut p = ByzantineProposal::new(
        "p1".into(),
        "leader".into(),
        "act".into(),
        "desc".into(),
        4,
    );

    // Pass prepare
    for i in 0..3 {
        p.add_prepare_vote(make_vote(&format!("a{}", i), true, ConsensusPhase::Prepare));
    }
    assert_eq!(p.phase, ConsensusPhase::Commit);

    // 2 rejections in commit phase
    p.add_commit_vote(make_vote("a1", false, ConsensusPhase::Commit));
    let r = p.add_commit_vote(make_vote("a2", false, ConsensusPhase::Commit));
    assert_eq!(r, PhaseAdvanceResult::Advanced(ConsensusPhase::Failed));
    assert!(p.is_finalized());
}

// ─── Already finalized ───

#[test]
fn votes_on_finalized_proposal_return_already_finalized() {
    let mut p = ByzantineProposal::new(
        "p1".into(),
        "leader".into(),
        "act".into(),
        "desc".into(),
        1,
    );

    p.add_prepare_vote(make_vote("a1", true, ConsensusPhase::Prepare));
    p.add_commit_vote(make_vote("a1", true, ConsensusPhase::Commit));
    assert!(p.is_finalized());

    let r = p.add_prepare_vote(make_vote("a2", true, ConsensusPhase::Prepare));
    assert_eq!(r, PhaseAdvanceResult::AlreadyFinalized);

    let r = p.add_commit_vote(make_vote("a2", true, ConsensusPhase::Commit));
    assert_eq!(r, PhaseAdvanceResult::AlreadyFinalized);

    let r = p.try_advance();
    assert_eq!(r, PhaseAdvanceResult::AlreadyFinalized);
}

// ─── Wrong phase errors ───

#[test]
fn commit_vote_in_prepare_phase_fails() {
    let mut p = ByzantineProposal::new(
        "p1".into(),
        "leader".into(),
        "act".into(),
        "desc".into(),
        4,
    );

    // Still in PrePrepare, try commit vote
    let r = p.add_commit_vote(make_vote("a1", true, ConsensusPhase::Commit));
    match r {
        PhaseAdvanceResult::Failed(msg) => {
            assert!(msg.contains("Cannot add commit vote"));
        }
        other => panic!("Expected Failed, got {:?}", other),
    }
}

// ─── Serialization roundtrips ───

#[test]
fn consensus_phase_serialization_roundtrip() {
    for phase in &[
        ConsensusPhase::PrePrepare,
        ConsensusPhase::Prepare,
        ConsensusPhase::Commit,
        ConsensusPhase::Finalized,
        ConsensusPhase::Failed,
    ] {
        let json = serde_json::to_string(phase).unwrap();
        let decoded: ConsensusPhase = serde_json::from_str(&json).unwrap();
        assert_eq!(&decoded, phase);
    }
}

#[test]
fn byzantine_proposal_full_serialization_roundtrip() {
    let mut p = ByzantineProposal::new(
        "p1".into(),
        "leader".into(),
        "deploy".into(),
        "Deploy v2".into(),
        4,
    );
    p.add_prepare_vote(make_vote("a1", true, ConsensusPhase::Prepare));
    p.add_prepare_vote(make_vote("a2", false, ConsensusPhase::Prepare));

    let json = serde_json::to_string(&p).unwrap();
    let decoded: ByzantineProposal = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.id, "p1");
    assert_eq!(decoded.proposer, "leader");
    assert_eq!(decoded.total_agents, 4);
    assert_eq!(decoded.prepare_votes.len(), 2);
    assert_eq!(decoded.phase, ConsensusPhase::Prepare);
}

// ─── ConsensusPhase equality ───

#[test]
fn consensus_phase_equality() {
    assert_eq!(ConsensusPhase::PrePrepare, ConsensusPhase::PrePrepare);
    assert_ne!(ConsensusPhase::PrePrepare, ConsensusPhase::Prepare);
    assert_ne!(ConsensusPhase::Commit, ConsensusPhase::Finalized);
    assert_ne!(ConsensusPhase::Finalized, ConsensusPhase::Failed);
}

// ─── has_voted helpers ───

#[test]
fn has_voted_prepare_and_commit() {
    let mut p = ByzantineProposal::new(
        "p1".into(),
        "leader".into(),
        "act".into(),
        "desc".into(),
        4,
    );

    assert!(!p.has_voted_prepare("a1"));
    assert!(!p.has_voted_commit("a1"));

    p.add_prepare_vote(make_vote("a1", true, ConsensusPhase::Prepare));
    assert!(p.has_voted_prepare("a1"));
    assert!(!p.has_voted_prepare("a2"));
    assert!(!p.has_voted_commit("a1"));
}

// ─── try_advance on PrePrepare ───

#[test]
fn try_advance_on_pre_prepare_returns_quorum_not_reached() {
    let mut p = ByzantineProposal::new(
        "p1".into(),
        "leader".into(),
        "act".into(),
        "desc".into(),
        4,
    );
    let r = p.try_advance();
    assert_eq!(r, PhaseAdvanceResult::QuorumNotReached);
}

// ─── Channel message variant tests ───

#[test]
fn consensus_proposal_message_serialization() {
    let msg = CollaborationMessage::ConsensusProposal {
        proposal_id: "p1".into(),
        action: "deploy".into(),
        description: "Deploy v2".into(),
        proposer: "leader".into(),
        total_agents: 4,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: CollaborationMessage = serde_json::from_str(&json).unwrap();
    match decoded {
        CollaborationMessage::ConsensusProposal {
            proposal_id,
            total_agents,
            ..
        } => {
            assert_eq!(proposal_id, "p1");
            assert_eq!(total_agents, 4);
        }
        other => panic!("Expected ConsensusProposal, got {:?}", other),
    }
}

#[test]
fn prepare_vote_message_serialization() {
    let msg = CollaborationMessage::PrepareVote {
        proposal_id: "p1".into(),
        agent_id: "a1".into(),
        approve: true,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: CollaborationMessage = serde_json::from_str(&json).unwrap();
    match decoded {
        CollaborationMessage::PrepareVote {
            proposal_id,
            agent_id,
            approve,
        } => {
            assert_eq!(proposal_id, "p1");
            assert_eq!(agent_id, "a1");
            assert!(approve);
        }
        other => panic!("Expected PrepareVote, got {:?}", other),
    }
}

#[test]
fn commit_vote_message_serialization() {
    let msg = CollaborationMessage::CommitVote {
        proposal_id: "p1".into(),
        agent_id: "a2".into(),
        approve: false,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: CollaborationMessage = serde_json::from_str(&json).unwrap();
    match decoded {
        CollaborationMessage::CommitVote {
            proposal_id,
            agent_id,
            approve,
        } => {
            assert_eq!(proposal_id, "p1");
            assert_eq!(agent_id, "a2");
            assert!(!approve);
        }
        other => panic!("Expected CommitVote, got {:?}", other),
    }
}

#[test]
fn consensus_result_message_serialization() {
    let msg = CollaborationMessage::ConsensusResult {
        proposal_id: "p1".into(),
        finalized: true,
        phase: "Finalized".into(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: CollaborationMessage = serde_json::from_str(&json).unwrap();
    match decoded {
        CollaborationMessage::ConsensusResult {
            proposal_id,
            finalized,
            phase,
        } => {
            assert_eq!(proposal_id, "p1");
            assert!(finalized);
            assert_eq!(phase, "Finalized");
        }
        other => panic!("Expected ConsensusResult, got {:?}", other),
    }
}

// ─── Integration with CollaborationContext ───

#[test]
fn byzantine_proposal_alongside_regular_proposals() {
    use octo_engine::agent::collaboration::{CollaborationContext, Proposal, ProposalStatus, Vote};

    let ctx = CollaborationContext::new("test-collab".to_string());

    // Add a regular proposal
    let regular = Proposal {
        id: "regular-1".into(),
        from_agent: "a1".into(),
        action: "refactor".into(),
        description: "Refactor module".into(),
        status: ProposalStatus::Pending,
        votes: vec![Vote {
            agent_id: "a2".into(),
            approve: true,
            reason: None,
        }],
    };
    ctx.add_proposal(regular);

    // Store a ByzantineProposal as JSON in shared state
    let mut bp = ByzantineProposal::new(
        "byz-1".into(),
        "leader".into(),
        "deploy".into(),
        "Deploy v2".into(),
        4,
    );
    bp.add_prepare_vote(make_vote("a1", true, ConsensusPhase::Prepare));

    let bp_json = serde_json::to_value(&bp).unwrap();
    ctx.set_state("byzantine:byz-1".into(), bp_json.clone());

    // Verify both coexist
    assert_eq!(ctx.proposals().len(), 1);
    assert_eq!(ctx.proposals()[0].id, "regular-1");

    let stored = ctx.get_state("byzantine:byz-1").unwrap();
    let restored: ByzantineProposal = serde_json::from_value(stored).unwrap();
    assert_eq!(restored.id, "byz-1");
    assert_eq!(restored.prepare_votes.len(), 1);
}
