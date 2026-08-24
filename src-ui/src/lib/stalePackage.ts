import type { LoadStaleResult } from './types';
import {
  elfPath,
  elfFileName,
  showGenerateDialog,
  generateDialogNotice,
  statusMessage,
} from './stores';

/**
 * 处理过期数据包（版本不符 / ELF 已修改 / 旧 schema）：
 * - 能定位到仍存在的 ELF：设置 ELF 状态并弹出带原因提示的生成对话框，一键重新生成
 * - ELF 不可用（被删除/移动/推断失败）：仅状态栏提示，引导用户重新打开 ELF
 */
export function handleStalePackage(stale: LoadStaleResult): void {
  if (stale.elf_path && stale.elf_exists) {
    elfPath.set(stale.elf_path);
    elfFileName.set(stale.elf_path.split('/').pop() || '');
    generateDialogNotice.set(stale.reason);
    showGenerateDialog.set(true);
    statusMessage.set(`⚠️ ${stale.reason}`);
  } else {
    statusMessage.set(`⚠️ ${stale.reason}（原始 ELF 不可用，请重新打开 ELF 文件后再生成缓存）`);
  }
}
