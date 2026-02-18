// 自包含的 Tauri Window API Mock
// 更新版本：支持 close-requested 事件监听

const windowEventListeners = new Map();

export function getCurrentWindow() {
  return {
    label: 'main',
    setTitle: async (title) => console.log('[Mock] setTitle:', title),
    minimize: async () => console.log('[Mock] minimize'),
    maximize: async () => console.log('[Mock] maximize'),
    close: async () => console.log('[Mock] close'),

    // 监听窗口事件 (如 close-requested)
    listen: async (eventName, callback) => {
      console.log('[Mock] window.listen:', eventName);
      if (!windowEventListeners.has(eventName)) {
        windowEventListeners.set(eventName, []);
      }
      windowEventListeners.get(eventName).push(callback);

      // 返回 unsubscribe 函数
      return () => {
        const listeners = windowEventListeners.get(eventName);
        const index = listeners.indexOf(callback);
        if (index > -1) listeners.splice(index, 1);
      };
    },

    // 销毁窗口
    destroy: async () => {
      console.log('[Mock] window.destroy');
    }
  };
}

// 测试辅助函数：触发窗口事件
export function triggerWindowEvent(eventName, payload) {
  const listeners = windowEventListeners.get(eventName) || [];
  listeners.forEach(cb => cb({ event: eventName, payload }));
}

export default { getCurrentWindow, triggerWindowEvent };
