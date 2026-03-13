//! Skill file management for OpenClaw.
//!
//! Skills live in  ~/.openclaw/<workspace>/skills/<skill-name>/
//! on every supported platform (macOS, Linux, Windows).
//! The home directory is resolved via Tauri's cross-platform path API,
//! so no hard-coded separators or platform-specific prefixes.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

// ── Path resolution ───────────────────────────────────────────────────────────

/// Returns  <home>/.openclaw/<workspace>/skills  cross-platform.
fn skills_dir_for(app: &AppHandle, workspace: &str) -> Result<PathBuf, String> {
    validate_workspace(workspace)?;
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    Ok(home.join(".openclaw").join(workspace).join("skills"))
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillMeta {
    /// Directory name, also used as the skill key.
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    /// All filenames inside the skill directory.
    pub files: Vec<String>,
}


// ── Frontmatter parser ───────────────────────────────────────────────────────

/// Extracts (version, description) from a YAML frontmatter block `---\n...\n---`.
fn parse_frontmatter(text: &str) -> (Option<String>, Option<String>) {
    let mut version = None;
    let mut description = None;

    let inner = text.strip_prefix("---").and_then(|s| {
        let end = s.find("\n---")?;
        Some(&s[..end])
    });

    if let Some(block) = inner {
        for line in block.lines() {
            if let Some(v) = line.strip_prefix("version:") {
                version = Some(v.trim().to_string());
            } else if let Some(d) = line.strip_prefix("description:") {
                description = Some(d.trim().to_string());
            }
        }
    }
    (version, description)
}

// ── Built-in skill: claw-browser-control ─────────────────────────────────────

const BUILTIN_SKILL_NAME: &str = "claw-browser-control";
const BUILTIN_SKILL_MD: &str = include_str!("../../openclaw-skill/SKILL.md");

/// Returns true if the claw-browser-control skill is installed in the given workspace.
#[tauri::command]
pub fn check_builtin_skill_installed(app: AppHandle, workspace: String) -> Result<bool, String> {
    let path = skills_dir_for(&app, &workspace)?.join(BUILTIN_SKILL_NAME).join("SKILL.md");
    Ok(path.exists())
}

/// Installs (or re-installs) the bundled claw-browser-control skill into the given workspace,
/// and registers it in ~/.openclaw/openclaw.json.
#[tauri::command]
pub fn install_builtin_skill(app: AppHandle, workspace: String) -> Result<(), String> {
    let dir = skills_dir_for(&app, &workspace)?.join(BUILTIN_SKILL_NAME);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::write(dir.join("SKILL.md"), BUILTIN_SKILL_MD).map_err(|e| e.to_string())?;
    register_skill_in_openclaw(&app)?;
    Ok(())
}

/// Adds claw-browser-control to skills.entries in ~/.openclaw/openclaw.json
/// and also ensures `exec` is in tools.alsoAllow.
fn register_skill_in_openclaw(app: &AppHandle) -> Result<(), String> {
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let config_path = home.join(".openclaw").join("openclaw.json");
    if !config_path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    let mut json: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| e.to_string())?;

    // Register skill in skills.entries
    if let Some(entries) = json
        .get_mut("skills")
        .and_then(|s| s.get_mut("entries"))
        .and_then(|e| e.as_object_mut())
    {
        if !entries.contains_key(BUILTIN_SKILL_NAME) {
            entries.insert(
                BUILTIN_SKILL_NAME.to_string(),
                serde_json::json!({ "enabled": true }),
            );
        }
    }

    // Ensure exec is in tools.alsoAllow
    if let Some(also_allow) = json
        .get_mut("tools")
        .and_then(|t| t.get_mut("alsoAllow"))
        .and_then(|a| a.as_array_mut())
    {
        if !also_allow.iter().any(|v| v.as_str() == Some("exec")) {
            also_allow.push(serde_json::json!("exec"));
        }
    }

    let new_content = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
    fs::write(&config_path, new_content).map_err(|e| e.to_string())
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// List all workspace directories under ~/.openclaw/ that follow the
/// `workspace` or `workspace-<name>` naming convention.
#[tauri::command]
pub fn list_workspaces(app: AppHandle) -> Result<Vec<String>, String> {
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let openclaw_dir = home.join(".openclaw");
    if !openclaw_dir.exists() {
        return Ok(vec!["workspace".to_string()]);
    }
    let mut workspaces: Vec<String> = fs::read_dir(&openclaw_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .filter(|name| name == "workspace" || name.starts_with("workspace-"))
        .collect();
    workspaces.sort();
    if workspaces.is_empty() {
        workspaces.push("workspace".to_string());
    }
    Ok(workspaces)
}

/// List all skills with their metadata in the given workspace.
#[tauri::command]
pub fn list_skills(app: AppHandle, workspace: String) -> Result<Vec<SkillMeta>, String> {
    let dir = skills_dir_for(&app, &workspace)?;
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut skills = vec![];
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }

        let skill_md_path = path.join("SKILL.md");
        let (version, description) = if skill_md_path.exists() {
            let content = fs::read_to_string(&skill_md_path).unwrap_or_default();
            parse_frontmatter(&content)
        } else {
            (None, None)
        };

        let mut files: Vec<String> = fs::read_dir(&path)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| e.path().is_file())
                    .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        files.sort();

        skills.push(SkillMeta { name, version, description, files });
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

/// Read a specific file within a skill directory.
#[tauri::command]
pub fn read_skill_file(app: AppHandle, workspace: String, skill_name: String, filename: String) -> Result<String, String> {
    validate_name(&skill_name)?;
    validate_name(&filename)?;
    let path = skills_dir_for(&app, &workspace)?.join(&skill_name).join(&filename);
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// Write (create or overwrite) a file within a skill directory.
#[tauri::command]
pub fn write_skill_file(
    app: AppHandle,
    workspace: String,
    skill_name: String,
    filename: String,
    content: String,
) -> Result<(), String> {
    validate_name(&skill_name)?;
    validate_name(&filename)?;
    let dir = skills_dir_for(&app, &workspace)?.join(&skill_name);
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    fs::write(dir.join(&filename), content).map_err(|e| e.to_string())
}

/// Create a new skill directory with a template SKILL.md.
#[tauri::command]
pub fn create_skill(app: AppHandle, workspace: String, skill_name: String) -> Result<(), String> {
    validate_name(&skill_name)?;
    let dir = skills_dir_for(&app, &workspace)?.join(&skill_name);
    if dir.exists() {
        return Err(format!("技能 '{}' 已存在", skill_name));
    }
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let template = format!(
        "---\nname: {name}\nversion: 1.0\ndescription: 描述这个技能的用途\n---\n\n# {name}\n\n> 在这里描述这个技能的功能、使用场景和操作方式。\n",
        name = skill_name
    );
    fs::write(dir.join("SKILL.md"), template).map_err(|e| e.to_string())
}

/// Delete an entire skill directory.
#[tauri::command]
pub fn delete_skill(app: AppHandle, workspace: String, skill_name: String) -> Result<(), String> {
    validate_name(&skill_name)?;
    let dir = skills_dir_for(&app, &workspace)?.join(&skill_name);
    if !dir.exists() {
        return Err(format!("技能 '{}' 不存在", skill_name));
    }
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())
}

/// Delete a single file within a skill directory.
#[tauri::command]
pub fn delete_skill_file(app: AppHandle, workspace: String, skill_name: String, filename: String) -> Result<(), String> {
    validate_name(&skill_name)?;
    validate_name(&filename)?;
    let path = skills_dir_for(&app, &workspace)?.join(&skill_name).join(&filename);
    fs::remove_file(&path).map_err(|e| e.to_string())
}

/// Reads gateway.auth.token from ~/.openclaw/openclaw.json.
/// Returns the token string, or an error if the file/field is missing.
#[tauri::command]
pub fn get_openclaw_gateway_token(app: AppHandle) -> Result<String, String> {
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let config_path = home.join(".openclaw").join("openclaw.json");
    if !config_path.exists() {
        return Err("未找到 ~/.openclaw/openclaw.json，请先启动 OpenClaw 完成初始化".to_string());
    }
    let content = fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    json.get("gateway")
        .and_then(|g| g.get("auth"))
        .and_then(|a| a.get("token"))
        .and_then(|t| t.as_str())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .ok_or_else(|| "openclaw.json 中未找到 gateway.auth.token".to_string())
}

// ── Security: path traversal guards ──────────────────────────────────────────

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(format!("无效的名称: {:?}", name));
    }
    Ok(())
}

fn validate_workspace(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || !(name == "workspace" || name.starts_with("workspace-"))
    {
        return Err(format!("无效的 workspace 名称: {:?}", name));
    }
    Ok(())
}


// ── Security: path traversal guards ──────────────────────────────────────────
