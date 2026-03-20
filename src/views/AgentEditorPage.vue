<script setup lang="ts">
import { useTabsStore } from '@/stores/tabs'
import { useAgentsStore } from '@/stores/agents'
import { readAgentFile, writeAgentFile } from '@/api/agents'

const tabsStore = useTabsStore()
const agentsStore = useAgentsStore()

// ─── 固定文件定义 ──────────────────────────────────────────────────────────

const FILE_DEFS = [
  {
    name: 'IDENTITY.md',
    label: 'IDENTITY',
    desc: '定义智能体的角色、名称和身份',
    placeholder: `# 角色定义

你是一个专注于 [领域] 的智能助手，擅长 [技能]。

## 性格特点

- 专业、严谨
- 简洁直接，避免废话

## 工作方式

在回答问题时，优先 [方式]。`,
  },
  {
    name: 'USER.md',
    label: 'USER',
    desc: '告诉智能体关于你的信息和偏好',
    placeholder: `# 用户信息

## 基本信息

- 职业：[你的职业]
- 技术背景：[技术栈/经验]

## 偏好

- 喜欢简洁的回答
- 代码示例优先用 [语言]`,
  },
  {
    name: 'SOUL.md',
    label: 'SOUL',
    desc: '设定智能体的行为准则和底线',
    placeholder: `# 行为准则

## 必须遵守

- 始终诚实，不捏造信息
- 不确定时明确说"不确定"

## 工作风格

- 回答要有条理
- 代码要加必要注释`,
  },
]

// ─── 状态 ─────────────────────────────────────────────────────────────────

const work = computed(() => agentsStore.editingWork)

/** 当前选中的文件名 */
const activeFile = ref('IDENTITY.md')
/** 每个文件的内容缓存 */
const contents = ref<Record<string, string>>({})
/** 记录哪些文件有未保存修改 */
const dirty = ref<Record<string, boolean>>({})
/** 当前编辑区内容（双向绑定） */
const editorContent = ref('')

const loading = ref(false)
const saving = ref(false)
const saveSuccess = ref(false)
const saveError = ref<string | null>(null)

let saveSuccessTimer: ReturnType<typeof setTimeout> | null = null

// ─── 加载 ──────────────────────────────────────────────────────────────────

async function loadAll() {
  loading.value = true
  saveError.value = null
  const results = await Promise.allSettled(
    FILE_DEFS.map(f => readAgentFile(work.value, f.name))
  )
  FILE_DEFS.forEach((f, i) => {
    const r = results[i]
    contents.value[f.name] = r.status === 'fulfilled' ? r.value : ''
    dirty.value[f.name] = false
  })
  editorContent.value = contents.value[activeFile.value] ?? ''
  loading.value = false
}

watch(() => work.value, (w) => {
  if (w) loadAll()
}, { immediate: true })

// ─── 文件切换 ──────────────────────────────────────────────────────────────

/** 切换文件前检查未保存，确认后切换 */
function trySwitch(filename: string) {
  if (filename === activeFile.value) return
  if (dirty.value[activeFile.value]) {
    pendingSwitchTarget.value = filename
    showDirtyConfirm.value = true
  } else {
    doSwitch(filename)
  }
}

function doSwitch(filename: string) {
  // 把当前编辑内容同步回缓存
  contents.value[activeFile.value] = editorContent.value
  activeFile.value = filename
  editorContent.value = contents.value[filename] ?? ''
  showDirtyConfirm.value = false
  pendingSwitchTarget.value = ''
}

const showDirtyConfirm = ref(false)
const pendingSwitchTarget = ref('')

function confirmDiscard() {
  dirty.value[activeFile.value] = false
  doSwitch(pendingSwitchTarget.value)
}

// ─── 编辑器变更 ────────────────────────────────────────────────────────────

function onInput() {
  dirty.value[activeFile.value] = editorContent.value !== contents.value[activeFile.value]
}

// ─── 保存 ──────────────────────────────────────────────────────────────────

async function save() {
  if (saving.value) return
  saving.value = true
  saveError.value = null
  try {
    await writeAgentFile(work.value, activeFile.value, editorContent.value)
    contents.value[activeFile.value] = editorContent.value
    dirty.value[activeFile.value] = false
    if (saveSuccessTimer) clearTimeout(saveSuccessTimer)
    saveSuccess.value = true
    saveSuccessTimer = setTimeout(() => { saveSuccess.value = false }, 2000)
  } catch (e: unknown) {
    saveError.value = (e as Error)?.message ?? String(e)
  } finally {
    saving.value = false
  }
}

