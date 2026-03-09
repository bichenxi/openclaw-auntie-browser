/**
 * claw-browser-control bridge
 *
 * 这不是一个转发代理。Claw Browser 的 HTTP API 已经内嵌在 Tauri 应用里，
 * 直接监听 127.0.0.1:18790，无需再开端口。
 *
 * 本脚本的职责：
 *   1. 验证 Claw Browser 是否正在运行
 *   2. 保持进程存活（让 OpenClaw 的 bridge-check 通过）
 *   3. 每 30 秒检查一次 API 健康状态
 *
 * 使用：npx tsx bridge.ts
 */

const API_BASE = 'http://127.0.0.1:18790'

async function checkAlive(): Promise<boolean> {
  try {
    const res = await fetch(`${API_BASE}/page-info`, {
      signal: AbortSignal.timeout(2000),
    })
    return res.status < 500
  } catch {
    return false
  }
}

function log(msg: string) {
  const ts = new Date().toLocaleTimeString('zh-CN', { hour12: false })
  console.log(`[${ts}] [claw-browser] ${msg}`)
}

async function main() {
  log(`Bridge 启动，验证 Claw Browser API (${API_BASE}) …`)

  const alive = await checkAlive()
  if (alive) {
    log(`✓ Claw Browser 在线，所有浏览器操控通过 curl → ${API_BASE}`)
  } else {
    log(`⚠ 未检测到 Claw Browser API，请确保 Claw Browser 应用已启动`)
    log(`  操控指令将在应用启动后自动生效，无需重启本 bridge`)
  }

  log('Bridge 运行中，Ctrl+C 退出')

  // 每 30 秒轮询一次，提供健康日志
  setInterval(async () => {
    const ok = await checkAlive()
    if (!ok) {
      log('⚠ Claw Browser API 暂时不可达，请检查应用是否在运行')
    }
  }, 30_000)
}

main()
