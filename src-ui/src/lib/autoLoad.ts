import { listen } from '@tauri-apps/api/event';
import { 
  packagePath, elfPath, elfFileName, elfTotalCount,
  a2lPath, a2lNames, elfEntries, a2lVariables,
  isLoading, statusMessage
} from './stores';
import { 
  loadPackage, loadA2l, searchElfEntries, searchA2lVariables 
} from './commands';

export function setupAutoLoad() {
  listen<{ package?: string; a2l?: string }>(
    'auto-load-files',
    async (event) => {
      const { package: pkg, a2l } = event.payload;
      
      if (pkg) {
        try {
          isLoading.set(true);
          statusMessage.set('⏳ 正在加载数据包...');
          const result = await loadPackage(pkg);
          packagePath.set(pkg);
          elfPath.set(result.meta.elf_path || null);
          elfFileName.set(result.meta.file_name);
          elfTotalCount.set(result.entry_count);
          const entries = await searchElfEntries('', 0, 10000);
          elfEntries.set(entries);
          statusMessage.set(`✅ 已加载 ${result.entry_count} 个条目`);
        } catch (e) {
          statusMessage.set(`❌ 加载数据包失败: ${e}`);
        }
      }
      
      if (a2l) {
        try {
          isLoading.set(true);
          statusMessage.set('⏳ 正在加载 A2L...');
          const result = await loadA2l(a2l);
          a2lPath.set(a2l);
          a2lNames.set(new Set(result.existing_names));
          const vars = await searchA2lVariables('', 0, 10000);
          a2lVariables.set(vars);
          statusMessage.set(`✅ 已加载目标 A2L (${result.variable_count} 个变量)`);
        } catch (e) {
          statusMessage.set(`❌ 加载 A2L 失败: ${e}`);
        }
      }
      
      isLoading.set(false);
    }
  );
}

// 测试专用：手动加载文件
export async function testLoadFiles(pkgPath?: string, a2lFilePath?: string): Promise<{ success: boolean; error?: string; debug?: any }> {
  const debug: any = { pkgPath, a2lFilePath };
  
  try {
    if (pkgPath) {
      isLoading.set(true);
      statusMessage.set('⏳ 正在加载数据包...');
      
      console.log('[testLoadFiles] Step 1: Loading package:', pkgPath);
      const result = await loadPackage(pkgPath);
      console.log('[testLoadFiles] Step 2: Package result:', result);
      debug.packageResult = result;
      
      packagePath.set(pkgPath);
      elfPath.set(result.meta.elf_path || null);
      elfFileName.set(result.meta.file_name);
      elfTotalCount.set(result.entry_count);
      
      console.log('[testLoadFiles] Step 3: State updated, entry_count:', result.entry_count);
      
      // 检查 getElfCount
      const count = await (await import('./commands')).getElfCount();
      console.log('[testLoadFiles] Step 4: getElfCount:', count);
      debug.elfCount = count;
      
      console.log('[testLoadFiles] Step 5: Calling searchElfEntries...');
      let entries: any[] = [];
      try {
        entries = await searchElfEntries('', 0, 10);
        console.log('[testLoadFiles] Step 6: Entries:', entries.length, entries.slice(0, 3));
        debug.entriesCount = entries.length;
        debug.sampleEntries = entries.slice(0, 3);
      } catch (searchError) {
        console.error('[testLoadFiles] searchElfEntries ERROR:', searchError);
        debug.searchError = String(searchError);
        entries = [];
      }
      
      elfEntries.set(entries);
      statusMessage.set(`✅ 已加载 ${result.entry_count} 个条目`);
    }
    
    if (a2lFilePath) {
      isLoading.set(true);
      statusMessage.set('⏳ 正在加载 A2L...');
      
      console.log('[testLoadFiles] Loading A2L:', a2lFilePath);
      const result = await loadA2l(a2lFilePath);
      console.log('[testLoadFiles] A2L result:', result);
      debug.a2lResult = { variable_count: result.variable_count };
      
      a2lPath.set(a2lFilePath);
      a2lNames.set(new Set(result.existing_names));
      const vars = await searchA2lVariables('', 0, 10000);
      console.log('[testLoadFiles] A2L vars loaded:', vars.length);
      debug.a2lVarsCount = vars.length;
      
      a2lVariables.set(vars);
      statusMessage.set(`✅ 已加载目标 A2L (${result.variable_count} 个变量)`);
    }
    
    isLoading.set(false);
    console.log('[testLoadFiles] Complete, debug:', JSON.stringify(debug));
    return { success: true, debug };
  } catch (e) {
    isLoading.set(false);
    statusMessage.set(`❌ 加载失败: ${e}`);
    console.error('[testLoadFiles] Error:', e);
    return { success: false, error: String(e), debug };
  }
}

// 暴露到 window 对象供 E2E 测试调用
if (typeof window !== 'undefined') {
  (window as any).__test_loadFiles__ = testLoadFiles;
}
