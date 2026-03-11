//! PBFT-lite Byzantine consensus module.
//!
//! Implements a simplified Practical Byzantine Fault Tolerance protocol
//! with three phases: PrePrepare, Prepare, and Commit. This is Phase 1
//! (basic consensus) without cryptographic signing.
//!
//! Quorum formula: for N total agents, f = floor((N-1)/3), quorum = 2f + 1.

use serde::{Deserialize, Serialize};

/// Consensus phases following simplified PBFT.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusPhase {
    /// Leader proposes an action.
    PrePrepare,
    /// Replicas acknowledge the proposal.
    Prepare,
    /// Replicas confirm they are ready to commit.
    Commit,
    /// Consensus reached successfully.
    Finalized,
    /// Consensus failed (timeout or insufficient votes).
    Failed,
}

impl std::fmt::Display for ConsensusPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PrePrepare => write!(f, "PrePrepare"),
            Self::Prepare => write!(f, "Prepare"),
            Self::Commit => write!(f, "Commit"),
            Self::Finalized => write!(f, "Finalized"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// Result of attempting to advance the consensus phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhaseAdvanceResult {
    /// Successfully moved to next phase.
    Advanced(ConsensusPhase),
    /// Not enough votes yet.
    QuorumNotReached,
    /// Proposal already done.
    AlreadyFinalized,
    /// Error description.
    Failed(String),
}

/// A vote in the consensus protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusVote {
    /// ID of the voting agent.
    pub agent_id: String,
    /// Whether this agent approves.
    pub approve: bool,
    /// Which phase this vote is for.
    pub phase: ConsensusPhase,
    /// ISO 8601 timestamp.
    pub timestamp: String,
}

/// A Byzantine consensus proposal with quorum tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByzantineProposal {
    /// Unique proposal identifier.
    pub id: String,
    /// Current phase of this proposal.
    pub phase: ConsensusPhase,
    /// The leader/proposer agent ID.
    pub proposer: String,
    /// Action being proposed.
    pub action: String,
    /// Human-readable description.
    pub description: String,
    /// Votes received during the Prepare phase.
    pub prepare_votes: Vec<ConsensusVote>,
    /// Votes received during the Commit phase.
    pub commit_votes: Vec<ConsensusVote>,
    /// Total number of agents in the quorum calculation.
    pub total_agents: usize,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// ISO 8601 finalization timestamp (set when Finalized or Failed).
    pub finalized_at: Option<String>,
}

impl ByzantineProposal {
    /// Creates a new proposal in the PrePrepare phase.
    pub fn new(
        id: String,
        proposer: String,
        action: String,
        description: String,
        total_agents: usize,
    ) -> Self {
        Self {
            id,
            phase: ConsensusPhase::PrePrepare,
            proposer,
            action,
            description,
            prepare_votes: Vec::new(),
            commit_votes: Vec::new(),
            total_agents,
            created_at: chrono::Utc::now().to_rfc3339(),
            finalized_at: None,
        }
    }

    /// Returns the quorum threshold: 2f + 1 where f = floor((N-1) / 3).
    pub fn quorum_threshold(&self) -> usize {
        let f = (self.total_agents.saturating_sub(1)) / 3;
        2 * f + 1
    }

    /// Returns whether this proposal has been finalized (successfully or failed).
    pub fn is_finalized(&self) -> bool {
        self.phase == ConsensusPhase::Finalized || self.phase == ConsensusPhase::Failed
    }

    /// Returns whether the given agent has already voted in the Prepare phase.
    pub fn has_voted_prepare(&self, agent_id: &str) -> bool {
        self.prepare_votes.iter().any(|v| v.agent_id == agent_id)
    }

    /// Returns whether the given agent has already voted in the Commit phase.
    pub fn has_voted_commit(&self, agent_id: &str) -> bool {
        self.commit_votes.iter().any(|v| v.agent_id == agent_id)
    }

