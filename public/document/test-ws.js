const WebSocket = require('ws');

// 使用你 openclaw(1).json 中最新的 token
const token = "15fa52bd6be6987b49fab9f92590e35308aa750497d64865";
const url = `ws://127.0.0.1:18789/?token=${token}`;

console.log(`[${new Date().toLocaleTimeString()}] 正在发起连接: ${url}`);

const ws = new WebSocket(url, {
  handshakeTimeout: 5000,
  headers: {
    "User-Agent": "OpenClaw-Debug-Client"
  }
});

ws.on('open', () => {
  console.log('✅ 握手成功！WebSocket 连接已建立，端口 18789 响应正常。');
  ws.close();
});

ws.on('error', (err) => {
  console.error('❌ 捕捉到错误:');
  console.error(`- 错误代码 (Code): ${err.code}`);
  console.error(`- 错误消息 (Message): ${err.message}`);

  if (err.code === 'ECONNREFUSED') {
    console.error('👉 诊断：Gateway 进程根本没有运行，或者没有监听 18789 端口。');
  } else if (err.code === 'ETIMEDOUT') {
    console.error('👉 诊断：连接超时，可能是防火墙或安全软件拦截了数据包。');
  }
});

ws.on('close', (code, reason) => {
  console.log(`ℹ️ 连接已关闭 | 状态码: ${code} | 原因: ${reason || '无'}`);
  if (code === 1006) {
    console.error('👉 诊断：典型的 1006 异常关闭。这通常意味着 Gateway 进程在收到请求后瞬间崩溃了。');
  }
});