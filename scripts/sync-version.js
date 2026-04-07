#!/usr/bin/env node

import { readFileSync, writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

const tauriConfPath = resolve(root, 'src-tauri/tauri.conf.json');
const tauriConf = JSON.parse(readFileSync(tauriConfPath, 'utf-8'));
const version = tauriConf.version;

if (!version) {
  console.error('无法从 src-tauri/tauri.conf.json 读取版本号');
  process.exit(1);
}

console.log(`同步版本号: ${version}`);

const files = [
  { path: resolve(root, 'Cargo.toml'), pattern: /^version\s*=\s*"[^"]*"/m },
  { path: resolve(root, 'src-tauri/Cargo.toml'), pattern: /^version\s*=\s*"[^"]*"/m },
  { path: resolve(root, 'package.json'), type: 'json' },
  { path: resolve(root, 'src-ui/package.json'), type: 'json' },
  { path: resolve(root, '.tauri-driver-test/lib/test-plan.json'), type: 'json' },
];

for (const file of files) {
  let content = readFileSync(file.path, 'utf-8');

  if (file.type === 'json') {
    const json = JSON.parse(content);
    if (json.version === version) {
      console.log(`  ✓ ${file.path.replace(root + '/', '')} 已经是 ${version}`);
      continue;
    }
    json.version = version;
    content = JSON.stringify(json, null, 2) + '\n';
  } else {
    const match = content.match(file.pattern);
    if (match && match[0].includes(version)) {
      console.log(`  ✓ ${file.path.replace(root + '/', '')} 已经是 ${version}`);
      continue;
    }
    content = content.replace(file.pattern, `version = "${version}"`);
  }

  writeFileSync(file.path, content, 'utf-8');
  console.log(`  ✎ ${file.path.replace(root + '/', '')} → ${version}`);
}

console.log('完成!');
