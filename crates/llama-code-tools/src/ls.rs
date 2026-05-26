// Copyright 2026 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Tree-style directory listing tool.

use crate::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use bytesize::ByteSize;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Default depth for directory listings.
const DEFAULT_DEPTH: usize = 2;

pub struct LsTool;

#[async_trait]
impl Tool for LsTool {
    fn name(&self) -> &str {
        "ls"
    }

    fn description(&self) -> &str {
        "List directory contents in a tree-style format. Respects .gitignore. \
         Shows file sizes and entry counts for directories."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory to list (default: current directory)"
                },
                "depth": {
                    "type": "integer",
                    "description": "Maximum depth to traverse (default: 2)"
                },
                "include_hidden": {
                    "type": "boolean",
                    "description": "Include hidden files (default: false)"
                }
            }
        })
    }

    async fn execute(&self, params: serde_json::Value, ctx: &ToolContext) -> ToolResult {
        let path = params.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let depth = params
            .get("depth")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(DEFAULT_DEPTH);

        let include_hidden = params
            .get("include_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let resolved = ctx.resolve_path(path);

        if !resolved.exists() {
            return ToolResult::error(format!("Directory not found: {path}"));
        }

        if !resolved.is_dir() {
            return ToolResult::error(format!("Not a directory: {path}"));
        }

        let tree = build_tree(&resolved, depth, include_hidden);
        ToolResult::success(format!("📁 {path}\n{tree}"))
    }
}

fn build_tree(root: &Path, max_depth: usize, include_hidden: bool) -> String {
    let mut entries: Vec<TreeEntry> = Vec::new();

    let walker = WalkBuilder::new(root)
        .max_depth(Some(max_depth + 1))
        .hidden(!include_hidden)
        .git_ignore(true)
        .sort_by_file_name(|a, b| a.cmp(b))
        .build();

    for entry in walker.flatten() {
        let entry_path = entry.path();

        // Skip the root itself
        if entry_path == root {
            continue;
        }

        let relative = match entry_path.strip_prefix(root) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let depth = relative.components().count();
        if depth > max_depth {
            continue;
        }

        let name = relative
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let is_dir = entry_path.is_dir();
        let size = if is_dir {
            None
        } else {
            entry_path.metadata().ok().map(|m| m.len())
        };

        entries.push(TreeEntry {
            relative: relative.to_path_buf(),
            name,
            depth,
            is_dir,
            size,
        });
    }

    format_tree(&entries)
}

struct TreeEntry {
    relative: PathBuf,
    name: String,
    depth: usize,
    is_dir: bool,
    size: Option<u64>,
}

fn is_last_sibling(entries: &[TreeEntry], index: usize) -> bool {
    let entry = &entries[index];
    let parent = entry.relative.parent();
    let depth = entry.depth;

    !entries[index + 1..].iter().any(|next| {
        next.depth == depth && next.relative.parent() == parent
    })
}

fn format_tree(entries: &[TreeEntry]) -> String {
    let mut output = String::new();
    let mut parent_is_last: Vec<bool> = Vec::new();

    for (index, entry) in entries.iter().enumerate() {
        parent_is_last.truncate(entry.depth.saturating_sub(1));

        let continuation: String = parent_is_last
            .iter()
            .map(|&is_last| {
                if is_last {
                    "    "
                } else {
                    "│   "
                }
            })
            .collect();

        let branch = if entry.depth == 0 {
            String::new()
        } else if is_last_sibling(entries, index) {
            "└── ".to_string()
        } else {
            "├── ".to_string()
        };

        if entry.is_dir {
            output.push_str(&format!(
                "{continuation}{branch}📁 {}/\n",
                entry.name
            ));
        } else {
            let size_str = entry
                .size
                .map(|s| format!(" ({})", ByteSize(s)))
                .unwrap_or_default();
            output.push_str(&format!(
                "{continuation}{branch}{}{size_str}\n",
                entry.name
            ));
        }

        if entry.depth > 0 {
            let is_last = is_last_sibling(entries, index);
            if parent_is_last.len() < entry.depth {
                parent_is_last.push(is_last);
            } else {
                parent_is_last[entry.depth - 1] = is_last;
            }
        } else {
            parent_is_last.clear();
        }
    }

    if output.ends_with('\n') {
        output.pop();
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_ls_basic() {
        let dir = TempDir::new().expect("tempdir creation should succeed");
        fs::write(dir.path().join("file1.rs"), "code").expect("file write should succeed");
        fs::write(dir.path().join("file2.txt"), "text").expect("file write should succeed");
        fs::create_dir(dir.path().join("subdir")).expect("directory creation should succeed");
        fs::write(dir.path().join("subdir/nested.rs"), "nested")
            .expect("file write should succeed");

        let tool = LsTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());

        let result = tool.execute(serde_json::json!({}), &ctx).await;

        assert!(result.is_success());
        assert!(result.content.contains("file1.rs"));
        assert!(result.content.contains("subdir"));
    }

    #[test]
    fn format_tree_uses_standard_branch_characters() {
        let entries = vec![
            TreeEntry {
                relative: PathBuf::from("alpha"),
                name: "alpha".into(),
                depth: 1,
                is_dir: true,
                size: None,
            },
            TreeEntry {
                relative: PathBuf::from("alpha/one.txt"),
                name: "one.txt".into(),
                depth: 2,
                is_dir: false,
                size: Some(1),
            },
            TreeEntry {
                relative: PathBuf::from("alpha/two.txt"),
                name: "two.txt".into(),
                depth: 2,
                is_dir: false,
                size: Some(1),
            },
            TreeEntry {
                relative: PathBuf::from("beta"),
                name: "beta".into(),
                depth: 1,
                is_dir: true,
                size: None,
            },
        ];

        let tree = format_tree(&entries);
        assert!(tree.contains("├── 📁 alpha/"));
        assert!(tree.contains("│   ├── one.txt"));
        assert!(tree.contains("│   └── two.txt"));
        assert!(tree.contains("└── 📁 beta/"));
    }

    #[tokio::test]
    async fn test_ls_nonexistent() {
        let dir = TempDir::new().expect("tempdir creation should succeed");
        let tool = LsTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());

        let result = tool
            .execute(serde_json::json!({"path": "nonexistent"}), &ctx)
            .await;

        assert!(!result.is_success());
    }
}
