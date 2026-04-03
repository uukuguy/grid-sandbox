//! Session commands implementation

use crate::commands::{AppState, SessionCommands};
use crate::ui::table::Table;
use anyhow::Result;
use grid_types::SessionId;

/// Handle session commands
pub async fn handle_session(action: SessionCommands, state: &AppState) -> Result<()> {
    match action {
        SessionCommands::List { limit } => list_sessions(limit, state).await?,
        SessionCommands::Create { name } => create_session(name, state).await?,
        SessionCommands::Show { session_id } => show_session(session_id, state).await?,
        SessionCommands::Delete { session_id } => delete_session(session_id, state).await?,
        SessionCommands::Export {
            session_id,
            format,
            output,
        } => {
            export_session(session_id, format, output, state).await?;
        }
    }
    Ok(())
}

/// List all sessions with table-formatted output
async fn list_sessions(limit: usize, state: &AppState) -> Result<()> {
    let session_store = state.agent_runtime.session_store();
    let sessions = session_store.list_sessions(limit, 0).await;

    if sessions.is_empty() {
        println!("No sessions found.");
        return Ok(());
    }

    let mut table = Table::new(vec!["Session ID", "Created", "Messages"]);
    for session in &sessions {
        let created = chrono::DateTime::from_timestamp(session.created_at, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| session.created_at.to_string());
        table.add_row(vec![
            session.session_id.clone(),
            created,
            session.message_count.to_string(),
        ]);
    }
    table.print();
    println!("\n{} session(s) total", sessions.len());
    Ok(())
}

/// Create a new session
async fn create_session(name: Option<String>, state: &AppState) -> Result<()> {
    let session_store = state.agent_runtime.session_store();
    let session = session_store.create_session().await;

    println!("Created session: {}", session.session_id);
    if let Some(n) = name {
        println!("  Name: {}", n);
    }
    Ok(())
}

/// Show session details
async fn show_session(session_id: String, state: &AppState) -> Result<()> {
    let session_store = state.agent_runtime.session_store();
    let sid = SessionId::from_string(&session_id);

    match session_store.get_session(&sid).await {
        Some(session) => {
            let msg_count = session_store
                .get_messages(&sid)
                .await
                .map(|msgs| msgs.len())
                .unwrap_or(0);
            let created = chrono::DateTime::from_timestamp(session.created_at, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| session.created_at.to_string());

            println!("Session: {}", session.session_id);
            println!("  User ID:  {}", session.user_id);
            println!("  Sandbox:  {}", session.sandbox_id);
            println!("  Created:  {}", created);
            println!("  Messages: {}", msg_count);
        }
        None => {
            eprintln!("Session not found: {}", session_id);
        }
    }
    Ok(())
}

/// Delete a session
async fn delete_session(session_id: String, state: &AppState) -> Result<()> {
    let session_store = state.agent_runtime.session_store();
    let sid = SessionId::from_string(&session_id);

    if session_store.delete_session(&sid).await {
        println!("Deleted session: {}", session_id);
    } else {
        eprintln!("Session not found: {}", session_id);
    }
    Ok(())
}

/// Export a session in json, markdown, or html format
async fn export_session(
    session_id: String,
    format: String,
    output: Option<String>,
    state: &AppState,
) -> Result<()> {
    let session_store = state.agent_runtime.session_store();
    let sid = SessionId::from_string(&session_id);

    let session = session_store
        .get_session(&sid)
        .await
        .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

    let messages = session_store
        .get_messages(&sid)
        .await
        .unwrap_or_default();

    let content = match format.as_str() {
        "json" => export_json(&session, &messages)?,
        "markdown" | "md" => export_markdown(&session, &messages),
        "html" => export_html(&session, &messages),
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported format: {}. Use json, markdown, or html.",
                format
            ));
        }
    };

    match output {
        Some(path) => {
            std::fs::write(&path, &content)?;
            println!("Exported session {} to {}", session_id, path);
        }
        None => {
            print!("{}", content);
        }
    }

    Ok(())
}

fn role_label(role: &grid_types::MessageRole) -> &'static str {
    match role {
        grid_types::MessageRole::User => "User",
        grid_types::MessageRole::Assistant => "Assistant",
        grid_types::MessageRole::System => "System",
    }
}

fn export_json(
    session: &grid_engine::session::SessionData,
    messages: &[grid_types::ChatMessage],
) -> Result<String> {
    let export = serde_json::json!({
        "session_id": session.session_id.to_string(),
        "user_id": session.user_id.to_string(),
        "sandbox_id": session.sandbox_id.to_string(),
        "created_at": session.created_at,
        "messages": messages,
    });
    Ok(serde_json::to_string_pretty(&export)?)
}

