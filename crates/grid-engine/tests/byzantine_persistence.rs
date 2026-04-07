//! Tests for Byzantine consensus persistence (D1-P3).

use grid_engine::agent::collaboration::{
    ByzantineProposal, ByzantineStore, ConsensusKeypair, ConsensusPhase, ConsensusVote,
    SqliteByzantineStore, ViewChangeReason, ViewChangeRequest, ViewChangeTracker,
    sign_consensus_vote,
};
use grid_engine::db::Database;
use grid_types::SessionId;

async fn setup_store() -> SqliteByzantineStore {
    let db = Database::open_in_memory().await.unwrap();
    SqliteByzantineStore::new(db.conn().clone())
}

fn make_proposal(id: &str, total: usize) -> ByzantineProposal {
    ByzantineProposal::new(
        id.to_string(),
        "leader".to_string(),
        "deploy".to_string(),
        "Deploy v2".to_string(),
        total,
    )
}

fn make_vote(agent: &str, approve: bool, phase: ConsensusPhase) -> ConsensusVote {
    ConsensusVote {
        agent_id: agent.to_string(),
        approve,
        phase,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

fn make_session() -> SessionId {
    SessionId::new()
}

fn test_encryption_key() -> [u8; 32] {
    [42u8; 32]
}

#[tokio::test]
async fn test_save_load_proposal() {
    let store = setup_store().await;
    let sid = make_session();
    let proposal = make_proposal("p1", 4);

    store.save_proposal(&sid, "collab-1", &proposal).await.unwrap();

    let loaded = store.load_proposal(&sid, "p1").await.unwrap();
    assert!(loaded.is_some());
    let p = loaded.unwrap();
    assert_eq!(p.id, "p1");
    assert_eq!(p.proposer, "leader");
    assert_eq!(p.action, "deploy");
    assert_eq!(p.description, "Deploy v2");
    assert_eq!(p.phase, ConsensusPhase::PrePrepare);
    assert_eq!(p.total_agents, 4);
    assert!(p.prepare_votes.is_empty());
    assert!(p.commit_votes.is_empty());
    assert!(p.finalized_at.is_none());
}

#[tokio::test]
async fn test_proposal_with_votes() {
    let store = setup_store().await;
    let sid = make_session();
    let mut proposal = make_proposal("p2", 4);
    proposal.prepare_votes.push(make_vote("a1", true, ConsensusPhase::Prepare));
    proposal.prepare_votes.push(make_vote("a2", false, ConsensusPhase::Prepare));
    proposal.commit_votes.push(make_vote("a3", true, ConsensusPhase::Commit));

    store.save_proposal(&sid, "collab-1", &proposal).await.unwrap();

    let loaded = store.load_proposal(&sid, "p2").await.unwrap().unwrap();
    assert_eq!(loaded.prepare_votes.len(), 2);
    assert_eq!(loaded.prepare_votes[0].agent_id, "a1");
    assert!(loaded.prepare_votes[0].approve);
    assert!(!loaded.prepare_votes[1].approve);
    assert_eq!(loaded.commit_votes.len(), 1);
    assert_eq!(loaded.commit_votes[0].agent_id, "a3");
}

#[tokio::test]
async fn test_list_proposals_by_phase() {
    let store = setup_store().await;
    let sid = make_session();

    let mut p1 = make_proposal("p1", 4);
    p1.phase = ConsensusPhase::Prepare;
    let mut p2 = make_proposal("p2", 4);
    p2.phase = ConsensusPhase::Commit;
    let mut p3 = make_proposal("p3", 4);
    p3.phase = ConsensusPhase::Prepare;

    store.save_proposal(&sid, "collab-1", &p1).await.unwrap();
    store.save_proposal(&sid, "collab-1", &p2).await.unwrap();
    store.save_proposal(&sid, "collab-1", &p3).await.unwrap();

    // All proposals
    let all = store.list_proposals(&sid, "collab-1", None).await.unwrap();
    assert_eq!(all.len(), 3);

    // Only Prepare
    let prepare = store
        .list_proposals(&sid, "collab-1", Some(ConsensusPhase::Prepare))
        .await
        .unwrap();
    assert_eq!(prepare.len(), 2);
    assert!(prepare.iter().all(|p| p.phase == ConsensusPhase::Prepare));

    // Only Commit
    let commit = store
        .list_proposals(&sid, "collab-1", Some(ConsensusPhase::Commit))
        .await
        .unwrap();
    assert_eq!(commit.len(), 1);
    assert_eq!(commit[0].id, "p2");
}

#[tokio::test]
async fn test_update_proposal_phase() {
    let store = setup_store().await;
    let sid = make_session();
    let mut proposal = make_proposal("p4", 4);

    store.save_proposal(&sid, "collab-1", &proposal).await.unwrap();

    // Update to Prepare
    proposal.phase = ConsensusPhase::Prepare;
    proposal.prepare_votes.push(make_vote("a1", true, ConsensusPhase::Prepare));
    store.update_proposal(&sid, &proposal).await.unwrap();

    let loaded = store.load_proposal(&sid, "p4").await.unwrap().unwrap();
    assert_eq!(loaded.phase, ConsensusPhase::Prepare);
    assert_eq!(loaded.prepare_votes.len(), 1);
}

#[tokio::test]
async fn test_delete_proposals() {
    let store = setup_store().await;
    let sid = make_session();

    store.save_proposal(&sid, "collab-1", &make_proposal("p1", 4)).await.unwrap();
    store.save_proposal(&sid, "collab-1", &make_proposal("p2", 4)).await.unwrap();
    store.save_proposal(&sid, "collab-2", &make_proposal("p3", 4)).await.unwrap();

    let deleted = store.delete_proposals(&sid, "collab-1").await.unwrap();
    assert_eq!(deleted, 2);

    let remaining = store.list_proposals(&sid, "collab-1", None).await.unwrap();
    assert!(remaining.is_empty());

    // collab-2 untouched
    let other = store.list_proposals(&sid, "collab-2", None).await.unwrap();
    assert_eq!(other.len(), 1);
}

#[tokio::test]
async fn test_save_load_view_state() {
    let store = setup_store().await;
    let sid = make_session();
    let agents = vec!["a1".to_string(), "a2".to_string(), "a3".to_string()];
    let tracker = ViewChangeTracker::new(agents.clone(), 5000);

    store.save_view_state(&sid, "collab-1", &tracker).await.unwrap();

    let loaded = store.load_view_state(&sid, "collab-1").await.unwrap();
    assert!(loaded.is_some());
    let t = loaded.unwrap();
    assert_eq!(t.state.view_number, 0);
    assert_eq!(t.state.leader, "a1");
    assert_eq!(t.state.agents, agents);
    assert_eq!(t.timeout_ms, 5000);
    assert!(t.requests.is_empty());
}

#[tokio::test]
async fn test_view_state_with_requests() {
    let store = setup_store().await;
    let sid = make_session();
    let agents = vec!["a1".to_string(), "a2".to_string(), "a3".to_string()];
    let mut tracker = ViewChangeTracker::new(agents, 3000);
    tracker.request_view_change(ViewChangeRequest {
        from_agent: "a2".to_string(),
        current_view: 0,
        proposed_view: 1,
        reason: ViewChangeReason::LeaderTimeout,
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    store.save_view_state(&sid, "collab-1", &tracker).await.unwrap();

    let loaded = store.load_view_state(&sid, "collab-1").await.unwrap().unwrap();
    assert_eq!(loaded.requests.len(), 1);
    assert_eq!(loaded.requests[0].from_agent, "a2");
    assert_eq!(loaded.requests[0].reason, ViewChangeReason::LeaderTimeout);
    assert_eq!(loaded.timeout_ms, 3000);
}

#[tokio::test]
async fn test_log_and_get_signatures() {
    let store = setup_store().await;
    let sid = make_session();
    let kp = ConsensusKeypair::generate("voter1".to_string());
    let signed = sign_consensus_vote(&kp, "prop-1", "Prepare", true);

    store
        .log_signature(&sid, "prop-1", &signed, "Prepare", true)
        .await
        .unwrap();

    let sigs = store.get_signatures(&sid, "prop-1").await.unwrap();
    assert_eq!(sigs.len(), 1);
    assert_eq!(sigs[0].agent_id, "voter1");
    assert_eq!(sigs[0].phase, "Prepare");
    assert!(sigs[0].approve);
    assert!(!sigs[0].signature.is_empty());
    assert!(!sigs[0].public_key.is_empty());
}

#[tokio::test]
async fn test_signature_ordering() {
    let store = setup_store().await;
    let sid = make_session();
    let kp1 = ConsensusKeypair::generate("voter1".to_string());
    let kp2 = ConsensusKeypair::generate("voter2".to_string());
    let kp3 = ConsensusKeypair::generate("voter3".to_string());

    let s1 = sign_consensus_vote(&kp1, "prop-1", "Prepare", true);
    let s2 = sign_consensus_vote(&kp2, "prop-1", "Prepare", false);
    let s3 = sign_consensus_vote(&kp3, "prop-1", "Commit", true);

    store.log_signature(&sid, "prop-1", &s1, "Prepare", true).await.unwrap();
    store.log_signature(&sid, "prop-1", &s2, "Prepare", false).await.unwrap();
    store.log_signature(&sid, "prop-1", &s3, "Commit", true).await.unwrap();

    let sigs = store.get_signatures(&sid, "prop-1").await.unwrap();
    assert_eq!(sigs.len(), 3);
    // Ordered by insertion (id ASC)
    assert_eq!(sigs[0].agent_id, "voter1");
    assert_eq!(sigs[1].agent_id, "voter2");
    assert_eq!(sigs[2].agent_id, "voter3");
    assert!(sigs[0].id < sigs[1].id);
    assert!(sigs[1].id < sigs[2].id);
}

#[tokio::test]
async fn test_keypair_encrypt_decrypt() {
    let store = setup_store().await;
    let sid = make_session();
    let key = test_encryption_key();
    let kp = ConsensusKeypair::generate("agent-x".to_string());
    let original_pub = kp.public_key_bytes();

    store.save_keypair(&sid, "agent-x", &kp, &key).await.unwrap();

    let loaded = store.load_keypair(&sid, "agent-x", &key).await.unwrap();
    assert!(loaded.is_some());
    let restored = loaded.unwrap();
    assert_eq!(restored.agent_id, "agent-x");
    assert_eq!(restored.public_key_bytes(), original_pub);

    // Verify signing still works after restore
    let signed = restored.sign(b"test message");
    assert_eq!(
        grid_engine::agent::collaboration::verify_signature(&signed),
        grid_engine::agent::collaboration::VerifyResult::Valid,
    );
}

#[tokio::test]
async fn test_keypair_wrong_key() {
    let store = setup_store().await;
    let sid = make_session();
    let key = test_encryption_key();
    let wrong_key = [99u8; 32];
    let kp = ConsensusKeypair::generate("agent-y".to_string());

    store.save_keypair(&sid, "agent-y", &kp, &key).await.unwrap();

    let result = store.load_keypair(&sid, "agent-y", &wrong_key).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_proposal_not_found() {
    let store = setup_store().await;
    let sid = make_session();

    let loaded = store.load_proposal(&sid, "nonexistent").await.unwrap();
    assert!(loaded.is_none());
}

#[tokio::test]
async fn test_view_state_not_found() {
    let store = setup_store().await;
    let sid = make_session();

    let loaded = store.load_view_state(&sid, "nonexistent").await.unwrap();
    assert!(loaded.is_none());
}

#[tokio::test]
async fn test_keypair_not_found() {
    let store = setup_store().await;
    let sid = make_session();
    let key = test_encryption_key();

    let loaded = store.load_keypair(&sid, "nobody", &key).await.unwrap();
    assert!(loaded.is_none());
}

#[tokio::test]
async fn test_full_consensus_lifecycle() {
    let store = setup_store().await;
    let sid = make_session();
    let key = test_encryption_key();

    // 1. Create keypairs for agents
    let kp_leader = ConsensusKeypair::generate("leader".to_string());
    let kp_a1 = ConsensusKeypair::generate("a1".to_string());
    let kp_a2 = ConsensusKeypair::generate("a2".to_string());
    let kp_a3 = ConsensusKeypair::generate("a3".to_string());

    store.save_keypair(&sid, "leader", &kp_leader, &key).await.unwrap();
    store.save_keypair(&sid, "a1", &kp_a1, &key).await.unwrap();
    store.save_keypair(&sid, "a2", &kp_a2, &key).await.unwrap();
    store.save_keypair(&sid, "a3", &kp_a3, &key).await.unwrap();

    // 2. Create proposal
    let mut proposal = make_proposal("lifecycle-1", 4);
    store.save_proposal(&sid, "collab-1", &proposal).await.unwrap();

    // 3. Prepare votes
    proposal.add_prepare_vote(make_vote("a1", true, ConsensusPhase::Prepare));
    proposal.add_prepare_vote(make_vote("a2", true, ConsensusPhase::Prepare));
    proposal.add_prepare_vote(make_vote("a3", true, ConsensusPhase::Prepare));
    // After 3 approvals with total_agents=4, quorum=3, should advance to Commit
    assert_eq!(proposal.phase, ConsensusPhase::Commit);

    // Log signed votes
    let sv1 = sign_consensus_vote(&kp_a1, "lifecycle-1", "Prepare", true);
    let sv2 = sign_consensus_vote(&kp_a2, "lifecycle-1", "Prepare", true);
    let sv3 = sign_consensus_vote(&kp_a3, "lifecycle-1", "Prepare", true);
    store.log_signature(&sid, "lifecycle-1", &sv1, "Prepare", true).await.unwrap();
    store.log_signature(&sid, "lifecycle-1", &sv2, "Prepare", true).await.unwrap();
    store.log_signature(&sid, "lifecycle-1", &sv3, "Prepare", true).await.unwrap();

    // 4. Persist updated proposal
    store.update_proposal(&sid, &proposal).await.unwrap();

    // 5. Commit votes
    proposal.add_commit_vote(make_vote("a1", true, ConsensusPhase::Commit));
    proposal.add_commit_vote(make_vote("a2", true, ConsensusPhase::Commit));
    proposal.add_commit_vote(make_vote("a3", true, ConsensusPhase::Commit));
    assert_eq!(proposal.phase, ConsensusPhase::Finalized);
    assert!(proposal.finalized_at.is_some());

    store.update_proposal(&sid, &proposal).await.unwrap();

    // 6. Verify full state from DB
    let loaded = store.load_proposal(&sid, "lifecycle-1").await.unwrap().unwrap();
    assert_eq!(loaded.phase, ConsensusPhase::Finalized);
    assert_eq!(loaded.prepare_votes.len(), 3);
    assert_eq!(loaded.commit_votes.len(), 3);
    assert!(loaded.finalized_at.is_some());

    // 7. Verify signatures audit log
    let sigs = store.get_signatures(&sid, "lifecycle-1").await.unwrap();
    assert_eq!(sigs.len(), 3);

    // 8. Verify keypairs can be restored
    let restored = store.load_keypair(&sid, "a1", &key).await.unwrap().unwrap();
    assert_eq!(restored.agent_id, "a1");

    // 9. Save view state
    let agents = vec![
        "leader".to_string(),
        "a1".to_string(),
        "a2".to_string(),
        "a3".to_string(),
    ];
    let tracker = ViewChangeTracker::new(agents, 5000);
    store.save_view_state(&sid, "collab-1", &tracker).await.unwrap();

    let vs = store.load_view_state(&sid, "collab-1").await.unwrap().unwrap();
    assert_eq!(vs.state.leader, "leader");
    assert_eq!(vs.state.agents.len(), 4);
}
