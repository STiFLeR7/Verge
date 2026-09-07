/// Identity of a vendor AI coding tool. A fact about the vendor, not code —
/// adding a variant here does not imply an adapter exists for it yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolId {
    ClaudeCode,
    Cursor,
    Codex,
    Glm,
    Antigravity,
    Grok,
    OpenCode,
}

/// Static identity + capability flags for a `ToolId`. Not the same thing as
/// an adapter: this says what a tool can *in principle* tell us, not how.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tool {
    pub id: ToolId,
    pub display_name: &'static str,
    pub has_usage_signal: bool,
    pub has_activity_signal: bool,
}

impl Tool {
    pub const fn claude_code() -> Self {
        Tool {
            id: ToolId::ClaudeCode,
            display_name: "Claude Code",
            has_usage_signal: true,
            has_activity_signal: true,
        }
    }
}

/// One specific signed-in identity of a `Tool`, discovered on this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub tool: ToolId,
    /// Human-facing label, e.g. "Claude Code (default)".
    pub label: String,
    /// Where/how this account was discovered, e.g. "~/.claude/.credentials.json".
    pub provenance: String,
}