fn export_markdown(
    session: &grid_engine::session::SessionData,
    messages: &[grid_types::ChatMessage],
) -> String {
    let mut md = format!("# Session: {}\n\n", session.session_id);
    let created = chrono::DateTime::from_timestamp(session.created_at, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| session.created_at.to_string());
    md.push_str(&format!("Created: {}\n\n---\n\n", created));

    for msg in messages {
        md.push_str(&format!("## {}\n\n", role_label(&msg.role)));
        md.push_str(&format!("{}\n\n", msg.text_content()));
    }

    md
}

fn export_html(
    session: &grid_engine::session::SessionData,
    messages: &[grid_types::ChatMessage],
) -> String {
    let css = "body{font-family:sans-serif;max-width:800px;margin:0 auto;padding:20px}\
        .user{background:#e3f2fd;padding:12px;border-radius:8px;margin:8px 0}\
        .assistant{background:#f5f5f5;padding:12px;border-radius:8px;margin:8px 0}\
        .system{background:#fff3e0;padding:12px;border-radius:8px;margin:8px 0}\
        .role{font-weight:bold;margin-bottom:4px}";
    let sid = html_escape(&session.session_id.to_string());
    let mut html = format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<title>Session {sid}</title>\n\
         <style>\n{css}\n</style>\n</head>\n<body>\n<h1>Session: {sid}</h1>\n"
    );
    for msg in messages {
        let role = role_label(&msg.role);
        let cls = match msg.role {
            grid_types::MessageRole::User => "user",
            grid_types::MessageRole::Assistant => "assistant",
            grid_types::MessageRole::System => "system",
        };
        html.push_str(&format!(
            "<div class=\"{cls}\">\n<div class=\"role\">{role}</div>\n\
             <div>{}</div>\n</div>\n",
            html_escape(&msg.text_content())
        ));
    }
    html.push_str("</body>\n</html>\n");
    html
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use grid_engine::session::SessionData;
    use grid_types::{ChatMessage, MessageRole, SandboxId, SessionId, UserId};

    fn test_session() -> SessionData {
        SessionData {
            session_id: SessionId::from_string("test-session-001"),
            user_id: UserId::from_string("user-1"),
            sandbox_id: SandboxId::from_string("sandbox-1"),
            created_at: 1700000000,
        }
    }

    fn test_messages() -> Vec<ChatMessage> {
        vec![
            ChatMessage::user("Hello, how are you?"),
            ChatMessage::assistant("I'm doing well, thanks!"),
        ]
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape(""), "");
        assert_eq!(html_escape("hello"), "hello");
        assert_eq!(
            html_escape("<script>alert(\"xss\")&</script>"),
            "&lt;script&gt;alert(&quot;xss&quot;)&amp;&lt;/script&gt;"
        );
    }

    #[test]
    fn test_role_label() {
        assert_eq!(role_label(&MessageRole::User), "User");
        assert_eq!(role_label(&MessageRole::Assistant), "Assistant");
        assert_eq!(role_label(&MessageRole::System), "System");
    }

    #[test]
    fn test_export_json() {
        let session = test_session();
        let json_str = export_json(&session, &test_messages()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(v["session_id"], "test-session-001");
        assert_eq!(v["user_id"], "user-1");
        assert_eq!(v["created_at"], 1700000000);
        assert_eq!(v["messages"].as_array().unwrap().len(), 2);
        // empty messages
        let j2 = export_json(&session, &[]).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&j2).unwrap();
        assert_eq!(v2["messages"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_export_markdown() {
        let session = test_session();
        let md = export_markdown(&session, &test_messages());
        assert!(md.starts_with("# Session: test-session-001\n"));
        assert!(md.contains("---"));
        assert!(md.contains("## User\n\nHello, how are you?"));
        assert!(md.contains("## Assistant\n\nI'm doing well, thanks!"));
        // empty messages
        let md2 = export_markdown(&session, &[]);
        assert!(md2.contains("# Session:"));
        assert!(!md2.contains("## User"));
    }

    #[test]
    fn test_export_html() {
        let session = test_session();
        let html = export_html(&session, &test_messages());
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<title>Session test-session-001</title>"));
        assert!(html.contains("<div class=\"user\">"));
        assert!(html.contains("<div class=\"assistant\">"));
        assert!(html.contains("</html>"));
        // XSS escaping
        let html2 = export_html(&session, &[ChatMessage::user("<b>bold</b>")]);
        assert!(html2.contains("&lt;b&gt;bold&lt;/b&gt;"));
        assert!(!html2.contains("<b>bold</b>"));
        // empty
        let html3 = export_html(&session, &[]);
        assert!(!html3.contains("class=\"user\""));
    }
}