// Cmd+S / Ctrl+S
function handleKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === 's') {
    e.preventDefault()
    save()
  }
  // Tab 键缩进
  if (e.key === 'Tab') {
    e.preventDefault()
    const el = e.target as HTMLTextAreaElement
    const start = el.selectionStart
    const end = el.selectionEnd
    editorContent.value = editorContent.value.slice(0, start) + '  ' + editorContent.value.slice(end)
    nextTick(() => {
      el.selectionStart = el.selectionEnd = start + 2
    })
    dirty.value[activeFile.value] = true
  }
}

// ─── 返回 ──────────────────────────────────────────────────────────────────

function goBack() {
  // 有任何未保存变更时先提示
  const hasDirty = FILE_DEFS.some(f => dirty.value[f.name])
  if (hasDirty) {
    pendingBack.value = true
    showDirtyConfirm.value = true
  } else {
    doBack()
  }
}

const pendingBack = ref(false)

function confirmDiscardAndBack() {
  FILE_DEFS.forEach(f => { dirty.value[f.name] = false })
  doBack()
}

function doBack() {
  showDirtyConfirm.value = false
  pendingBack.value = false
  agentsStore.editingWork = ''
  tabsStore.switchToSpecialView('agents')
}

// ─── 当前文件定义 ──────────────────────────────────────────────────────────

const activeFileDef = computed(() => FILE_DEFS.find(f => f.name === activeFile.value)!)
const hasDirtyAny = computed(() => FILE_DEFS.some(f => dirty.value[f.name]))
</script>

