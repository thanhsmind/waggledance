//! Built-in slash commands the agent CLI itself answers (feature
//! `slash-builtin-commands`, D1 `86c7ef5f`).
//!
//! This table is DATA, not recall. Every row below was transcribed from the
//! output of the committed extraction tool run against an installed agent
//! bundle — a memory-written list was already proven wrong (it carried
//! `/cost`, `/review`, `/doctor`, `/todos`, `/rewind`, `/vim`, none of which
//! claude 2.1.251 registers). Never invent, edit, or "fix" an entry by hand:
//! the only legitimate change to this file is a full regeneration.
//!
//! Source: claude 2.1.251
//! (`~/.local/share/claude/versions/2.1.251`)
//!
//! Regenerate with:
//!
//! ```text
//! python3 crates/waggledance/tools/extract-agent-builtins.py \
//!     ~/.local/share/claude/versions/<version>
//! ```
//!
//! and transcribe the JSON rows into the table below, sorted by name,
//! dropping only rows whose description begins with `(removed)`. Rows with an
//! empty description are KEPT: a name with no blurb is still a real command
//! the agent answers. The table is a snapshot refreshed by hand — it does not
//! track the installed agent, and it is not a live query of the running
//! process.
//!
//! Only the `claude` vendor has a table today; no other agent CLI was
//! installed to extract from, and inventing one from memory is the exact
//! error this file exists to prevent.