    /// Adds a Prepare phase vote and auto-advances to Commit if quorum is reached.
    ///
    /// Returns `AlreadyFinalized` if the proposal is done, `Failed` if the
    /// proposal is not in Prepare phase or the agent already voted, and
    /// `Advanced(Commit)` or `QuorumNotReached` otherwise.
    pub fn add_prepare_vote(&mut self, vote: ConsensusVote) -> PhaseAdvanceResult {
        if self.is_finalized() {
            return PhaseAdvanceResult::AlreadyFinalized;
        }

        // Auto-advance from PrePrepare to Prepare on first vote
        if self.phase == ConsensusPhase::PrePrepare {
            self.phase = ConsensusPhase::Prepare;
        }

        if self.phase != ConsensusPhase::Prepare {
            return PhaseAdvanceResult::Failed(format!(
                "Cannot add prepare vote in {:?} phase",
                self.phase
            ));
        }

        if self.has_voted_prepare(&vote.agent_id) {
            return PhaseAdvanceResult::Failed(format!(
                "Agent {} already voted in Prepare phase",
                vote.agent_id
            ));
        }

        self.prepare_votes.push(vote);
        self.try_advance_prepare()
    }

    /// Adds a Commit phase vote and auto-advances to Finalized if quorum is reached.
    ///
    /// Returns `AlreadyFinalized` if the proposal is done, `Failed` if the
    /// proposal is not in Commit phase or the agent already voted, and
    /// `Advanced(Finalized)` or `QuorumNotReached` otherwise.
    pub fn add_commit_vote(&mut self, vote: ConsensusVote) -> PhaseAdvanceResult {
        if self.is_finalized() {
            return PhaseAdvanceResult::AlreadyFinalized;
        }

        if self.phase != ConsensusPhase::Commit {
            return PhaseAdvanceResult::Failed(format!(
                "Cannot add commit vote in {:?} phase",
                self.phase
            ));
        }

        if self.has_voted_commit(&vote.agent_id) {
            return PhaseAdvanceResult::Failed(format!(
                "Agent {} already voted in Commit phase",
                vote.agent_id
            ));
        }

        self.commit_votes.push(vote);
        self.try_advance_commit()
    }

    /// Checks if the current phase can advance based on accumulated votes.
    pub fn try_advance(&mut self) -> PhaseAdvanceResult {
        if self.is_finalized() {
            return PhaseAdvanceResult::AlreadyFinalized;
        }

        match self.phase {
            ConsensusPhase::PrePrepare => PhaseAdvanceResult::QuorumNotReached,
            ConsensusPhase::Prepare => self.try_advance_prepare(),
            ConsensusPhase::Commit => self.try_advance_commit(),
            ConsensusPhase::Finalized | ConsensusPhase::Failed => {
                PhaseAdvanceResult::AlreadyFinalized
            }
        }
    }

    /// Count approvals in prepare votes and try to advance to Commit.
    fn try_advance_prepare(&mut self) -> PhaseAdvanceResult {
        let approvals = self.prepare_votes.iter().filter(|v| v.approve).count();
        let rejections = self.prepare_votes.iter().filter(|v| !v.approve).count();
        let quorum = self.quorum_threshold();

        if approvals >= quorum {
            self.phase = ConsensusPhase::Commit;
            PhaseAdvanceResult::Advanced(ConsensusPhase::Commit)
        } else if rejections > self.total_agents - quorum {
            // Too many rejections — impossible to reach quorum
            self.phase = ConsensusPhase::Failed;
            self.finalized_at = Some(chrono::Utc::now().to_rfc3339());
            PhaseAdvanceResult::Advanced(ConsensusPhase::Failed)
        } else {
            PhaseAdvanceResult::QuorumNotReached
        }
    }

