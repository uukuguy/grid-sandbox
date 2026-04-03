pub mod channel;
pub mod consensus;
pub mod context;
pub mod crypto;
pub mod handle;
pub mod injection;
pub mod manager;
pub mod persistence;
pub mod protocol;
pub mod sqlite_store;

pub use channel::{create_channel_pair, CollaborationChannel, CollaborationMessage};
pub use consensus::{
    ByzantineProposal, ConsensusPhase, ConsensusVote, PhaseAdvanceResult, ViewChangeReason,
    ViewChangeRequest, ViewChangeTracker, ViewState,
};
pub use crypto::{
    sign_consensus_vote, verify_consensus_vote, verify_signature, ConsensusKeypair, SignedMessage,
    VerifyResult,
};
pub use context::*;
pub use handle::CollaborationHandle;
pub use injection::build_collaboration_injection;
pub use manager::{CollaborationAgent, CollaborationManager};
pub use persistence::{
    ByzantineStore, CollaborationSnapshot, CollaborationStore, InMemoryCollaborationStore,
    SignatureRecord,
};
pub use protocol::CollaborationProtocol;
pub use sqlite_store::SqliteByzantineStore;
