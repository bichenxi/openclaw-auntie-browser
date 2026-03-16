import { invoke } from '@tauri-apps/api/core'

export interface SkillMeta {
  name: string
  version?: string
  description?: string
  files: string[]
}

export function listWorkspaces(): Promise<string[]> {
  return invoke('list_workspaces')
}

export function listSkills(workspace: string): Promise<SkillMeta[]> {
  return invoke('list_skills', { workspace })
}

export function readSkillFile(workspace: string, skillName: string, filename: string): Promise<string> {
  return invoke('read_skill_file', { workspace, skillName, filename })
}

export function writeSkillFile(workspace: string, skillName: string, filename: string, content: string): Promise<void> {
  return invoke('write_skill_file', { workspace, skillName, filename, content })
}

export function createSkill(workspace: string, skillName: string): Promise<void> {
  return invoke('create_skill', { workspace, skillName })
}

export function deleteSkill(workspace: string, skillName: string): Promise<void> {
  return invoke('delete_skill', { workspace, skillName })
}

export function deleteSkillFile(workspace: string, skillName: string, filename: string): Promise<void> {
  return invoke('delete_skill_file', { workspace, skillName, filename })
}

export function checkBuiltinSkillInstalled(workspace: string): Promise<boolean> {
  return invoke('check_builtin_skill_installed', { workspace })
}

export function installBuiltinSkill(workspace: string): Promise<void> {
  return invoke('install_builtin_skill', { workspace })
}

export function getOpenclawGatewayToken(): Promise<string> {
  return invoke('get_openclaw_gateway_token')
}

export interface SyncResult {
  added: string[]
  removed: string[]
}

export function syncSkillsToConfig(): Promise<SyncResult> {
  return invoke('sync_skills_to_config')
}
