use std::collections::HashMap;

use octo_types::skill::{SkillDefinition, SkillTrigger};

/// Parsed slash command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommand {
    pub skill_name: String,
    pub args: Vec<String>,
}

/// Routes `/skill-name args` messages to skills.
///
/// Each skill automatically gets a route for `/skill-name`.
/// Skills with `Command` triggers also get routes for those commands.
pub struct SkillSlashRouter {
    /// Maps slash command name (lowercase) -> skill name
    routes: HashMap<String, String>,
}

impl SkillSlashRouter {
    /// Build router from skill definitions.
    pub fn build(skills: &[SkillDefinition]) -> Self {
        let mut routes = HashMap::new();
        for skill in skills {
            // Auto-register /skill-name
            routes.insert(skill.name.to_lowercase(), skill.name.clone());

            // Also register explicit command triggers
            for trigger in &skill.triggers {
                if let SkillTrigger::Command { command } = trigger {
                    let cmd = command.trim_start_matches('/').to_lowercase();
                    routes.insert(cmd, skill.name.clone());
                }
            }
        }
        Self { routes }
    }

    /// Try to route a message. Returns `Some` if it starts with `/` and
    /// matches a registered skill route.
    pub fn route(&self, message: &str) -> Option<SlashCommand> {
        let trimmed = message.trim();
        if !trimmed.starts_with('/') {
            return None;
        }

        let without_slash = &trimmed[1..];
        let mut parts = without_slash.splitn(2, char::is_whitespace);
        let cmd = parts.next()?.to_lowercase();
        let args_str = parts.next().unwrap_or("");

        if let Some(skill_name) = self.routes.get(&cmd) {
            let args: Vec<String> = if args_str.is_empty() {
                vec![]
            } else {
                args_str.split_whitespace().map(String::from).collect()
            };
            Some(SlashCommand {
                skill_name: skill_name.clone(),
                args,
            })
        } else {
            None
        }
    }

    /// List all registered routes as (command, skill_name) pairs.
    pub fn list_routes(&self) -> Vec<(&str, &str)> {
        self.routes
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }
}
