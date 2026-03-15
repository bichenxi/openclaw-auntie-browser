<script setup lang="ts">
import { useSettingsStore } from '@/stores/settings'
import { useTabsStore } from '@/stores/tabs'
import { useOnboardStore } from '@/stores/onboard'
import { checkOpenclawAlive } from '@/api/openclaw'
import { getOpenclawGatewayToken } from '@/api/skills'
import { checkAndFixGatewayConfig, restartOpenclawGateway, type GatewayConfigStatus } from '@/api/gateway'
import { fullUninstall, type UninstallResult } from '@/api/installer'

const settings = useSettingsStore()
const onboardStore = useOnboardStore()
const store = useTabsStore()

const tokenInput = ref(settings.bearerToken)
const sessionKeyInput = ref(settings.sessionKey)
const baseUrlInput = ref(settings.baseUrl)
const saved = ref(false)
const openclawAlive = ref(false)
const checkingAlive = ref(false)
const fetchingToken = ref(false)
const fetchTokenError = ref('')

// ── Gateway 配置检测 ────────────────────────────────────────
const gatewayStatus = ref<GatewayConfigStatus | null>(null)
const checkingGateway = ref(false)
const restartingGateway = ref(false)
const restartError = ref('')
const restartSuccess = ref(false)

// ── 一键卸载 ────────────────────────────────────────────────
const uninstalling = ref(false)
const uninstallConfirm = ref(false)
const uninstallResult = ref<UninstallResult | null>(null)

async function doUninstall() {
  uninstalling.value = true
  uninstallResult.value = null
  try {
    uninstallResult.value = await fullUninstall()
  } catch (e: any) {
    uninstallResult.value = {
      success: false,
      steps: [{ name: '卸载失败', ok: false, detail: e?.message ?? String(e) }],
    }
  } finally {
    uninstalling.value = false
    uninstallConfirm.value = false
  }
}

async function checkGatewayConfig() {
  checkingGateway.value = true
  gatewayStatus.value = null
  restartError.value = ''
  restartSuccess.value = false
  try {
    gatewayStatus.value = await checkAndFixGatewayConfig()
  } finally {
    checkingGateway.value = false
  }
}

async function doRestartGateway() {
  restartingGateway.value = true
  restartError.value = ''
  restartSuccess.value = false
  try {
    await restartOpenclawGateway()
    restartSuccess.value = true
    setTimeout(() => { restartSuccess.value = false }, 3000)
  } catch (e: any) {
    restartError.value = e?.message ?? String(e)
  } finally {
    restartingGateway.value = false
  }
}

