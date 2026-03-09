import { defineStore } from 'pinia'
import { listen } from '@tauri-apps/api/event'

export type MessageType = 'thought' | 'tool' | 'user'
export interface Message {
  type: MessageType
  text: string
  streaming: boolean
}

export const useOpenclawStore = defineStore('openclaw', () => {
  const messages = ref<Message[]>([])
  const sending = ref(false)
  const sendError = ref('')

  let listenersStarted = false

  function startListeners() {
    if (listenersStarted) return
    listenersStarted = true

    listen<{ type: string; text: string }>('stream-item', (e) => {
      const payload = e.payload
      if (!payload?.type || !payload?.text) return
      const type: MessageType = payload.type === 'tool' ? 'tool' : 'thought'
      const last = messages.value[messages.value.length - 1]
      if (last && last.streaming && last.type === type) {
        last.text += payload.text
      } else {
        messages.value.push({ type, text: payload.text, streaming: true })
      }
    })

    listen('stream-done', () => {
      const last = messages.value[messages.value.length - 1]
      if (last && last.streaming) {
        last.streaming = false
      }
      sending.value = false
    })
  }

  return { messages, sending, sendError, startListeners }
})
