import { defineStore } from 'pinia'
import * as profileApi from '@/api/profile'
import { useTabsStore } from '@/stores/tabs'
import { useRecordingStore } from '@/stores/recording'
import { useSettingsStore } from '@/stores/settings'

export const PROFILE_OPTIONS = ['default', 'work', 'personal'] as const
export type ProfileName = (typeof PROFILE_OPTIONS)[number]

export const useProfileStore = defineStore('profile', () => {
  const currentProfile = ref<string>('default')

  async function loadProfile() {
    try {
      currentProfile.value = await profileApi.getCurrentProfile()
    } catch {
      currentProfile.value = 'default'
    }
    useSettingsStore().loadForProfile(currentProfile.value)
  }

  /** 切换身份：关闭所有 tab、清空录制并写入新 profile，后续新开页将使用该 profile 的数据目录 */
  async function switchProfile(name: string): Promise<void> {
    const tabsStore = useTabsStore()
    const recordingStore = useRecordingStore()
    const ids = [...tabsStore.tabs.map((t) => t.id)]
    for (const id of ids) {
      await tabsStore.closeTab(id).catch(() => {})
    }
    recordingStore.clearSteps()
    await profileApi.setCurrentProfile(name)
    currentProfile.value = name
    useSettingsStore().loadForProfile(name)
  }

  return {
    currentProfile,
    loadProfile,
    switchProfile,
  }
})
