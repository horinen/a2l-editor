// 自包含的 Tauri Dialog Plugin Mock

// 可通过 window.__mock_dialog_result__ 自定义返回值
export async function open(options = {}) {
  console.log('[Mock] dialog.open:', options);
  if (typeof window !== 'undefined' && window.__mock_dialog_result__) {
    return window.__mock_dialog_result__;
  }
  // 默认返回模拟文件路径
  if (options.filters?.[0]?.extensions?.includes('elf')) {
    return '/mock/test.elf';
  }
  if (options.filters?.[0]?.extensions?.includes('a2l')) {
    return '/mock/test.a2l';
  }
  return '/mock/test.a2ldata';
}

export async function save(options = {}) {
  console.log('[Mock] dialog.save:', options);
  return '/mock/saved.a2ldata';
}

export async function message(msg, options = {}) {
  console.log('[Mock] dialog.message:', msg);
}

export async function ask(msg, options = {}) {
  console.log('[Mock] dialog.ask:', msg);
  return true;
}

export async function confirm(msg, options = {}) {
  console.log('[Mock] dialog.confirm:', msg);
  return true;
}

export default { open, save, message, ask, confirm };
