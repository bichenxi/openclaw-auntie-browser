<script setup lang="ts">
import { useSettingsStore } from '@/stores/settings'
import { useTabsStore } from '@/stores/tabs'
import { useProfileStore, PROFILE_OPTIONS } from '@/stores/profile'
import { checkOpenclawAlive } from '@/api/openclaw'
import { getOpenclawGatewayToken } from '@/api/skills'

const settings = useSettingsStore()
const store = useTabsStore()
const profileStore = useProfileStore()

const tokenInput = ref(settings.bearerToken)
const sessionKeyInput = ref(settings.sessionKey)
const baseUrlInput = ref(settings.baseUrl)
const saved = ref(false)
const switching = ref(false)
const openclawAlive = ref(false)
const checkingAlive = ref(false)
const fetchingToken = ref(false)
const fetchTokenError = ref('')

const profileLabels: Record<string, string> = {
  default: '默认',
  work: '工作',
  personal: '个人',
}

watch(() => settings.sessionKey, (val) => {
  sessionKeyInput.value = val
})

async function selectProfile(name: string) {
  if (name === profileStore.currentProfile || switching.value) return
  switching.value = true
  try {
    await profileStore.switchProfile(name)
  } finally {
    switching.value = false
  }
}

async function refreshStatus() {
  checkingAlive.value = true
  openclawAlive.value = await checkOpenclawAlive(baseUrlInput.value || undefined)
  checkingAlive.value = false
}

async function fetchGatewayToken() {
  fetchingToken.value = true
  fetchTokenError.value = ''
  try {
    tokenInput.value = await getOpenclawGatewayToken()
  } catch (e: any) {
    fetchTokenError.value = e?.message ?? String(e)
  } finally {
    fetchingToken.value = false
  }
}

function save() {
  settings.save(tokenInput.value, sessionKeyInput.value, baseUrlInput.value)
  saved.value = true
  setTimeout(() => { saved.value = false }, 2000)
}

onMounted(refreshStatus)
</script>

