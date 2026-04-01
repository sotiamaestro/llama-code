// Copyright 2026 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! System prompt templates embedded at compile time.

/// Core system prompt - the main agent identity and rules.
pub const SYSTEM_CORE: &str = r#"You are Llama Code, an AI coding agent running in the user's terminal. You have direct access to their filesystem and can execute commands on their behalf.

Your capabilities:
- Read, write, and edit files in the current working directory and below
- Execute bash commands
- Search codebases using ripgrep
- Interact with git repositories
- Plan multi-step changes before executing them

Your rules:
- Always show your plan before making changes
- Never modify files outside the current working directory without explicit permission
- When editing files, make minimal targeted changes - do not rewrite entire files unless asked
- If a command could be destructive (rm, overwrite, force push), ask for confirmation
- If you are unsure about something, say so rather than guessing
- When you encounter an error, explain what went wrong and suggest fixes
- Think step by step for complex tasks

You are running locally on the user's machine. You have no internet access. All your knowledge comes from your training data and the files in the current project."#;

/// Planning phase prompt - tells the model to plan before acting.
pub const SYSTEM_PLANNING: &str = r#"Before executing any changes, create a brief plan:

1. What files need to be read to understand the current state?
2. What changes are needed and in which files?
3. What is the order of operations?
4. What could go wrong and how will you verify success?

Keep plans concise. 3-5 bullet points max for simple tasks. More detail for complex multi-file changes.

After executing changes, verify:
- Did the file write succeed?
- Does the code parse/compile?
- Do existing tests still pass (if applicable)?"#;

/// Tool usage instructions.
pub const SYSTEM_TOOLS: &str = r#"To use a tool, respond with a JSON object in this format:
{"name": "tool_name", "parameters": {"param1": "value1"}}

Important:
- Only call one tool at a time
- Wait for the tool result before proceeding
- If a tool call fails, read the error message and try to fix the issue
- Always use relative paths from the current working directory"#;

/// Compacted history prompt - used when context is getting full.
pub const SYSTEM_COMPACT: &str = r#"You are Llama Code. You are deep in a coding session. Here is a summary of what has happened so far:

{compacted_history}

The user's latest request is below. Continue working from the current state."#;

/// Build the full system prompt with runtime-specific information.
pub fn build_system_prompt(cwd: &str, os: &str, tool_names: &[&str]) -> String {
    format!(
        "{SYSTEM_CORE}\n\n\
         Current working directory: {cwd}\n\
         Operating system: {os}\n\
         Available tools: {tools}\n\n\
         {SYSTEM_PLANNING}\n\n\
         {SYSTEM_TOOLS}",
        tools = tool_names.join(", "),
    )
}

/// Build a compacted system prompt with history summary.
pub fn build_compact_prompt(
    compacted_history: &str,
    cwd: &str,
    os: &str,
    tool_names: &[&str],
) -> String {
    let compact = SYSTEM_COMPACT.replace("{compacted_history}", compacted_history);
    format!(
        "{compact}\n\n\
         Current working directory: {cwd}\n\
         Operating system: {os}\n\
         Available tools: {tools}\n\n\
         {SYSTEM_TOOLS}",
        tools = tool_names.join(", "),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_system_prompt() {
        let prompt = build_system_prompt("/project", "macOS", &["file_read", "file_write", "bash"]);
        assert!(prompt.contains("Llama Code"));
        assert!(prompt.contains("/project"));
        assert!(prompt.contains("macOS"));
        assert!(prompt.contains("file_read, file_write, bash"));
    }

    #[test]
    fn test_build_compact_prompt() {
        let prompt = build_compact_prompt(
            "User asked to fix a bug in auth.rs. We read the file and identified the issue.",
            "/project",
            "Linux",
            &["file_read"],
        );
        assert!(prompt.contains("fix a bug"));
        assert!(prompt.contains("/project"));
    }

    #[test]
    fn test_templates_not_empty() {
        assert!(!SYSTEM_CORE.is_empty());
        assert!(!SYSTEM_PLANNING.is_empty());
        assert!(!SYSTEM_TOOLS.is_empty());
        assert!(!SYSTEM_COMPACT.is_empty());
    }
}
