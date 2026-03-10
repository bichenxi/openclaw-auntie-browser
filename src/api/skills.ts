import { invoke } from '@tauri-apps/api/core'

export interface SkillMeta {
  name: string
  version?: string
  description?: string
  files: string[]
}

export function listSkills(): Promise<SkillMeta[]> {
  return invoke('list_skills')
}

export function readSkillFile(skillName: string, filename: string): Promise<string> {
  return invoke('read_skill_file', { skillName, filename })
}

export function writeSkillFile(skillName: string, filename: string, content: string): Promise<void> {
  return invoke('write_skill_file', { skillName, filename, content })
}

export function createSkill(skillName: string): Promise<void> {
  return invoke('create_skill', { skillName })
}

export function deleteSkill(skillName: string): Promise<void> {
  return invoke('delete_skill', { skillName })
}

export function deleteSkillFile(skillName: string, filename: string): Promise<void> {
  return invoke('delete_skill_file', { skillName, filename })
}

export function checkBuiltinSkillInstalled(): Promise<boolean> {
  return invoke('check_builtin_skill_installed')
}

export function installBuiltinSkill(): Promise<void> {
  return invoke('install_builtin_skill')
}

export function getOpenclawGatewayToken(): Promise<string> {
  return invoke('get_openclaw_gateway_token')
}
