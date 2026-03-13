import { checkAndFixGatewayConfig } from '@/api/gateway'
import { getOpenclawGatewayToken } from '@/api/skills'
import { useSettingsStore } from '@/stores/settings'

/**
 * 自动完成设置：检测修复 gateway 配置 + 获取认证 token + 保存。
 * 两步均为 best-effort，失败不阻断后续流程。
 */
export function useAutoSetup() {
  const settings = useSettingsStore()

  async function autoSetup(): Promise<void> {
    // 1. 检测并修复 gateway 配置
    try {
      await checkAndFixGatewayConfig()
    } catch {}

    // 2. 自动获取认证 token，成功则保存设置
    try {
      const token = await getOpenclawGatewayToken()
      const baseUrl = settings.baseUrl.trim() || 'http://127.0.0.1:18789'
      settings.save(token, settings.sessionKey, baseUrl)
    } catch {}
  }

  return { autoSetup }
}
