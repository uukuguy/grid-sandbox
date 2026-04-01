//! TeamManager — multi-agent team coordination.
//!
//! Manages named teams of agent sessions with leader/worker roles.
//! Used by team_create / team_add_member / team_dissolve LLM tools.

use std::collections::HashMap;
use std::time::Instant;

use dashmap::DashMap;
use octo_types::SessionId;
use serde::{Deserialize, Serialize};

/// Role of a team member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamRole {
    Leader,
    Worker,
}

impl std::fmt::Display for TeamRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Leader => write!(f, "leader"),
            Self::Worker => write!(f, "worker"),
        }
    }
}

/// A member of a team.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub name: String,
    pub session_id: SessionId,
    pub agent_type: Option<String>,
    pub role: TeamRole,
    #[serde(skip)]
    pub joined_at: Option<Instant>,
}

/// A named team of agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub name: String,
    pub description: Option<String>,
    pub leader_session_id: SessionId,
    pub members: HashMap<String, TeamMember>,
    #[serde(skip)]
    pub created_at: Option<Instant>,
}

/// In-memory team manager.
pub struct TeamManager {
    teams: DashMap<String, Team>,
}

impl TeamManager {
    pub fn new() -> Self {
        Self {
            teams: DashMap::new(),
        }
    }

    /// Create a new team with the given leader session.
    pub fn create_team(
        &self,
        name: &str,
        leader_session_id: SessionId,
        description: Option<&str>,
    ) -> Result<Team, String> {
        if self.teams.contains_key(name) {
            return Err(format!("Team '{}' already exists", name));
        }
        let mut members = HashMap::new();
        members.insert(
            "leader".to_string(),
            TeamMember {
                name: "leader".to_string(),
                session_id: leader_session_id.clone(),
                agent_type: None,
                role: TeamRole::Leader,
                joined_at: Some(Instant::now()),
            },
        );
        let team = Team {
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            leader_session_id,
            members,
            created_at: Some(Instant::now()),
        };
        self.teams.insert(name.to_string(), team.clone());
        Ok(team)
    }

    /// Add a member to an existing team.
    pub fn add_member(
        &self,
        team_name: &str,
        member_name: &str,
        session_id: SessionId,
        agent_type: Option<&str>,
    ) -> Result<(), String> {
        let mut entry = self
            .teams
            .get_mut(team_name)
            .ok_or_else(|| format!("Team '{}' not found", team_name))?;
        if entry.members.contains_key(member_name) {
            return Err(format!(
                "Member '{}' already exists in team '{}'",
                member_name, team_name
            ));
        }
        entry.members.insert(
            member_name.to_string(),
            TeamMember {
                name: member_name.to_string(),
                session_id,
                agent_type: agent_type.map(|s| s.to_string()),
                role: TeamRole::Worker,
                joined_at: Some(Instant::now()),
            },
        );
        Ok(())
    }

    /// Dissolve a team and return all member session IDs (for cleanup).
    pub fn dissolve_team(&self, team_name: &str) -> Result<Vec<SessionId>, String> {
        let (_, team) = self
            .teams
            .remove(team_name)
            .ok_or_else(|| format!("Team '{}' not found", team_name))?;
        Ok(team
            .members
            .values()
            .map(|m| m.session_id.clone())
            .collect())
    }

    /// Find a member's session ID by name within a team.
    pub fn find_member(&self, team_name: &str, member_name: &str) -> Option<SessionId> {
        self.teams
            .get(team_name)
            .and_then(|team| team.members.get(member_name).map(|m| m.session_id.clone()))
    }

    /// List all teams.
    pub fn list_teams(&self) -> Vec<Team> {
        self.teams.iter().map(|e| e.value().clone()).collect()
    }

    /// Get a specific team by name.
    pub fn get_team(&self, name: &str) -> Option<Team> {
        self.teams.get(name).map(|e| e.value().clone())
    }
}

impl Default for TeamManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(s: &str) -> SessionId {
        SessionId::from_string(s)
    }

    #[test]
    fn test_create_team() {
        let mgr = TeamManager::new();
        let team = mgr.create_team("alpha", sid("s-1"), Some("Alpha team")).unwrap();
        assert_eq!(team.name, "alpha");
        assert_eq!(team.members.len(), 1);
        assert_eq!(team.members["leader"].role, TeamRole::Leader);
    }

    #[test]
    fn test_create_duplicate_team() {
        let mgr = TeamManager::new();
        mgr.create_team("alpha", sid("s-1"), None).unwrap();
        assert!(mgr.create_team("alpha", sid("s-2"), None).is_err());
    }

    #[test]
    fn test_add_member() {
        let mgr = TeamManager::new();
        mgr.create_team("alpha", sid("s-1"), None).unwrap();
        mgr.add_member("alpha", "coder", sid("s-2"), Some("coder"))
            .unwrap();
        let team = mgr.get_team("alpha").unwrap();
        assert_eq!(team.members.len(), 2);
        assert_eq!(team.members["coder"].role, TeamRole::Worker);
    }

    #[test]
    fn test_add_member_duplicate() {
        let mgr = TeamManager::new();
        mgr.create_team("alpha", sid("s-1"), None).unwrap();
        mgr.add_member("alpha", "coder", sid("s-2"), None).unwrap();
        assert!(mgr.add_member("alpha", "coder", sid("s-3"), None).is_err());
    }

    #[test]
    fn test_dissolve_team() {
        let mgr = TeamManager::new();
        mgr.create_team("alpha", sid("s-1"), None).unwrap();
        mgr.add_member("alpha", "coder", sid("s-2"), None).unwrap();
        let sessions = mgr.dissolve_team("alpha").unwrap();
        assert_eq!(sessions.len(), 2);
        assert!(mgr.get_team("alpha").is_none());
    }

    #[test]
    fn test_dissolve_not_found() {
        let mgr = TeamManager::new();
        assert!(mgr.dissolve_team("ghost").is_err());
    }

    #[test]
    fn test_find_member() {
        let mgr = TeamManager::new();
        mgr.create_team("alpha", sid("s-1"), None).unwrap();
        mgr.add_member("alpha", "coder", sid("s-2"), None).unwrap();
        assert_eq!(mgr.find_member("alpha", "coder"), Some(sid("s-2")));
        assert_eq!(mgr.find_member("alpha", "ghost"), None);
    }

    #[test]
    fn test_list_teams() {
        let mgr = TeamManager::new();
        mgr.create_team("alpha", sid("s-1"), None).unwrap();
        mgr.create_team("beta", sid("s-2"), None).unwrap();
        assert_eq!(mgr.list_teams().len(), 2);
    }
}
