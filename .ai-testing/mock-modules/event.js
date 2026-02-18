// 自包含的 Tauri Event API Mock
const eventListeners = new Map();

export async function listen(eventName, callback) {
  console.log('[Mock] listen:', eventName);
  if (!eventListeners.has(eventName)) {
    eventListeners.set(eventName, []);
  }
  eventListeners.get(eventName).push(callback);

  // 返回 unsubscribe 函数
  return () => {
    const listeners = eventListeners.get(eventName);
    const index = listeners.indexOf(callback);
    if (index > -1) listeners.splice(index, 1);
  };
}

export async function emit(eventName, payload) {
  console.log('[Mock] emit:', eventName, payload);
  const listeners = eventListeners.get(eventName) || [];
  listeners.forEach(cb => cb({ event: eventName, payload }));
}

export default { listen, emit };
