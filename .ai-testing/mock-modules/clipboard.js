// 自包含的 Tauri Clipboard Plugin Mock
let clipboardText = '';

export async function writeText(text) {
  console.log('[Mock] clipboard.writeText:', text?.slice(0, 50));
  clipboardText = text;
}

export async function readText() {
  return clipboardText;
}

export default { writeText, readText };
