<script setup lang="ts">
import {
  listSkills,
  readSkillFile,
  writeSkillFile,
  createSkill,
  deleteSkill,
  deleteSkillFile,
  checkBuiltinSkillInstalled,
  installBuiltinSkill,
  type SkillMeta,
} from '@/api/skills'

// ── State ─────────────────────────────────────────────────────────────────────
const skills = ref<SkillMeta[]>([])
const selectedSkill = ref<SkillMeta | null>(null)
const selectedFile = ref<string>('SKILL.md')
const fileContent = ref('')
const loading = ref(false)
const saving = ref(false)
const saved = ref(false)
const error = ref('')

const showNewSkill = ref(false)
const newSkillName = ref('')
const creating = ref(false)

const showNewFile = ref(false)
const newFileName = ref('')

// Built-in skill
const builtinInstalled = ref(false)
const installing = ref(false)

// Confirm delete modal
const confirmDelete = ref<{ type: 'skill' | 'file'; name: string } | null>(null)

// Right-click context menu
const ctxMenu = ref<{ x: number; y: number; skill: SkillMeta } | null>(null)

// ── Load ──────────────────────────────────────────────────────────────────────
async function load() {
  loading.value = true
  error.value = ''
  try {
    const [skillList, installed] = await Promise.all([listSkills(), checkBuiltinSkillInstalled()])
    skills.value = skillList
    builtinInstalled.value = installed
    if (!selectedSkill.value && skills.value.length > 0) {
      await selectSkill(skills.value[0])
    } else if (selectedSkill.value) {
      const refreshed = skills.value.find(s => s.name === selectedSkill.value!.name)
      if (refreshed) selectedSkill.value = refreshed
      else { selectedSkill.value = null; fileContent.value = '' }
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

// ── Install builtin skill ─────────────────────────────────────────────────────
async function doInstallBuiltin() {
  installing.value = true
  error.value = ''
  try {
    await installBuiltinSkill()
    await load()
    // Auto-select the installed skill
    const skill = skills.value.find(s => s.name === 'claw-browser-control')
    if (skill) await selectSkill(skill)
  } catch (e) {
    error.value = String(e)
  } finally {
    installing.value = false
  }
}

async function selectSkill(skill: SkillMeta) {
  selectedSkill.value = skill
  selectedFile.value = skill.files.includes('SKILL.md') ? 'SKILL.md' : (skill.files[0] ?? '')
  await loadFile()
}

async function selectFile(filename: string) {
  selectedFile.value = filename
  await loadFile()
}

async function loadFile() {
  if (!selectedSkill.value || !selectedFile.value) return
  error.value = ''
  try {
    fileContent.value = await readSkillFile(selectedSkill.value.name, selectedFile.value)
  } catch (e) {
    error.value = String(e)
    fileContent.value = ''
  }
}

// ── Save ──────────────────────────────────────────────────────────────────────
async function save() {
  if (!selectedSkill.value || !selectedFile.value || saving.value) return
  saving.value = true
  error.value = ''
  try {
    await writeSkillFile(selectedSkill.value.name, selectedFile.value, fileContent.value)
    saved.value = true
    setTimeout(() => { saved.value = false }, 2000)
  } catch (e) {
    error.value = String(e)
  } finally {
    saving.value = false
  }
}

// ── Create skill ──────────────────────────────────────────────────────────────
async function doCreateSkill() {
  const name = newSkillName.value.trim()
  if (!name) return
  creating.value = true
  error.value = ''
  try {
    await createSkill(name)
    newSkillName.value = ''
    showNewSkill.value = false
    await load()
    const created = skills.value.find(s => s.name === name)
    if (created) await selectSkill(created)
  } catch (e) {
    error.value = String(e)
  } finally {
    creating.value = false
  }
}

// ── Create file ───────────────────────────────────────────────────────────────
async function doCreateFile() {
  if (!selectedSkill.value) return
  const name = newFileName.value.trim()
  if (!name) return
  error.value = ''
  try {
    await writeSkillFile(selectedSkill.value.name, name, '')
    newFileName.value = ''
    showNewFile.value = false
    await load()
    selectedFile.value = name
    fileContent.value = ''
  } catch (e) {
    error.value = String(e)
  }
}

// ── Delete ────────────────────────────────────────────────────────────────────
async function doDelete() {
  if (!confirmDelete.value) return
  error.value = ''
  try {
    if (confirmDelete.value.type === 'skill') {
      await deleteSkill(confirmDelete.value.name)
      if (selectedSkill.value?.name === confirmDelete.value.name) {
        selectedSkill.value = null
        fileContent.value = ''
      }
    } else if (selectedSkill.value) {
      await deleteSkillFile(selectedSkill.value.name, confirmDelete.value.name)
      if (selectedFile.value === confirmDelete.value.name) {
        fileContent.value = ''
        selectedFile.value = ''
      }
    }
    confirmDelete.value = null
    await load()
  } catch (e) {
    error.value = String(e)
    confirmDelete.value = null
  }
}

// ── Context menu ──────────────────────────────────────────────────────────────
function openCtxMenu(e: MouseEvent, skill: SkillMeta) {
  e.preventDefault()
  ctxMenu.value = { x: e.clientX, y: e.clientY, skill }
}

function closeCtxMenu() {
  ctxMenu.value = null
}

function ctxDeleteSkill() {
  if (!ctxMenu.value) return
  confirmDelete.value = { type: 'skill', name: ctxMenu.value.skill.name }
  closeCtxMenu()
}

// ── Keyboard shortcuts ────────────────────────────────────────────────────────
function handleEditorKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === 's') {
    e.preventDefault()
    save()
  }
  if (e.key === 'Tab') {
    e.preventDefault()
    const el = e.target as HTMLTextAreaElement
    const start = el.selectionStart
    const end = el.selectionEnd
    fileContent.value = fileContent.value.slice(0, start) + '  ' + fileContent.value.slice(end)
    nextTick(() => { el.selectionStart = el.selectionEnd = start + 2 })
  }
}

onMounted(load)
</script>

<template>
  <div class="h-full flex flex-col bg-[#f8f6ff] overflow-hidden">

    <!-- ── Header ─────────────────────────────────────────────────────────── -->
    <div class="shrink-0 flex items-center justify-between px-6 py-3.5 bg-white border-b border-[#e8e2f4]">
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-[10px] bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] flex items-center justify-center shadow-[0_2px_10px_rgba(95,71,206,0.25)] shrink-0">
          <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
            <line x1="16" y1="13" x2="8" y2="13" />
            <line x1="16" y1="17" x2="8" y2="17" />
          </svg>
        </div>
        <div class="flex flex-col">
          <span class="text-[16px] font-bold text-[#1f1f2e] leading-[1.2]">技能管理</span>
          <span class="text-[11px] text-[#9b8ec4] mt-px font-mono">~/.openclaw/workspace/skills</span>
        </div>
      </div>
      <button
        type="button"
        class="flex items-center gap-1.5 px-3.5 py-[7px] text-[13px] text-secondary bg-secondary/8 border border-secondary/20 rounded-[8px] cursor-pointer transition hover:bg-secondary/13"
        @click="load"
      >
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="23 4 23 10 17 10" /><polyline points="1 20 1 14 7 14" />
          <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
        </svg>
        刷新
      </button>
    </div>

    <!-- ── Error bar ──────────────────────────────────────────────────────── -->
    <div
      v-if="error"
      class="shrink-0 flex items-center gap-2 px-5 py-2 text-[12px] text-[#dc2626] bg-[rgba(239,68,68,0.06)] border-b border-[rgba(239,68,68,0.15)]"
    >
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" />
      </svg>
      {{ error }}
    </div>

    <!-- ── Body ───────────────────────────────────────────────────────────── -->
    <div class="flex-1 flex min-h-0 overflow-hidden">

      <!-- Sidebar -->
      <div class="shrink-0 w-[216px] flex flex-col border-r border-[#e8e2f4] bg-white">

        <!-- Sidebar header -->
        <div class="shrink-0 flex items-center justify-between px-3.5 pt-3 pb-2">
          <span class="text-[11px] font-semibold text-[#b8b0cc] uppercase tracking-[0.8px]">技能列表</span>
          <button
            type="button"
            class="flex items-center gap-1 px-2 py-0.5 rounded-[5px] text-[11px] text-secondary border border-secondary/20 bg-secondary/6 cursor-pointer transition hover:bg-secondary/12"
            @click="showNewSkill = !showNewSkill; newSkillName = ''"
          >
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.8" stroke-linecap="round">
              <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            新建
          </button>
        </div>

        <!-- New skill form -->
        <div v-if="showNewSkill" class="shrink-0 mx-3 mb-2 p-2.5 bg-[#f8f6ff] border border-[#e8e2f4] rounded-[8px] flex flex-col gap-2">
          <input
            v-model="newSkillName"
            type="text"
            placeholder="技能名称（英文）"
            class="w-full px-2.5 py-1.5 text-[12px] bg-white border border-[#e8e2f4] rounded-[6px] outline-none focus:border-secondary transition placeholder-[#c4bdd8]"
            @keydown.enter="doCreateSkill"
            @keydown.escape="showNewSkill = false"
          />
          <div class="flex gap-1.5">
            <button
              type="button"
              class="flex-1 py-[5px] text-[12px] font-medium text-white bg-secondary rounded-[6px] cursor-pointer transition hover:opacity-85 disabled:opacity-40"
              :disabled="creating || !newSkillName.trim()"
              @click="doCreateSkill"
            >{{ creating ? '创建中…' : '创建' }}</button>
            <button
              type="button"
              class="px-3 py-[5px] text-[12px] text-[#8a80a7] border border-[#e8e2f4] rounded-[6px] cursor-pointer transition hover:bg-[#f0ecfa]"
              @click="showNewSkill = false; newSkillName = ''"
            >取消</button>
          </div>
        </div>

        <!-- Skill list — right-click for more actions -->
        <div class="skill-list flex-1 overflow-y-auto px-2 pb-2">

          <!-- 内置技能：claw-browser-control -->
          <div class="mb-2 mt-1.5">
            <div class="px-1 mb-1">
              <span class="text-[10px] font-semibold text-[#b8b0cc] uppercase tracking-[0.8px]">内置技能</span>
            </div>
            <div
              class="flex items-center gap-2.5 px-2.5 py-2.5 rounded-[8px] transition select-none"
              :class="builtinInstalled
                ? (selectedSkill?.name === 'claw-browser-control' ? 'bg-secondary/8 cursor-pointer' : 'hover:bg-[#f5f2fc] cursor-pointer')
                : 'bg-[rgba(245,158,11,0.05)] border border-[rgba(245,158,11,0.2)]'"
              @click="builtinInstalled && selectSkill(skills.find(s => s.name === 'claw-browser-control')!)"
            >
              <img src="/logo.jpg" class="w-6 h-6 rounded-[6px] object-cover shrink-0 shadow-sm" alt="logo" />
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-1.5">
                  <span class="text-[12px] font-medium text-[#2d2b3d] truncate">claw-browser-control</span>
                  <span
                    class="shrink-0 px-1.5 py-px rounded-[4px] text-[9px] font-semibold tracking-[0.3px]"
                    :class="builtinInstalled
                      ? 'bg-[rgba(34,197,94,0.1)] text-[#16a34a]'
                      : 'bg-[rgba(245,158,11,0.12)] text-[#b45309]'"
                  >{{ builtinInstalled ? '已安装' : '未安装' }}</span>
                </div>
                <div class="text-[10px] text-[#b8b0cc] mt-0.5">大虾 · 浏览器控制专属技能</div>
              </div>
              <button
                v-if="!builtinInstalled"
                type="button"
                class="shrink-0 flex items-center gap-1 px-2 py-1 text-[11px] font-medium text-white bg-[linear-gradient(135deg,#7c5cfc,#5f47ce)] rounded-[6px] cursor-pointer transition hover:opacity-85 disabled:opacity-50 shadow-[0_1px_6px_rgba(95,71,206,0.3)]"
                :disabled="installing"
                @click.stop="doInstallBuiltin"
              >
                <svg v-if="!installing" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                  <polyline points="7 10 12 15 17 10" /><line x1="12" y1="15" x2="12" y2="3" />
                </svg>
                <span v-else class="w-2.5 h-2.5 border border-white/60 border-t-white rounded-full animate-spin" />
                {{ installing ? '…' : '安装' }}
              </button>
            </div>
          </div>

          <div class="mx-1 mb-2 border-t border-[#f0ecfa]" />
          <div class="px-1 mb-1">
            <span class="text-[10px] font-semibold text-[#b8b0cc] uppercase tracking-[0.8px]">自定义技能</span>
          </div>

          <div v-if="loading && skills.length === 0" class="py-6 text-center text-[12px] text-[#c4bdd8]">加载中…</div>
          <div v-else-if="skills.filter(s => s.name !== 'claw-browser-control').length === 0" class="py-4 text-center text-[12px] text-[#c4bdd8]">暂无自定义技能</div>

          <div
            v-for="skill in skills.filter(s => s.name !== 'claw-browser-control')"
            :key="skill.name"
            class="flex items-start gap-2.5 px-2.5 py-2.5 mt-0.5 rounded-[8px] cursor-pointer select-none transition"
            :class="selectedSkill?.name === skill.name
              ? 'bg-secondary/8'
              : 'hover:bg-[#f5f2fc]'"
            @click="selectSkill(skill)"
            @contextmenu="openCtxMenu($event, skill)"
          >
            <div
              class="mt-[4px] shrink-0 w-2 h-2 rounded-full transition"
              :class="selectedSkill?.name === skill.name ? 'bg-secondary' : 'bg-[#d4cfe8]'"
            />
            <div class="flex-1 min-w-0">
              <div
                class="text-[13px] font-medium truncate transition"
                :class="selectedSkill?.name === skill.name ? 'text-secondary' : 'text-[#2d2b3d]'"
              >{{ skill.name }}</div>
              <div v-if="skill.description" class="text-[11px] text-[#b8b0cc] mt-0.5 line-clamp-2 leading-[1.4]">{{ skill.description }}</div>
              <div v-if="skill.version" class="text-[10px] text-[#d4cfe8] mt-0.5">v{{ skill.version }} · {{ skill.files.length }} 个文件</div>
            </div>
          </div>
        </div>

        <!-- Sidebar hint -->
        <div class="shrink-0 px-4 py-2.5 border-t border-[#f5f2fc]">
          <p class="text-[10px] text-[#d4cfe8] m-0 leading-[1.5]">右键点击技能可删除</p>
        </div>
      </div>

      <!-- ── Editor area ─────────────────────────────────────────────────── -->
      <div class="flex-1 flex flex-col min-w-0 bg-white overflow-hidden">

        <!-- Empty state -->
        <div v-if="!selectedSkill" class="flex-1 flex flex-col items-center justify-center gap-3 text-[#c4bdd8]">
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" class="opacity-40">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
            <line x1="16" y1="13" x2="8" y2="13" /><line x1="16" y1="17" x2="8" y2="17" />
          </svg>
          <span class="text-[13px]">从左侧选择一个技能开始编辑</span>
        </div>

        <template v-else>

          <!-- File tabs — clean, no × in tab chrome -->
          <div class="shrink-0 flex items-center border-b border-[#e8e2f4] bg-[#faf8ff] px-3 gap-0.5 overflow-x-auto">
            <button
              v-for="f in selectedSkill.files"
              :key="f"
              type="button"
              class="flex items-center gap-1.5 px-3 py-2 text-[12px] cursor-pointer transition whitespace-nowrap border-b-2 -mb-px"
              :class="selectedFile === f
                ? 'text-secondary border-secondary font-medium bg-white'
                : 'text-[#8a80a7] border-transparent hover:text-[#4a4568] hover:bg-white/70'"
              @click="selectFile(f)"
            >
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="shrink-0 opacity-60">
                <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" />
                <polyline points="13 2 13 9 20 9" />
              </svg>
              {{ f }}
            </button>

            <!-- Add file -->
            <div class="flex items-center ml-1">
              <button
                v-if="!showNewFile"
                type="button"
                class="flex items-center gap-1 px-2 py-2 text-[12px] text-[#b8b0cc] hover:text-secondary cursor-pointer transition rounded-[5px] hover:bg-secondary/6"
                @click="showNewFile = true"
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                  <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
                </svg>
                新文件
              </button>
              <div v-else class="flex items-center gap-1.5 py-1.5">
                <input
                  v-model="newFileName"
                  type="text"
                  placeholder="文件名.md"
                  class="px-2 py-1 text-[12px] border border-[#e8e2f4] rounded-[5px] outline-none w-[130px] focus:border-secondary transition"
                  @keydown.enter="doCreateFile"
                  @keydown.escape="showNewFile = false; newFileName = ''"
                />
                <button type="button" class="px-2.5 py-1 text-[11px] text-white bg-secondary rounded-[5px] cursor-pointer hover:opacity-85 transition" @click="doCreateFile">确定</button>
                <button type="button" class="px-2 py-1 text-[11px] text-[#8a80a7] border border-[#e8e2f4] rounded-[5px] cursor-pointer hover:bg-[#f5f2fc] transition" @click="showNewFile = false; newFileName = ''">取消</button>
              </div>
            </div>
          </div>

          <!-- Description bar -->
          <div
            v-if="selectedSkill.description && selectedFile === 'SKILL.md'"
            class="shrink-0 px-4 py-1.5 text-[11px] text-[#a89ec4] bg-[#faf8ff] border-b border-[#f0ecfa] truncate"
          >
            <span class="text-[#c4bdd8] mr-1">描述</span>{{ selectedSkill.description }}
          </div>

          <!-- Editor -->
          <textarea
            v-model="fileContent"
            class="sk-editor flex-1 w-full px-5 py-4 text-[13px] font-mono leading-[1.75] resize-none outline-none border-none bg-white text-[#1f1f2e] box-border"
            placeholder="在此编辑技能文件内容…"
            spellcheck="false"
            @keydown="handleEditorKeydown"
          />

          <!-- Save bar: file info left, [delete file] + [save] right -->
          <div class="shrink-0 flex items-center justify-between px-4 py-2.5 border-t border-[#e8e2f4] bg-[#faf8ff]">
            <div class="flex items-center gap-1.5 text-[11px] text-[#c4bdd8] font-mono min-w-0 overflow-hidden">
              <span class="truncate">{{ selectedSkill.name }}<span class="mx-1 opacity-50">/</span>{{ selectedFile }}</span>
              <span class="shrink-0 opacity-60 ml-1">· ⌘S 保存</span>
            </div>
            <div class="flex items-center gap-2 shrink-0">
              <!-- Delete current file (only for non-SKILL.md) -->
              <button
                v-if="selectedFile && selectedFile !== 'SKILL.md'"
                type="button"
                class="flex items-center gap-1.5 px-3 py-[7px] text-[12px] text-[#b8b0cc] border border-[#e8e2f4] rounded-[8px] cursor-pointer transition hover:text-[#dc2626] hover:border-[rgba(239,68,68,0.3)] hover:bg-[rgba(239,68,68,0.04)]"
                @click="confirmDelete = { type: 'file', name: selectedFile }"
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="3 6 5 6 21 6" />
                  <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
                  <path d="M10 11v6M14 11v6M9 6V4h6v2" />
                </svg>
                删除文件
              </button>
              <!-- Save button -->
              <button
                type="button"
                class="flex items-center gap-[7px] px-4 py-[7px] text-[13px] font-medium text-white rounded-[8px] cursor-pointer transition disabled:opacity-50"
                :class="saved
                  ? 'bg-[linear-gradient(135deg,#22c55e,#16a34a)] shadow-[0_2px_8px_rgba(34,197,94,0.22)]'
                  : 'bg-[linear-gradient(135deg,#7c5cfc,#5f47ce)] shadow-[0_2px_8px_rgba(95,71,206,0.2)] hover:shadow-[0_4px_14px_rgba(95,71,206,0.3)] hover:-translate-y-px active:translate-y-0'"
                :disabled="saving"
                @click="save"
              >
                <svg v-if="!saved" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
                  <polyline points="17 21 17 13 7 13 7 21" /><polyline points="7 3 7 8 15 8" />
                </svg>
                <svg v-else width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="20 6 9 17 4 12" />
                </svg>
                {{ saved ? '已保存' : saving ? '保存中…' : '保存' }}
              </button>
            </div>
          </div>

        </template>
      </div>
    </div>

    <!-- ── Right-click context menu ──────────────────────────────────────── -->
    <Teleport to="body">
      <div
        v-if="ctxMenu"
        class="fixed inset-0 z-40"
        @click="closeCtxMenu"
        @contextmenu.prevent="closeCtxMenu"
      >
        <div
          class="absolute bg-white rounded-[10px] shadow-[0_8px_32px_rgba(0,0,0,0.14),0_0_0_1px_rgba(0,0,0,0.06)] py-1.5 z-50 min-w-[160px]"
          :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }"
          @click.stop
        >
          <!-- Skill name header -->
          <div class="px-3.5 py-1.5 border-b border-[#f0ecfa] mb-1">
            <span class="text-[11px] font-semibold text-[#9b8ec4] truncate block">{{ ctxMenu.skill.name }}</span>
          </div>
          <!-- Delete -->
          <button
            type="button"
            class="w-full flex items-center gap-2.5 px-3.5 py-2 text-[13px] text-[#dc2626] cursor-pointer transition hover:bg-[rgba(239,68,68,0.07)]"
            @click="ctxDeleteSkill"
          >
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="3 6 5 6 21 6" />
              <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
              <path d="M10 11v6M14 11v6M9 6V4h6v2" />
            </svg>
            删除技能
          </button>
        </div>
      </div>
    </Teleport>

    <!-- ── Confirm delete dialog ──────────────────────────────────────────── -->
    <Teleport to="body">
      <Transition name="dialog">
        <div
          v-if="confirmDelete"
          class="fixed inset-0 bg-black/25 backdrop-blur-[2px] flex items-center justify-center z-50"
          @click.self="confirmDelete = null"
        >
          <div class="bg-white rounded-[16px] p-6 w-[340px] shadow-[0_20px_60px_rgba(0,0,0,0.16)] flex flex-col gap-5">
            <div class="flex items-start gap-3.5">
              <div class="shrink-0 w-9 h-9 rounded-full bg-[rgba(239,68,68,0.1)] flex items-center justify-center mt-0.5">
                <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="#dc2626" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="3 6 5 6 21 6" />
                  <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
                  <path d="M10 11v6M14 11v6M9 6V4h6v2" />
                </svg>
              </div>
              <div class="flex flex-col gap-1">
                <span class="text-[15px] font-bold text-[#1f1f2e]">
                  {{ confirmDelete.type === 'skill' ? '删除技能' : '删除文件' }}
                </span>
                <span class="text-[13px] text-[#6b7280] leading-[1.5]">
                  {{ confirmDelete.type === 'skill'
                    ? `将删除技能「${confirmDelete.name}」的整个目录及所有文件，无法恢复。`
                    : `将删除文件「${confirmDelete.name}」，无法恢复。` }}
                </span>
              </div>
            </div>
            <div class="flex gap-2.5">
              <button
                type="button"
                class="flex-1 py-2 text-[13px] font-medium text-[#4a4568] bg-[#f5f2fc] rounded-[9px] cursor-pointer transition hover:bg-[#ede8f8]"
                @click="confirmDelete = null"
              >取消</button>
              <button
                type="button"
                class="flex-1 py-2 text-[13px] font-medium text-white bg-[#dc2626] rounded-[9px] cursor-pointer transition hover:bg-[#b91c1c] active:scale-[0.98]"
                @click="doDelete"
              >确认删除</button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

  </div>
</template>

<style scoped>
.skill-list::-webkit-scrollbar { width: 3px; }
.skill-list::-webkit-scrollbar-thumb { background: rgba(95,71,206,0.12); border-radius: 2px; }
.sk-editor::-webkit-scrollbar { width: 5px; }
.sk-editor::-webkit-scrollbar-thumb { background: rgba(95,71,206,0.1); border-radius: 3px; }

.dialog-enter-active, .dialog-leave-active { transition: opacity 0.15s ease; }
.dialog-enter-from, .dialog-leave-to { opacity: 0; }
.dialog-enter-active > div, .dialog-leave-active > div { transition: transform 0.15s ease, opacity 0.15s ease; }
.dialog-enter-from > div, .dialog-leave-to > div { transform: scale(0.96) translateY(4px); opacity: 0; }
</style>