/// `(name, argument_hint, description)` for every slash command claude 2.1.251
/// registers, sorted by name. An empty `argument_hint` or `description` means
/// the bundle carried none.
pub const CLAUDE_BUILTINS: &[(&str, &str, &str)] = &[
    ("add-dir", "<path>", "Add a new working directory"),
    ("advisor", "", "Let Claude consult a stronger model at key moments"),
    ("artifacts", "", "Browse your published and shared artifacts"),
    ("auto-mode-setup", "", "Teach auto mode about your environment, plus optional rule tweaks"),
    ("autocompact", "[auto|<tokens>]", "Set how full the context gets before auto-summarizing"),
    ("autofix-pr", "", "Monitor and autofix any issues with the current PR"),
    ("background", "[prompt]", "Send this session to the background and free the terminal"),
    ("branch", "[name]", "Create a branch of the current conversation at this point"),
    ("brief", "", "Toggle brief-only mode"),
    ("btw", "[question]", "Ask a quick side question without interrupting the main conversation"),
    ("bug", "[report]", "Report a bug or share your conversation"),
    ("cd", "<path>", "Move this session to a new working directory"),
    ("chrome", "", "Open Claude in Chrome settings"),
    ("clear", "[name]", "Start a new session with empty context; previous session stays on disk (resumable with /resume)"),
    ("cloud-plugins", "", "Choose whether cloud sessions use the plugins enabled on this machine"),
    ("color", "", "Set the prompt bar color for this session"),
    ("compact", "<optional custom summarization instructions>", "Free up context by summarizing the conversation so far"),
    ("config", "[key=value]", "Open settings"),
    ("context", "[all]", "Visualize current context usage as a colored grid"),
    ("copy", "", "Copy Claude's last response to clipboard (or /copy N for the Nth-latest)"),
    ("daemon", "", "Manage background services and routines"),
    ("design", "consent | revoke", "Grant or revoke Claude agent access to your Design projects"),
    ("design-consent", "", "Grant Claude agent access to your Design projects"),
    ("design-login", "", "Authorize design-system access for /design-sync with your claude.ai account"),
    ("design-revoke", "", "Revoke Claude agent access to your Design projects"),
    ("desktop", "", "Continue the current session in Claude Desktop"),
    ("diff", "", ""),
    ("effort", "", "Set effort level for model usage"),
    ("exit", "", ""),
    ("export", "[filename]", "Export the current conversation to a file or clipboard"),
    ("extra-usage", "", "Renamed to /usage-credits"),
    ("fast", "[on|off]", ""),
    ("feedback", "[report]", "Send feedback to Anthropic or report a bug"),
    ("focus", "", "Toggle focus view: just your prompt, summary, and response"),
    ("fork", "<directive>", "Spawn a background agent that inherits the full conversation"),
    ("function", "", ""),
    ("goal", "[<condition> | clear]", "Set a goal Claude checks before stopping"),
    ("heapdump", "", "Dump the JS heap to ~/Desktop"),
    ("help", "", "Show help and available commands"),
    ("hooks", "", "View hook configurations for tool events"),
    ("ide", "[open]", "Manage IDE integrations and show status"),
    ("import", "[codex|gemini] [--dry-run]", "Import config from another AI coding agent"),
    ("init", "", ""),
    ("insights", "", "Generate a report analyzing your Claude Code sessions"),
    ("install", "[options]", "Install Claude Code native build"),
    ("install-github-app", "", "Set up Claude GitHub Actions for a repository"),
    ("install-slack-app", "", "Install the Claude Slack app"),
    ("list-agents", "", "List subagents, teammates, and other Claude sessions you can message"),
    ("login", "", ""),
    ("logout", "", "Sign out from your Anthropic account"),
    ("loops", "", "List, create, and delete loops"),
    ("mcp", "[reconnect|enable|disable [<server>|all]]", "Manage MCP servers"),
    ("memory", "", "Edit CLAUDE.md files and memory settings"),
    ("mobile", "", "Show QR code to download the Claude mobile app"),
    ("model", "<model>", "Set the AI model for Claude Code"),
    ("passes", "", ""),
    ("pause-memory", "", "Pause automemory for this session"),
    ("permissions", "", "Manage allow and deny tool permission rules"),
    ("plan", "[open|share|<description>]", "Enable plan mode or view the current session plan"),
    ("plugin", "", "Manage Claude Code plugins"),
    ("plugin-types", "[dir]", "Write claude-code-mcp.d.ts: the inputs of the connected MCP tools, for typing a plugin against this session"),
    ("powerup", "", "Discover Claude Code features through quick interactive lessons"),
    ("privacy-settings", "", "View and update your privacy settings"),
    ("pro-trial-expired", "", "Options shown when the Pro plan Claude Code trial has ended"),
    ("radio", "", "Listen to Claude FM lo-fi radio"),
    ("rate-limit-options", "", "Show options when rate limit is reached"),
    ("recap", "", "Generate a one-line session recap now"),
    ("reload-plugins", "[--force]", "Activate pending plugin changes in the current session"),
    ("reload-skills", "", "Pick up skills added or changed on disk during this session"),
    ("remote-control", "", ""),
    ("remote-env", "", "Choose the default environment for cloud agents"),
    ("rename", "[name]", "Rename the current conversation"),
    ("resume", "[conversation id or search term]", "Resume a previous conversation"),
    ("scroll-speed", "", "Adjust mouse wheel scroll speed"),
    ("session", "", "Show cloud session URL and QR code"),
    ("setup-bedrock", "", "Reconfigure Amazon Bedrock authentication, region, or model pins"),
    ("setup-vertex", "", "Reconfigure Google Vertex AI authentication, project, region, or model pins"),
    ("skill-doctor", "", "Show which loaded skills are unused and costing context"),
    ("skills", "", "List available skills"),
    ("status", "", "Show Claude Code status including version, model, account, API connectivity, and tool statuses"),
    ("statusline", "", "Set up Claude Code's status line UI"),
    ("stickers", "", "Order Claude Code stickers"),
    ("stop", "", "Stop this background session; transcript and worktree are kept"),
    ("subtask", "<task>", "Send a subagent off with your full context; its result comes back here"),
    ("tasks", "", "View and manage everything running in the background"),
    ("team-onboarding", "", "Help teammates ramp on Claude Code with a guide from your usage"),
    ("teleport", "", "Send this session to the cloud, or resume one from claude.ai"),
    ("terminal-setup", "", ""),
    ("theme", "", "Change the theme"),
    ("tui", "[default|fullscreen]", "Set the terminal UI renderer (default | fullscreen)"),
    ("ultraplan", "<prompt>", ""),
    ("ultrareview", "", ""),
    ("update", "", "Switch to the latest version (conversation continues)"),
    ("upgrade", "", "Upgrade to Max for higher rate limits and more Opus"),
    ("usage", "", "Show session cost, plan usage, and activity stats"),
    ("usage-credits", "", "Configure usage credits or request them from your admin when you hit a limit"),
    ("version", "", "Show this session's version (autoupdate may have a newer one)"),
    ("voice", "[hold|tap|off]", "Toggle voice mode"),
    ("web-setup", "", "Set up Claude Code on the web with your GitHub account"),
    ("wellbeing", "", "Configure optional break reminders and quiet-hours nudges"),
    ("workflow-launch-exec", "", "Execute a server-launched workflow handoff (workflow_launch event sessions only)"),
    ("workflows", "", "Browse running and completed workflows"),
];
