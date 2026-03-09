import { defineStore } from 'pinia'

const LS_TOKEN = 'openclaw_bearer_token'
const LS_BASE_URL = 'openclaw_base_url'

// Session key 按身份隔离，key 格式: openclaw_session_key:<profile>
const DEFAULT_SESSION_KEYS: Record<string, string> = {
  default: 'agent:main:main',
  work: 'agent:main:work',
  personal: 'agent:main:personal',
}

function sessionKeyStorageKey(profile: string) {
  return `openclaw_session_key:${profile}`
}

function loadSessionKey(profile: string): string {
  return (
    localStorage.getItem(sessionKeyStorageKey(profile)) ??
    DEFAULT_SESSION_KEYS[profile] ??
    'agent:main:main'
  )
}

export const useSettingsStore = defineStore('settings', () => {
  const bearerToken = ref(localStorage.getItem(LS_TOKEN) ?? '')
  const sessionKey = ref(loadSessionKey('default'))
  const baseUrl = ref(localStorage.getItem(LS_BASE_URL) ?? '')
  // 当前正在编辑哪个身份的 session key
  const currentProfile = ref('default')

  /** 切换到指定身份时加载对应的 session key */
  function loadForProfile(profile: string) {
    currentProfile.value = profile
    sessionKey.value = loadSessionKey(profile)
  }

  function save(token: string, key: string, url: string) {
    bearerToken.value = token.trim()
    sessionKey.value = key.trim()
    baseUrl.value = url.trim()
    localStorage.setItem(LS_TOKEN, token.trim())
    localStorage.setItem(sessionKeyStorageKey(currentProfile.value), key.trim())
    if (url.trim()) {
      localStorage.setItem(LS_BASE_URL, url.trim())
    } else {
      localStorage.removeItem(LS_BASE_URL)
    }
  }

  return { bearerToken, sessionKey, baseUrl, currentProfile, loadForProfile, save }
})