    /// Count approvals in commit votes and try to advance to Finalized.
    fn try_advance_commit(&mut self) -> PhaseAdvanceResult {
        let approvals = self.commit_votes.iter().filter(|v| v.approve).count();
        let rejections = self.commit_votes.iter().filter(|v| !v.approve).count();
        let quorum = self.quorum_threshold();

        if approvals >= quorum {
            self.phase = ConsensusPhase::Finalized;
            self.finalized_at = Some(chrono::Utc::now().to_rfc3339());
            PhaseAdvanceResult::Advanced(ConsensusPhase::Finalized)
        } else if rejections > self.total_agents - quorum {
            self.phase = ConsensusPhase::Failed;
            self.finalized_at = Some(chrono::Utc::now().to_rfc3339());
            PhaseAdvanceResult::Advanced(ConsensusPhase::Failed)
        } else {
            PhaseAdvanceResult::QuorumNotReached
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_vote(agent_id: &str, approve: bool, phase: ConsensusPhase) -> ConsensusVote {
        ConsensusVote {
            agent_id: agent_id.to_string(),
            approve,
            phase,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn new_proposal_starts_in_pre_prepare() {
        let p = ByzantineProposal::new(
            "p1".into(),
            "leader".into(),
            "deploy".into(),
            "Deploy v2".into(),
            4,
        );
        assert_eq!(p.phase, ConsensusPhase::PrePrepare);
        assert!(!p.is_finalized());
        assert!(p.prepare_votes.is_empty());
        assert!(p.commit_votes.is_empty());
        assert!(p.finalized_at.is_none());
    }

    #[test]
    fn quorum_threshold_n1() {
        let p = ByzantineProposal::new("p".into(), "l".into(), "a".into(), "d".into(), 1);
        assert_eq!(p.quorum_threshold(), 1);
    }

    #[test]
    fn quorum_threshold_n2() {
        let p = ByzantineProposal::new("p".into(), "l".into(), "a".into(), "d".into(), 2);
        assert_eq!(p.quorum_threshold(), 1);
    }

    #[test]
    fn quorum_threshold_n3() {
        let p = ByzantineProposal::new("p".into(), "l".into(), "a".into(), "d".into(), 3);
        assert_eq!(p.quorum_threshold(), 1);
    }

    #[test]
    fn quorum_threshold_n4() {
        let p = ByzantineProposal::new("p".into(), "l".into(), "a".into(), "d".into(), 4);
        assert_eq!(p.quorum_threshold(), 3);
    }

    #[test]
    fn quorum_threshold_n7() {
        let p = ByzantineProposal::new("p".into(), "l".into(), "a".into(), "d".into(), 7);
        assert_eq!(p.quorum_threshold(), 5);
    }

    #[test]
    fn quorum_threshold_n10() {
        let p = ByzantineProposal::new("p".into(), "l".into(), "a".into(), "d".into(), 10);
        assert_eq!(p.quorum_threshold(), 7);
    }

    #[test]
    fn phase_display() {
        assert_eq!(ConsensusPhase::PrePrepare.to_string(), "PrePrepare");
        assert_eq!(ConsensusPhase::Finalized.to_string(), "Finalized");
        assert_eq!(ConsensusPhase::Failed.to_string(), "Failed");
    }

    #[test]
    fn consensus_vote_serialization_roundtrip() {
        let vote = make_vote("agent-1", true, ConsensusPhase::Prepare);
        let json = serde_json::to_string(&vote).unwrap();
        let decoded: ConsensusVote = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.agent_id, "agent-1");
        assert!(decoded.approve);
        assert_eq!(decoded.phase, ConsensusPhase::Prepare);
    }

    #[test]
    fn byzantine_proposal_serialization_roundtrip() {
        let mut p = ByzantineProposal::new(
            "p1".into(),
            "leader".into(),
            "deploy".into(),
            "Deploy v2".into(),
            4,
        );
        p.prepare_votes
            .push(make_vote("a1", true, ConsensusPhase::Prepare));
        let json = serde_json::to_string(&p).unwrap();
        let decoded: ByzantineProposal = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, "p1");
        assert_eq!(decoded.proposer, "leader");
        assert_eq!(decoded.prepare_votes.len(), 1);
    }
}