watch(() => settings.sessionKey, (val) => {
  sessionKeyInput.value = val
})

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
    // 获取成功说明 18789 在线，Base URL 为空时填入默认值
    if (!baseUrlInput.value.trim()) {
      baseUrlInput.value = 'http://127.0.0.1:18789'
    }
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

      <!-- ── OpenClaw 可视化配置 ── -->
      <div class="bg-white border border-[#e8e2f4] rounded-[12px] overflow-hidden mb-4">
        <div class="flex items-center gap-[7px] px-5 py-3.5 border-b border-[#f0ecfa]">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-secondary">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
          </svg>
          <span class="text-[13px] font-semibold text-secondary">OpenClaw 可视化配置</span>
        </div>
        <div class="p-5 flex flex-col gap-3">
          <p class="text-[11px] text-[#9b8ec4] m-0 leading-[1.6]">
            运行 <code class="bg-[#f5f3ff] text-secondary px-1 py-px rounded text-[10px]">openclaw onboard</code> 进行初始化配置，完成后自动启动网关。
          </p>
          <div class="flex gap-2">
            <button
              type="button"
              class="flex-1 flex items-center justify-center gap-2 px-3.5 py-2.5 text-[12px] font-medium rounded-[8px] border cursor-pointer transition border-secondary/30 text-secondary bg-secondary/6 hover:bg-secondary/12"
              @click="onboardStore.open()"
            >
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="4 17 10 11 4 5" />
                <line x1="12" y1="19" x2="20" y2="19" />
              </svg>
              配置向导（内嵌终端）
            </button>
            <button
              type="button"
              class="flex-1 flex items-center justify-center gap-2 px-3.5 py-2.5 text-[12px] font-medium rounded-[8px] border cursor-pointer transition border-secondary/30 text-secondary bg-secondary/6 hover:bg-secondary/12"
              @click="onboardStore.openWizard()"
            >
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
                <line x1="3" y1="9" x2="21" y2="9" />
                <line x1="9" y1="21" x2="9" y2="9" />
              </svg>
              配置向导（可视化配置）
            </button>
          </div>
          <p class="text-[10px] text-[#c4bdd8] m-0 leading-[1.5]">
            内嵌终端：直接与 TUI 实时交互（仅 macOS / Linux）。可视化配置：通过表单步骤操作，支持全平台。
          </p>
        </div>
      </div>

      <!-- ── 会话配置 ── -->
      <div class="bg-white border border-[#e8e2f4] rounded-[12px] overflow-hidden mb-4">
        <div class="flex items-center gap-[7px] px-5 py-3.5 border-b border-[#f0ecfa]">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-secondary">
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
          </svg>
          <span class="text-[13px] font-semibold text-secondary">会话配置</span>
        </div>
        <div class="p-5">
          <label class="block text-[12px] font-semibold text-[#4a4568] mb-1.5">OPENCLAW_SESSION_KEY</label>
          <p class="text-[11px] text-[#9b8ec4] m-0 mb-2.5">会话标识，用于区分不同的对话上下文。技能同步后会自动刷新。</p>
          <input
            v-model="sessionKeyInput"
            type="text"
            class="w-full px-3.5 py-2.5 text-[13px] font-[inherit] border-[1.5px] border-[#e8e2f4] rounded-[8px] outline-none box-border text-[#1f1f2e] bg-[#fafafa] transition placeholder-[#c4bdd8] focus:border-[#7c5cfc] focus:bg-white focus:shadow-[0_0_0_3px_rgba(95,71,206,0.08)]"
            placeholder="agent:main:main"
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

      <!-- ── Gateway 配置检测 ── -->
      <div class="bg-white border border-[#e8e2f4] rounded-[12px] overflow-hidden mb-4">
        <div class="flex items-center gap-[7px] px-5 py-3.5 border-b border-[#f0ecfa]">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-secondary">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
          </svg>
          <span class="text-[13px] font-semibold text-secondary">Gateway 配置检测</span>
        </div>
        <div class="p-5">
          <p class="text-[11px] text-[#9b8ec4] m-0 mb-3 leading-[1.6]">
            检测 <code class="bg-[#f5f3ff] text-secondary px-1 py-px rounded text-[10px]">~/.openclaw/openclaw.json</code>
            中 <code class="bg-[#f5f3ff] text-secondary px-1 py-px rounded text-[10px]">gateway.controlUi</code> 和
            <code class="bg-[#f5f3ff] text-secondary px-1 py-px rounded text-[10px]">gateway.http.endpoints</code>
            配置是否完整，缺失时自动补写。
          </p>

          <!-- 检测结果 -->
          <div
            v-if="gatewayStatus"
            class="flex items-start gap-2 px-3.5 py-2.5 rounded-[8px] text-[12px] mb-3 leading-relaxed"
            :class="gatewayStatus.error
              ? 'bg-[rgba(239,68,68,0.06)] border border-[rgba(239,68,68,0.18)] text-[#dc2626]'
              : gatewayStatus.already_ok
                ? 'bg-[rgba(34,197,94,0.07)] border border-[rgba(34,197,94,0.2)] text-[#15803d]'
                : 'bg-[rgba(95,71,206,0.06)] border border-secondary/18 text-secondary'"
          >
            <svg v-if="gatewayStatus.error" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mt-px shrink-0">
              <circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
            <svg v-else-if="gatewayStatus.already_ok" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mt-px shrink-0">
              <polyline points="20 6 9 17 4 12" />
            </svg>
            <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mt-px shrink-0">
              <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
            </svg>
            <span>{{
              gatewayStatus.error
                ? gatewayStatus.error
                : gatewayStatus.already_ok
                  ? '配置完整，无需修改'
                  : '配置已补写完成，建议重启 gateway 使配置生效'
            }}</span>
          </div>

          <!-- 重启结果 -->
          <div v-if="restartSuccess" class="flex items-center gap-2 px-3.5 py-2.5 rounded-[8px] text-[12px] mb-3 bg-[rgba(34,197,94,0.07)] border border-[rgba(34,197,94,0.2)] text-[#15803d]">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="20 6 9 17 4 12" />
            </svg>
            Gateway 已成功重启
          </div>
          <div v-if="restartError" class="flex items-start gap-2 px-3.5 py-2.5 rounded-[8px] text-[12px] mb-3 bg-[rgba(239,68,68,0.06)] border border-[rgba(239,68,68,0.18)] text-[#dc2626]">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mt-px shrink-0">
              <circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
            <div>
              <div class="mb-1">自动重启失败，请手动执行：</div>
              <code class="block bg-[rgba(239,68,68,0.08)] px-2 py-1 rounded text-[11px] font-mono">openclaw gateway restart</code>
              <div class="mt-1 text-[11px] opacity-70">{{ restartError }}</div>
            </div>
          </div>

          <!-- 操作按钮 -->
          <div class="flex flex-wrap items-center gap-2">
            <button
              type="button"
              class="flex items-center gap-1.5 px-3.5 py-2 text-[12px] font-medium rounded-[8px] border cursor-pointer transition disabled:opacity-50 disabled:cursor-not-allowed"
              :class="checkingGateway
                ? 'border-[#e8e2f4] text-[#8a80a7] bg-transparent'
                : 'border-secondary/30 text-secondary bg-secondary/6 hover:bg-secondary/12'"
              :disabled="checkingGateway"
              @click="checkGatewayConfig"
            >
              <svg v-if="!checkingGateway" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
              </svg>
              <span v-else class="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin" />
              {{ checkingGateway ? '检测中…' : '检测并修复配置' }}
            </button>
            <button
              v-if="gatewayStatus && !gatewayStatus.error && !gatewayStatus.already_ok"
              type="button"
              class="flex items-center gap-1.5 px-3.5 py-2 text-[12px] font-medium rounded-[8px] border cursor-pointer transition disabled:opacity-50 disabled:cursor-not-allowed border-[rgba(34,197,94,0.35)] text-[#15803d] bg-[rgba(34,197,94,0.06)] hover:bg-[rgba(34,197,94,0.12)]"
              :disabled="restartingGateway"
              @click="doRestartGateway"
            >
              <svg v-if="!restartingGateway" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="23 4 23 10 17 10" /><polyline points="1 20 1 14 7 14" />
                <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
              </svg>
              <span v-else class="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin" />
              {{ restartingGateway ? '重启中…' : '重启 Gateway' }}
            </button>
          </div>
        </div>
      </div>

      <!-- ── 危险操作：一键卸载 ── -->
      <div class="bg-white border border-[rgba(239,68,68,0.25)] rounded-[12px] overflow-hidden mb-4">
        <div class="flex items-center gap-[7px] px-5 py-3.5 border-b border-[rgba(239,68,68,0.12)]">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-accent">
            <polyline points="3 6 5 6 21 6" />
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
            <line x1="10" y1="11" x2="10" y2="17" />
            <line x1="14" y1="11" x2="14" y2="17" />
          </svg>
          <span class="text-[13px] font-semibold text-accent">一键彻底卸载</span>
        </div>
        <div class="p-5">
          <p class="text-[11px] text-[#9b8ec4] m-0 mb-3 leading-[1.6]">
            彻底移除 OpenClaw npm 包、<code class="bg-[#f5f3ff] text-secondary px-1 py-px rounded text-[10px]">~/.openclaw</code> 配置目录、内置 fnm 及 Node.js、shell 配置中添加的 PATH。此操作不可逆。
          </p>

          <!-- 卸载结果 -->
          <div v-if="uninstallResult" class="mb-3">
            <div
              v-for="step in uninstallResult.steps"
              :key="step.name"
              class="flex items-center gap-2 text-[12px] py-1"
            >
              <svg v-if="step.ok" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#16a34a" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="shrink-0">
                <polyline points="20 6 9 17 4 12" />
              </svg>
              <svg v-else width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#dc2626" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="shrink-0">
                <circle cx="12" cy="12" r="10" /><line x1="15" y1="9" x2="9" y2="15" /><line x1="9" y1="9" x2="15" y2="15" />
              </svg>
              <span :class="step.ok ? 'text-[#15803d]' : 'text-[#dc2626]'">{{ step.name }}</span>
              <span class="text-[#c4bdd8] text-[11px]">{{ step.detail }}</span>
            </div>
            <div
              class="flex items-center gap-2 mt-2 px-3 py-2 rounded-[8px] text-[12px]"
              :class="uninstallResult.success
                ? 'bg-[rgba(34,197,94,0.07)] text-[#15803d]'
                : 'bg-[rgba(239,68,68,0.06)] text-[#dc2626]'"
            >
              <svg v-if="uninstallResult.success" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="20 6 9 17 4 12" />
              </svg>
              {{ uninstallResult.success ? '卸载完成，请重新打开终端使 PATH 变更生效。' : '部分步骤失败，请检查日志。' }}
            </div>
          </div>

          <!-- 确认流程 -->
          <div v-if="!uninstallConfirm" class="flex items-center gap-3">
            <button
              type="button"
              class="flex items-center gap-1.5 px-3.5 py-2 text-[12px] font-medium rounded-[8px] border cursor-pointer transition border-[rgba(239,68,68,0.3)] text-accent bg-[rgba(239,68,68,0.04)] hover:bg-[rgba(239,68,68,0.1)] disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="uninstalling"
              @click="uninstallConfirm = true; uninstallResult = null"
            >
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="3 6 5 6 21 6" />
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
              </svg>
              一键卸载 OpenClaw + Node.js + fnm
            </button>
          </div>
          <div v-else class="flex items-center gap-3">
            <span class="text-[12px] text-accent font-medium">确定要彻底卸载吗？此操作不可逆！</span>
            <button
              type="button"
              class="flex items-center gap-1.5 px-3.5 py-2 text-[12px] font-medium rounded-[8px] border cursor-pointer transition border-accent text-white bg-accent hover:bg-[#dc2626] disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="uninstalling"
              @click="doUninstall"
            >
              <span v-if="uninstalling" class="w-3 h-3 border-2 border-white border-t-transparent rounded-full animate-spin" />
              {{ uninstalling ? '卸载中…' : '确认卸载' }}
            </button>
            <button
              type="button"
              class="px-3.5 py-2 text-[12px] rounded-[8px] border border-[#e8e2f4] text-[#8a80a7] bg-transparent cursor-pointer transition hover:bg-[#f5f3ff]"
              :disabled="uninstalling"
              @click="uninstallConfirm = false"
            >
              取消
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
