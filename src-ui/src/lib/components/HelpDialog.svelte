<script lang="ts">
  import { showHelpDialog } from '$lib/stores';

  function close() {
    showHelpDialog.set(false);
  }
</script>

{#if $showHelpDialog}
  <div class="overlay" onclick={close} role="dialog" aria-modal="true">
    <div class="dialog" onclick={(e) => e.stopPropagation()}>
      <div class="header">
        <h2>A2L Editor 操作手册</h2>
        <button class="close-btn" onclick={close}>✕</button>
      </div>
      
      <div class="content">
        <section>
          <h3>快速开始</h3>
          <ol>
            <li><strong>打开文件</strong>: 文件 → 打开 ELF（或直接打开数据包）</li>
            <li><strong>选择 A2L</strong>: 点击"选择目标 A2L"按钮</li>
            <li><strong>搜索变量</strong>: 在搜索框输入关键词</li>
            <li><strong>添加变量</strong>: 右键选中变量 → 添加为观测变量/标定变量</li>
          </ol>
        </section>

        <section>
          <h3>选择与排序</h3>
          <ul>
            <li><strong>单选</strong>: 单击变量行</li>
            <li><strong>多选 (Ctrl)</strong>: 按住 Ctrl + 单击</li>
            <li><strong>范围选择 (Shift)</strong>: 按住 Shift + 单击</li>
            <li><strong>全选</strong>: Ctrl + A</li>
            <li><strong>键盘导航</strong>: ↑↓ 方向键</li>
            <li><strong>单列排序</strong>: 点击列头</li>
            <li><strong>多列排序</strong>: Shift + 点击列头（数字表示优先级）</li>
          </ul>
        </section>

        <section>
          <h3>添加变量</h3>
          <ul>
            <li><strong>从 ELF 添加</strong>: 右键选中变量 → 添加为观测变量/标定变量</li>
            <li><strong>手动添加</strong>: 点击 A2L 面板搜索栏右侧 ➕ 按钮，输入变量信息</li>
            <li>手动添加需填写：变量名、地址（十六进制）、数据类型</li>
          </ul>
        </section>

        <section>
          <h3>编辑与复制</h3>
          <ul>
            <li><strong>编辑变量</strong>: 在 A2L 面板选中单个变量，下方编辑区域修改属性</li>
            <li><strong>复制名称</strong>: 右键变量 → 复制名称</li>
            <li><strong>复制地址</strong>: 右键变量 → 复制地址</li>
            <li><strong>删除变量</strong>: 右键变量 → 删除变量</li>
            <li><strong>调整编辑区高度</strong>: 拖拽编辑区域上方的分隔条</li>
            <li><strong>调整列宽</strong>: 拖拽列标题之间的分隔线</li>
          </ul>
        </section>

        <section>
          <h3>数据包系统</h3>
          <p>每个 ELF 对应一个 <code>.a2ldata</code> 文件，与 ELF 同目录存放。</p>
          <ul>
            <li>首次打开 ELF 需要生成数据包（解析约 160 秒）</li>
            <li>后续加载只需约 150 毫秒</li>
            <li>可直接打开数据包文件，无需 ELF 源文件</li>
            <li>可通过"重新生成缓存"更新数据包</li>
          </ul>
        </section>

        <section>
          <h3>变量类型说明</h3>
          <ul>
            <li><strong>观测变量 (MEASUREMENT)</strong>: 只读，用于监控变量值</li>
            <li><strong>标定变量 (CHARACTERISTIC)</strong>: 可写，用于标定参数</li>
          </ul>
        </section>

        <section>
          <h3>支持的类型</h3>
          <table>
            <thead>
              <tr><th>DWARF 类型</th><th>A2L 类型</th><th>大小</th></tr>
            </thead>
            <tbody>
              <tr><td>uint8_t, char</td><td>UBYTE</td><td>1</td></tr>
              <tr><td>int8_t</td><td>SBYTE</td><td>1</td></tr>
              <tr><td>uint16_t</td><td>UWORD</td><td>2</td></tr>
              <tr><td>int16_t</td><td>SWORD</td><td>2</td></tr>
              <tr><td>uint32_t</td><td>ULONG</td><td>4</td></tr>
              <tr><td>int32_t</td><td>SLONG</td><td>4</td></tr>
              <tr><td>uint64_t</td><td>A_UINT64</td><td>8</td></tr>
              <tr><td>int64_t</td><td>A_INT64</td><td>8</td></tr>
              <tr><td>float</td><td>FLOAT32_IEEE</td><td>4</td></tr>
              <tr><td>double</td><td>FLOAT64_IEEE</td><td>8</td></tr>
            </tbody>
          </table>
        </section>

        <section>
          <h3>主题切换</h3>
          <p>点击右上角 🎨 按钮切换主题：Dark / Light / Midnight / Ocean（自动保存）</p>
        </section>

        <section>
          <h3>字节序设置</h3>
          <p>点击 Header 右侧「小端」/「大端」按钮切换字节序</p>
        </section>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
  }

  .dialog {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 12px;
    width: 90%;
    max-width: 600px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }

  .header h2 {
    margin: 0;
    font-size: 18px;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text);
    cursor: pointer;
    font-size: 20px;
    padding: 4px 8px;
  }

  .close-btn:hover {
    background: var(--bg-hover);
    border-radius: 4px;
  }

  .content {
    padding: 20px;
    overflow-y: auto;
  }

  section {
    margin-bottom: 20px;
  }

  section:last-child {
    margin-bottom: 0;
  }

  h3 {
    margin: 0 0 10px 0;
    font-size: 15px;
    color: var(--accent);
  }

  ol, ul {
    margin: 0;
    padding-left: 20px;
  }

  li {
    margin-bottom: 6px;
    line-height: 1.5;
  }

  p {
    margin: 0 0 10px 0;
    line-height: 1.5;
  }

  code {
    background: var(--bg-hover);
    padding: 2px 6px;
    border-radius: 4px;
    font-family: monospace;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  th, td {
    padding: 8px 12px;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }

  th {
    background: var(--bg-hover);
    font-weight: 500;
  }
</style>
