//! Skill file management for OpenClaw.
//!
//! Skills live in  ~/.openclaw/workspace/skills/<skill-name>/
//! on every supported platform (macOS, Linux, Windows).
//! The home directory is resolved via Tauri's cross-platform path API,
//! so no hard-coded separators or platform-specific prefixes.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

// ── Path resolution ───────────────────────────────────────────────────────────

/// Returns  <home>/.openclaw/workspace/skills  cross-platform.
fn skills_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    Ok(home.join(".openclaw").join("workspace").join("skills"))
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillFile {
    pub content: String,
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

// ── Commands ──────────────────────────────────────────────────────────────────

/// List all skills with their metadata.
#[tauri::command]
pub fn list_skills(app: AppHandle) -> Result<Vec<SkillMeta>, String> {
    let dir = skills_dir(&app)?;
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
pub fn read_skill_file(app: AppHandle, skill_name: String, filename: String) -> Result<String, String> {
    validate_name(&skill_name)?;
    validate_name(&filename)?;
    let path = skills_dir(&app)?.join(&skill_name).join(&filename);
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// Write (create or overwrite) a file within a skill directory.
#[tauri::command]
pub fn write_skill_file(
    app: AppHandle,
    skill_name: String,
    filename: String,
    content: String,
) -> Result<(), String> {
    validate_name(&skill_name)?;
    validate_name(&filename)?;
    let dir = skills_dir(&app)?.join(&skill_name);
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    fs::write(dir.join(&filename), content).map_err(|e| e.to_string())
}

/// Create a new skill directory with a template SKILL.md.
#[tauri::command]
pub fn create_skill(app: AppHandle, skill_name: String) -> Result<(), String> {
    validate_name(&skill_name)?;
    let dir = skills_dir(&app)?.join(&skill_name);
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
pub fn delete_skill(app: AppHandle, skill_name: String) -> Result<(), String> {
    validate_name(&skill_name)?;
    let dir = skills_dir(&app)?.join(&skill_name);
    if !dir.exists() {
        return Err(format!("技能 '{}' 不存在", skill_name));
    }
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())
}

/// Delete a single file within a skill directory.
#[tauri::command]
pub fn delete_skill_file(app: AppHandle, skill_name: String, filename: String) -> Result<(), String> {
    validate_name(&skill_name)?;
    validate_name(&filename)?;
    let path = skills_dir(&app)?.join(&skill_name).join(&filename);
    fs::remove_file(&path).map_err(|e| e.to_string())
}

// ── Security: path traversal guard ───────────────────────────────────────────

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(format!("无效的名称: {:?}", name));
    }
    Ok(())
}
