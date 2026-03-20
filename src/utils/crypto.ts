/**
 * Simple credential encryption for localStorage.
 * Uses base64 + fixed key XOR obfuscation - not military-grade,
 * but prevents casual reading of credentials.
 */

const ENCRYPTION_KEY = 'oclaw_browser_v1'

function xorEncrypt(text: string): string {
  const key = ENCRYPTION_KEY
  let result = ''
  for (let i = 0; i < text.length; i++) {
    result += String.fromCharCode(text.charCodeAt(i) ^ key.charCodeAt(i % key.length))
  }
  return btoa(result)
}

function xorDecrypt(encoded: string): string {
  try {
    const key = ENCRYPTION_KEY
    const decoded = atob(encoded)
    let result = ''
    for (let i = 0; i < decoded.length; i++) {
      result += String.fromCharCode(decoded.charCodeAt(i) ^ key.charCodeAt(i % key.length))
    }
    return result
  } catch {
    return ''
  }
}

export function encryptCredential(value: string): string {
  return xorEncrypt(value)
}

export function decryptCredential(encrypted: string): string {
  return xorDecrypt(encrypted)
}
