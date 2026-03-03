import presetLegacyCompat from '@unocss/preset-legacy-compat'
import {
  defineConfig,
  presetTypography,
  presetUno,
  transformerDirectives,
  transformerVariantGroup,
} from 'unocss'

/**
 * 将mobile:xxx/pc:xxx/app:xxx等转换为对应的类名
 * 例如：
 * mobile:xxx 转换为 ".is-mobile .xxx"
 * pc:xxx 转换为 ".is-pc .xxx"
 * app:xxx 转换为 ".is-app .xxx"
 *
 * 通常情况无需使用，仅当需要根据特定环境设置样式时使用
 * 需通过js等自行设置body或根节点的对应class（如"is-mobile"）
 * 例如：mobile:text-xl 或 pc:text-xl 或 app:text-xl
 */
function transformerEnv() {
  return {
    name: 'transformer-env',
    transform(code: any) {
      // 自定义不同的环境前缀
      return code.replace(/(pc|app):(\w+)/g, (match: string, p1: string, p2: string) => {
        return `.is-${p1} .${p2}`
      })
    },
  }
}

export default defineConfig({
  shortcuts: [
    ['btn', 'py-1 px-4 text-[14px] leading-[14px] rounded-[6px] border border-[#ddd] border-solid cursor-pointer transition text-gray-600 disabled:opacity-60 disabled:!cursor-not-allowed hover:[&:not(:disabled)]:text-primary hover:[&:not(:disabled)]:border-primary active:[&:not(:disabled)]:scale-95'],
    ['btn-plain', 'py-2 px-4 text-[14px] leading-[14px] rounded-[6px] border border-transparent border-solid cursor-pointer transition bg-[#eee] dark:bg-[#ffffff0d] dark:border-transparent disabled:opacity-60 disabled:!cursor-not-allowed hover:[&:not(:disabled)]:text-white hover:[&:not(:disabled)]:bg-primary/80 hover:[&:not(:disabled)]:border-primary active:[&:not(:disabled)]:scale-95'],
  ],
  presets: [
    presetUno(),
    presetTypography(),
    // @ts-ignore
    presetLegacyCompat({
      commaStyleColorFunction: true,
      legacyColorSpace: true,
    }),
  ],
  theme: {
    colors: {
      primary: {
        DEFAULT: '#1a00ff',
        50: '#f0f0ff',
        100: '#e0e0ff',
        200: '#c7c7ff',
        300: '#a3a3ff',
        400: '#7a7aff',
        500: '#1a00ff', // 主蓝色
        600: '#1600e6',
        700: '#1200cc',
        800: '#0f00b3',
        900: '#0c0099',
      },
      secondary: {
        DEFAULT: '#5f47ce',
        50: '#f7f5ff',
        100: '#efebff',
        200: '#ddd6ff',
        300: '#c4b8ff',
        400: '#a291ff',
        500: '#5f47ce', // 主紫色
        600: '#5439b8',
        700: '#482ca2',
        800: '#3d238c',
        900: '#321d76',
      },
      accent: {
        DEFAULT: '#ef4444',
        50: '#fef2f2',
        100: '#fee2e2',
        200: '#fecaca',
        300: '#fca5a5',
        400: '#f87171',
        500: '#ef4444', // 主红色
        600: '#dc2626',
        700: '#b91c1c',
        800: '#991b1b',
        900: '#7f1d1d',
      },
      neutral: {
        DEFAULT: '#6b7280',
        50: '#f9fafb',
        100: '#f3f4f6',
        200: '#e5e7eb',
        300: '#d1d5db',
        400: '#9ca3af',
        500: '#6b7280', // 主灰色
        600: '#4b5563',
        700: '#374151',
        800: '#1f2937',
        900: '#111827',
      },
    },
    fontFamily: {
      poppins: ['Poppins', 'sans-serif'],
    },
  },
  variants: [
    (matcher) => {
      if (!matcher.startsWith('mobile:'))
        return matcher
      return {
        matcher: matcher.slice(7), // 移除 mobile: 前缀
        parent: '@media (max-width: 768px)', // 添加媒体查询
      }
    },
  ],
  rules: [
    // 自定义背景颜色规则，直接使用十六进制值
    // [/^bg-\[#([a-fA-F0-9]{3,6})\]$/, ([, hex]) => ({ 'background-color': `#${hex}` })],
    // // 自定义文字颜色规则，直接使用十六进制值
    // [/^text-\[#([a-fA-F0-9]{3,6})\]$/, ([, hex]) => ({ 'color': `#${hex}` })],
    // // 自定义边框颜色规则，直接使用十六进制值
    // [/^border-\[#([a-fA-F0-9]{3,6})\]$/, ([, hex]) => ({ 'border-color': `#${hex}` })],
    ['xy-center', { position: 'absolute', top: '50%', left: '50%', transform: 'translate(-50%, -50%)' }],
    ['flex-center', { 'display': 'flex', 'justify-content': 'center', 'align-items': 'center' }],
    ['transition', { 'transition-property': 'all', 'transition-timing-function': 'cubic-bezier(0.4, 0, 0.2, 1)', 'transition-duration': '150ms' }],
    ['safe-pt', { 'padding-top': `env(safe-area-inset-top)` }],
    ['safe-pb', { 'padding-bottom': `env(safe-area-inset-bottom)` }],
    ['safe-mt', { 'margin-top': `env(safe-area-inset-top)` }],
    ['safe-mb', { 'margin-bottom': `env(safe-area-inset-bottom)` }],
    [/^safe-pt-(\d)$/, ([, d]) => ({ 'padding-top': `calc(env(safe-area-inset-top) + ${+d * 0.25}rem)` }), { layer: 'utilities' }], // 不兼容iOS < 11.2
    [/^safe-pb-(\d)$/, ([, d]) => ({ 'padding-bottom': `calc(env(safe-area-inset-bottom) + ${+d * 0.25}rem)` }), { layer: 'utilities' }], // 不兼容iOS < 11.2
    [/^safe-pb-\[(.*)\]$/, ([, d]) => ({ 'padding-bottom': `calc(env(safe-area-inset-bottom) + ${d})` }), { layer: 'utilities' }], // 不兼容iOS < 11.2
    [/^safe-mt-(\d)$/, ([, d]) => ({ 'margin-top': `calc(env(safe-area-inset-top) + ${+d * 0.25}rem)` }), { layer: 'utilities' }], // 不兼容iOS < 11.2
    [/^safe-mb-(\d)$/, ([, d]) => ({ 'margin-bottom': `calc(env(safe-area-inset-bottom) + ${+d * 0.25}rem)` }), { layer: 'utilities' }], // 不兼容iOS < 11.2
  ],
  transformers: [
    transformerDirectives(),
    transformerVariantGroup(),
    transformerEnv(),
  ],
  safelist: 'prose prose-sm m-auto text-left'.split(' '),
})
