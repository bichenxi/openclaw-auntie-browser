<script setup lang="ts">
import { useTabsStore } from '@/stores/tabs'

const store = useTabsStore()

const onResize = useDebounceFn(() => {
  store.resizeAllWebviews()
}, 100)

onMounted(() => {
  window.addEventListener('resize', onResize)
})

onUnmounted(() => {
  window.removeEventListener('resize', onResize)
})
</script>

<template>
  <RouterView />
</template>
