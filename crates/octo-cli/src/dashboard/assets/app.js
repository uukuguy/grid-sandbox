// Octo Dashboard — Alpine.js Application
// Embedded via include_str!() — Alpine.js loaded from CDN in index.html

function app() {
    return {
        // State
        tab: 'chat',
        connected: false,
        input: '',
        messages: [],
        sessions: [],
        memories: [],
        mcpServers: [],
        messageId: 0,

        // Lifecycle
        init() {
            this.checkHealth();
            // Poll health every 5 seconds
            setInterval(() => this.checkHealth(), 5000);
        },

        // API Methods
        async checkHealth() {
            try {
                const res = await fetch('/api/health');
                if (res.ok) {
                    this.connected = true;
                } else {
                    this.connected = false;
                }
            } catch {
                this.connected = false;
            }
        },

        async sendMessage() {
            if (!this.input.trim()) return;
            const content = this.input;
            this.input = '';

            // Add user message
            this.messages.push({
                id: ++this.messageId,
                role: 'user',
                content: content,
            });

            // TODO: Send to backend via WebSocket or POST
            // For now, echo back
            this.messages.push({
                id: ++this.messageId,
                role: 'assistant',
                content: 'Dashboard chat is a preview. Use the CLI for full agent interaction.',
            });
        },

        async refreshSessions() {
            try {
                const res = await fetch('/api/sessions');
                if (res.ok) this.sessions = await res.json();
            } catch { /* ignore */ }
        },

        async refreshMemories() {
            try {
                const res = await fetch('/api/memories');
                if (res.ok) this.memories = await res.json();
            } catch { /* ignore */ }
        },

        async refreshMcp() {
            try {
                const res = await fetch('/api/mcp/servers');
                if (res.ok) this.mcpServers = await res.json();
            } catch { /* ignore */ }
        },
    };
}