<template>
  <div class="h-full bg-[#f8f6ff] overflow-hidden" style="display:grid;grid-template-rows:auto 1fr">

    <!-- Header -->
    <div class="flex items-center justify-between px-6 py-3.5 bg-white border-b border-[#e8e2f4]">
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-[10px] bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] flex items-center justify-center shadow-[0_2px_10px_rgba(95,71,206,0.25)] shrink-0">
          <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
          </svg>
        </div>
        <div>
          <div class="text-[16px] font-bold text-[#1f1f2e] leading-[1.2]">设置</div>
          <div class="text-[11px] text-[#9b8ec4] mt-px">OpenClaw 连接配置</div>
        </div>
      </div>
      <button
        type="button"
        class="flex items-center gap-1.5 px-3.5 py-[7px] text-[13px] text-secondary bg-secondary/8 border border-secondary/20 rounded-[8px] cursor-pointer transition hover:bg-secondary/13"
        @click="store.switchToSpecialView('openclaw')"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
        </svg>
        前往对话
      </button>
    </div>

    <!-- Body -->
    <div class="sp-body overflow-y-auto p-6 max-w-[700px] w-full mx-auto box-border">

      <!-- ── 身份选择 ── -->
      <div class="bg-white border border-[#e8e2f4] rounded-[12px] p-5 mb-4">
        <div class="flex items-center gap-[7px] text-[13px] font-semibold text-secondary mb-3">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
            <circle cx="12" cy="7" r="4" />
          </svg>
          当前身份
        </div>
        <div class="flex gap-2 mb-3">
          <button
            v-for="name in PROFILE_OPTIONS"
            :key="name"
            type="button"
            class="flex-1 py-2.5 text-[13px] font-medium rounded-[9px] border cursor-pointer transition disabled:opacity-60 disabled:cursor-not-allowed"
            :class="profileStore.currentProfile === name
              ? 'text-secondary border-secondary/35 bg-secondary/10 shadow-[0_0_0_3px_rgba(95,71,206,0.08)]'
              : 'text-[#8a80a7] border-[#e8e2f4] bg-transparent hover:text-secondary hover:border-secondary/25 hover:bg-secondary/5'"
            :disabled="switching"
            @click="selectProfile(name)"
          >
            {{ profileLabels[name] ?? name }}
          </button>
        </div>
        <p class="text-[11px] text-[#c4bdd8] m-0 leading-[1.5]">
          切换身份会关闭所有当前网页标签，并使用独立的浏览器数据目录。
        </p>
      </div>

      <!-- ── 会话配置（per-profile）── -->
      <div class="bg-white border border-[#e8e2f4] rounded-[12px] overflow-hidden mb-4">
        <div class="flex items-center gap-[7px] px-5 py-3.5 border-b border-[#f0ecfa]">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-secondary">
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
          </svg>
          <span class="text-[13px] font-semibold text-secondary">会话配置</span>
          <span class="ml-auto flex items-center gap-1.5 px-2.5 py-1 rounded-[6px] text-[11px] font-medium bg-secondary/8 text-secondary">
            <span
              class="w-1.5 h-1.5 rounded-full bg-secondary"
              :class="{ 'animate-[sp-pulse_1.5s_ease-in-out_infinite]': !switching }"
            />
            {{ profileLabels[profileStore.currentProfile] ?? profileStore.currentProfile }}
          </span>
        </div>
        <div class="p-5">
          <label class="block text-[12px] font-semibold text-[#4a4568] mb-1.5">OPENCLAW_SESSION_KEY</label>
          <p class="text-[11px] text-[#9b8ec4] m-0 mb-2.5">每个身份拥有独立的会话标识，切换身份后此处自动更新。</p>
          <input
            v-model="sessionKeyInput"
            type="text"
            class="w-full px-3.5 py-2.5 text-[13px] font-[inherit] border-[1.5px] border-[#e8e2f4] rounded-[8px] outline-none box-border text-[#1f1f2e] bg-[#fafafa] transition placeholder-[#c4bdd8] focus:border-[#7c5cfc] focus:bg-white focus:shadow-[0_0_0_3px_rgba(95,71,206,0.08)]"
            :placeholder="`agent:main:${profileStore.currentProfile === 'default' ? 'main' : profileStore.currentProfile}`"
          />
        </div>
      </div>

      <!-- ── 认证配置（全局）── -->
      <div class="bg-white border border-[#e8e2f4] rounded-[12px] overflow-hidden mb-4">
        <div class="flex items-center gap-[7px] px-5 py-3.5 border-b border-[#f0ecfa]">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-secondary">
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
            <path d="M7 11V7a5 5 0 0 1 10 0v4" />
          </svg>
          <span class="text-[13px] font-semibold text-secondary">认证配置</span>
          <span class="ml-auto text-[11px] text-[#c4bdd8]">所有身份共用</span>
        </div>
        <div class="p-5">
          <label class="block text-[12px] font-semibold text-[#4a4568] mb-1.5">OPENCLAW_BEARER_TOKEN</label>
          <p class="text-[11px] text-[#9b8ec4] m-0 mb-2.5">Bearer Token，用于 API 鉴权。若已设置同名环境变量可留空。</p>
          <div class="flex gap-2">
            <input
              v-model="tokenInput"
              type="password"
              class="flex-1 min-w-0 px-3.5 py-2.5 text-[13px] font-[inherit] border-[1.5px] border-[#e8e2f4] rounded-[8px] outline-none box-border text-[#1f1f2e] bg-[#fafafa] transition placeholder-[#c4bdd8] focus:border-[#7c5cfc] focus:bg-white focus:shadow-[0_0_0_3px_rgba(95,71,206,0.08)]"
              placeholder="输入 Bearer Token"
              autocomplete="off"
              @blur="tokenInput = tokenInput.trim()"
            />
            <button
              type="button"
              class="shrink-0 px-3.5 py-2.5 text-[12px] font-medium rounded-[8px] border border-[#e8e2f4] text-[#8a80a7] bg-transparent cursor-pointer transition whitespace-nowrap hover:text-secondary hover:border-secondary/30 hover:bg-secondary/6 disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="fetchingToken"
              @click="fetchGatewayToken"
            >
              {{ fetchingToken ? '读取中…' : '自动获取' }}
            </button>
          </div>
          <p v-if="fetchTokenError" class="text-[11px] text-accent m-0 mt-2">{{ fetchTokenError }}</p>
        </div>
      </div>

      <!-- ── 连接配置（全局）── -->
      <div class="bg-white border border-[#e8e2f4] rounded-[12px] overflow-hidden mb-4">
        <div class="flex items-center gap-[7px] px-5 py-3.5 border-b border-[#f0ecfa]">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-secondary">
            <circle cx="12" cy="12" r="10" />
            <line x1="2" y1="12" x2="22" y2="12" />
            <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
          </svg>
          <span class="text-[13px] font-semibold text-secondary">连接配置</span>
          <span class="ml-auto text-[11px] text-[#c4bdd8]">所有身份共用</span>
        </div>
        <div class="p-5">
          <label class="block text-[12px] font-semibold text-[#4a4568] mb-1.5">Base URL</label>
          <p class="text-[11px] text-[#9b8ec4] m-0 mb-2.5">OpenClaw HTTP 服务地址，留空使用默认值。</p>
          <input
            v-model="baseUrlInput"
            type="text"
            class="w-full px-3.5 py-2.5 text-[13px] font-[inherit] border-[1.5px] border-[#e8e2f4] rounded-[8px] outline-none box-border text-[#1f1f2e] bg-[#fafafa] transition placeholder-[#c4bdd8] focus:border-[#7c5cfc] focus:bg-white focus:shadow-[0_0_0_3px_rgba(95,71,206,0.08)]"
            placeholder="http://127.0.0.1:18789"
          />
          <!-- 连接状态 -->
          <div class="flex items-center gap-2.5 mt-3">
            <div
              class="flex items-center gap-[5px] px-3 py-[5px] rounded-[20px] text-[12px] font-medium"
              :class="openclawAlive
                ? 'bg-[rgba(34,197,94,0.1)] text-[#16a34a]'
                : 'bg-[rgba(107,114,128,0.1)] text-[#6b7280]'"
            >
              <span
                class="w-1.5 h-1.5 rounded-full bg-current"
                :class="{ 'animate-[sp-pulse_1.5s_ease-in-out_infinite]': openclawAlive }"
              />
              {{ openclawAlive ? '已连接' : '未连接' }}
            </div>
            <button
              type="button"
              class="px-3 py-[5px] text-[12px] rounded-[6px] border border-[#e8e2f4] text-[#8a80a7] bg-transparent cursor-pointer transition hover:text-secondary hover:border-secondary/30 hover:bg-secondary/6 disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="checkingAlive"
              @click="refreshStatus"
            >
              {{ checkingAlive ? '检测中…' : '测试连接' }}
            </button>
          </div>
        </div>
      </div>

      <!-- ── 保存 ── -->
      <div class="flex items-center justify-between">
        <p class="text-[11px] text-[#c4bdd8] m-0">设置保存在本地，不会上传到任何服务器</p>
        <button
          type="button"
          class="flex items-center gap-[7px] px-5 py-2.5 text-[13px] font-medium text-white rounded-[10px] cursor-pointer transition"
          :class="saved
            ? 'bg-[linear-gradient(135deg,#22c55e_0%,#16a34a_100%)] shadow-[0_2px_10px_rgba(34,197,94,0.22)]'
            : 'bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] shadow-[0_2px_10px_rgba(95,71,206,0.22)] hover:shadow-[0_4px_16px_rgba(95,71,206,0.32)] hover:-translate-y-px active:translate-y-0'"
          @click="save"
        >
          <svg v-if="!saved" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
            <polyline points="17 21 17 13 7 13 7 21" />
            <polyline points="7 3 7 8 15 8" />
          </svg>
          <svg v-else width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12" />
          </svg>
          {{ saved ? '已保存' : '保存设置' }}
        </button>
      </div>

    </div>
  </div>
</template>

<style scoped>
.sp-body::-webkit-scrollbar { width: 4px; }
.sp-body::-webkit-scrollbar-thumb { background: rgba(95, 71, 206, 0.15); border-radius: 2px; }

@keyframes sp-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
</style>
