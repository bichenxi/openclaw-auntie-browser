declare module 'virtual:generated-layouts' {
  import { RouteRecordRaw } from 'vue-router'
  export function setupLayouts(routes: RouteRecordRaw[]): RouteRecordRaw[]
}

declare module 'vue-router/auto-routes' {
  import { RouteRecordRaw } from 'vue-router'
  export const routes: RouteRecordRaw[]
} 