<template>
  <div class="flex flex-col h-full bg-white">
    <!-- 顶部导航 -->
    <div class="flex items-center gap-0 px-5 py-3 border-b border-[#e8e2f4] bg-white shrink-0">
      <!-- 返回按钮 -->
      <button
        class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[#9b8ec4] hover:text-secondary hover:bg-secondary/6 transition cursor-pointer bg-transparent border-none text-[13px] font-medium"
        @click="goBack"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="15 18 9 12 15 6" />
        </svg>
        智能体
      </button>
      <!-- 面包屑分隔 -->
      <svg class="text-[#d4cdf4] mx-1" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="9 18 15 12 9 6" /></svg>
      <!-- 当前智能体名 -->
      <span class="text-[13px] font-semibold text-[#1f1f2e]">{{ work }}</span>

      <div class="flex-1" />

      <!-- 保存状态 -->
      <Transition name="fade-badge">
        <span
          v-if="saveSuccess"
          class="flex items-center gap-1.5 px-3 py-1 rounded-full bg-emerald-50 text-emerald-600 border border-emerald-200 text-[12px] font-medium"
        >
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12" /></svg>
          已保存
        </span>
      </Transition>
      <span
        v-if="hasDirtyAny && !saveSuccess"
        class="text-[11px] text-[#9b8ec4] mr-2"
      >有未保存的修改</span>
    </div>

    <!-- 内容区 -->
    <div class="flex-1 min-h-0 flex flex-col">
      <!-- 加载中 -->
      <div v-if="loading" class="flex-center flex-1">
        <span class="w-7 h-7 border-[2.5px] border-secondary border-t-transparent rounded-full animate-spin" />
      </div>

      <template v-else>
        <!-- 文件 Tab 栏 -->
        <div class="flex items-stretch gap-0 px-6 pt-4 pb-0 border-b border-[#e8e2f4] shrink-0">
          <button
            v-for="f in FILE_DEFS"
            :key="f.name"
            type="button"
            class="flex items-center gap-2 px-4 py-2.5 text-[13px] font-medium border-b-2 transition cursor-pointer bg-transparent border-none mr-1 rounded-t-lg"
            :class="activeFile === f.name
              ? 'text-secondary border-secondary bg-secondary/5'
              : 'text-[#9b8ec4] border-transparent hover:text-[#6b5f8a] hover:bg-[#f5f3ff]'"
            @click="trySwitch(f.name)"
          >
            <span>{{ f.label }}</span>
            <!-- 未保存小圆点 -->
            <span
              v-if="dirty[f.name]"
              class="w-1.5 h-1.5 rounded-full bg-secondary shrink-0"
            />
          </button>
        </div>

        <!-- 文件描述行 -->
        <div class="flex items-center gap-2 px-6 py-2.5 border-b border-[#f0ecfa] bg-[#faf9ff] shrink-0">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#9b8ec4" stroke-width="2">
            <circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" />
          </svg>
          <span class="text-[12px] text-[#9b8ec4]">{{ activeFileDef.desc }}</span>
          <div class="flex-1" />
          <span class="text-[11px] text-[#c4bdd8] font-mono">{{ work }}/{{ activeFile }}</span>
        </div>

        <!-- 编辑器 -->
        <div class="flex-1 min-h-0 relative">
          <textarea
            v-model="editorContent"
            class="absolute inset-0 w-full h-full resize-none outline-none border-none px-6 py-5 font-mono text-[13px] text-[#1f1f2e] leading-[1.75] bg-white"
            :placeholder="activeFileDef.placeholder"
            spellcheck="false"
            @input="onInput"
            @keydown="handleKeydown"
          />
        </div>

        <!-- 底部保存栏 -->
        <div class="flex items-center justify-between px-6 py-3 border-t border-[#e8e2f4] bg-[#faf9ff] shrink-0">
          <span class="text-[11px] text-[#c4bdd8]">
            <kbd class="bg-white border border-[#e8e2f4] rounded px-1.5 py-0.5 text-[10px] text-[#9b8ec4]">⌘S</kbd>
            保存
          </span>

          <div class="flex items-center gap-2">
            <div v-if="saveError" class="text-[11px] text-red-500 flex items-center gap-1">
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" /></svg>
              {{ saveError }}
            </div>
            <button
              type="button"
              class="flex items-center gap-1.5 px-4 py-1.5 text-[13px] font-medium text-white rounded-[8px] cursor-pointer transition bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] shadow-[0_2px_6px_rgba(95,71,206,0.18)] hover:shadow-[0_3px_10px_rgba(95,71,206,0.28)] active:scale-[0.97] disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="saving || !dirty[activeFile]"
              @click="save"
            >
              <span v-if="saving" class="w-3 h-3 border-2 border-white border-t-transparent rounded-full animate-spin" />
              {{ saving ? '保存中…' : '保存' }}
            </button>
          </div>
        </div>
      </template>
    </div>
  </div>

  <!-- 未保存确认弹窗 -->
  <Teleport to="body">
    <Transition name="overlay">
      <div
        v-if="showDirtyConfirm"
        class="fixed inset-0 z-[9999] flex items-center justify-center"
      >
        <div class="absolute inset-0 bg-black/25 backdrop-blur-sm" @click="showDirtyConfirm = false; pendingBack = false" />
        <div class="relative bg-white rounded-2xl shadow-2xl w-full max-w-[360px] mx-4 overflow-hidden">
          <div class="px-6 pt-6 pb-4">
            <div class="flex items-start gap-3">
              <div class="w-8 h-8 rounded-full bg-amber-50 border border-amber-200 flex-center shrink-0 mt-0.5">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#d97706" stroke-width="2.5">
                  <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
                  <line x1="12" y1="9" x2="12" y2="13" /><line x1="12" y1="17" x2="12.01" y2="17" />
                </svg>
              </div>
              <div>
                <div class="text-[14px] font-semibold text-[#1f1f2e]">有未保存的修改</div>
                <div class="text-[12px] text-[#9b8ec4] mt-1">{{ pendingBack ? '返回后修改将丢失，是否继续？' : '切换文件后修改将丢失，是否继续？' }}</div>
              </div>
            </div>
          </div>
          <div class="flex gap-3 justify-end px-6 py-4 bg-[#faf9ff] border-t border-[#f0ecfa]">
            <button
              type="button"
              class="px-4 py-2 text-[13px] rounded-[8px] border border-[#e8e2f4] text-[#6b5f8a] bg-white hover:bg-[#f5f3ff] cursor-pointer transition"
              @click="showDirtyConfirm = false; pendingBack = false"
            >
              取消
            </button>
            <button
              type="button"
              class="px-4 py-2 text-[13px] font-medium text-white rounded-[8px] cursor-pointer transition bg-amber-500 hover:bg-amber-600 shadow-[0_2px_6px_rgba(217,119,6,0.2)]"
              @click="pendingBack ? confirmDiscardAndBack() : confirmDiscard()"
            >
              放弃修改
            </button>
            <button
              type="button"
              class="px-4 py-2 text-[13px] font-medium text-white rounded-[8px] cursor-pointer transition bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] shadow-[0_2px_6px_rgba(95,71,206,0.18)]"
              @click="save().then(() => { showDirtyConfirm = false; if (pendingBack) doBack(); else doSwitch(pendingSwitchTarget) })"
            >
              先保存
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
textarea::placeholder {
  color: #c4bdd8;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}

.fade-badge-enter-active,
.fade-badge-leave-active {
  transition: opacity 0.3s ease, transform 0.3s ease;
}
.fade-badge-enter-from,
.fade-badge-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

.overlay-enter-active,
.overlay-leave-active { transition: opacity 0.15s ease; }
.overlay-enter-active > div:last-child,
.overlay-leave-active > div:last-child { transition: transform 0.15s ease, opacity 0.15s ease; }
.overlay-enter-from,
.overlay-leave-to { opacity: 0; }
.overlay-enter-from > div:last-child { transform: scale(0.97) translateY(8px); opacity: 0; }
.overlay-leave-to > div:last-child { transform: scale(0.97) translateY(8px); opacity: 0; }
</style>
