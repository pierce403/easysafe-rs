export const encoder = new TextEncoder()
export const decoder = new TextDecoder()

export function utf8(input: string): Uint8Array {
  return encoder.encode(input)
}

export function decodeUtf8(input: Uint8Array): string {
  return decoder.decode(input)
}

export function concatBytes(...arrays: Uint8Array[]): Uint8Array {
  const total = arrays.reduce((sum, part) => sum + part.length, 0)
  const output = new Uint8Array(total)
  let offset = 0
  arrays.forEach((part) => {
    output.set(part, offset)
    offset += part.length
  })
  return output
}

export function le32(value: number): Uint8Array {
  const out = new Uint8Array(4)
  new DataView(out.buffer).setUint32(0, value, true)
  return out
}

export function bytesToBase64(input: Uint8Array): string {
  let binary = ''
  input.forEach((byte) => {
    binary += String.fromCharCode(byte)
  })
  return btoa(binary)
}

export function base64ToBytes(input: string): Uint8Array {
  const binary = atob(input)
  const output = new Uint8Array(binary.length)
  for (let index = 0; index < binary.length; index += 1) {
    output[index] = binary.charCodeAt(index)
  }
  return output
}

export function isoNow(): string {
  return new Date().toISOString()
}

export function randomDocumentId(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(12))
  return Array.from(bytes)
    .map((byte) => byte.toString(16).padStart(2, '0'))
    .join('')
}
