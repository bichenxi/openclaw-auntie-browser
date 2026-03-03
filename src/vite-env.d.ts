/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

interface ImportMetaEnv {
  readonly RENDERER_VITE_BASE_URL: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}

declare module "~icons/local/*" {
  import { FunctionalComponent, SVGAttributes } from "vue";
  const component: FunctionalComponent<SVGAttributes>;
  export default component;
}
