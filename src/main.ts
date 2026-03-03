import { createApp } from "vue";
import App from "./App.vue";
import { setupLayouts } from 'virtual:generated-layouts'
import { routes } from 'vue-router/auto-routes'
import { createRouter, createWebHistory } from 'vue-router'
import 'uno.css'
import './styles/main.css'
import { createPinia } from 'pinia'

const routeList = setupLayouts(routes)

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: routeList,
})

const pinia = createPinia()

const app = createApp(App)
app.use(router)
app.use(pinia)
app.mount("#app")
