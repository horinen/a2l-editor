use a2l_editor::{
    compute_file_hash, format_file_size, A2lGenerator, Cache, CacheEntry, DataPackage, DwarfParser,
    ElfParser, TypeInfo,
};
use anyhow::Result;
use std::path::PathBuf;
use std::time::Instant;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "parse" => {
            if args.len() < 3 {
                eprintln!("用法: a2l-cli parse <elf文件路径> [--deep]");
                return Ok(());
            }
            let path = PathBuf::from(&args[2]);
            let deep = args.contains(&"--deep".to_string());
            parse_elf(&path, deep)?;
        }
        "struct" => {
            if args.len() < 4 {
                eprintln!("用法: a2l-cli struct <elf文件> <结构体名或成员名> [--export]");
                eprintln!("  --export    导出结构体成员到 A2L");
                return Ok(());
            }
            let path = PathBuf::from(&args[2]);
            let name = &args[3];
            let export = args.contains(&"--export".to_string());
            search_struct(&path, name, export)?;
        }
        "export" => {
            if args.len() < 3 {
                eprintln!("用法: a2l-cli export <elf文件路径> [-o 输出文件] [-n 变量数量]");
                return Ok(());
            }
            let path = PathBuf::from(&args[2]);
            let output = args
                .iter()
                .position(|a| a == "-o")
                .and_then(|i| args.get(i + 1))
                .map(|s| s.to_string());
            let limit: usize = args
                .iter()
                .position(|a| a == "-n")
                .and_then(|i| args.get(i + 1))
                .and_then(|s| s.parse().ok())
                .unwrap_or(100);
            export_a2l(&path, output.as_deref(), limit)?;
        }
        "cache" => {
            list_cache()?;
        }
        "clear" => {
            clear_cache()?;
        }
        "type" => {
            if args.len() < 4 {
                eprintln!("用法: a2l-cli type <elf文件路径> <变量名>");
                return Ok(());
            }
            let path = PathBuf::from(&args[2]);
            let var_name = &args[3];
            show_variable_type(&path, var_name)?;
        }
        "arrays" => {
            if args.len() < 3 {
                eprintln!("用法: a2l-cli arrays <elf文件路径> [数量]");
                return Ok(());
            }
            let path = PathBuf::from(&args[2]);
            let limit: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(20);
            list_array_types(&path, limit)?;
        }
        "enums" => {
            if args.len() < 3 {
                eprintln!("用法: a2l-cli enums <elf文件路径> [数量]");
                return Ok(());
            }
            let path = PathBuf::from(&args[2]);
            let limit: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(20);
            list_enum_types(&path, limit)?;
        }
        "dwarf-vars" => {
            if args.len() < 3 {
                eprintln!("用法: a2l-cli dwarf-vars <elf文件路径> [数量]");
                return Ok(());
            }
            let path = PathBuf::from(&args[2]);
            let limit: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(20);
            list_dwarf_variables(&path, limit)?;
        }
        "struct-instances" => {
            if args.len() < 3 {
                eprintln!("用法: a2l-cli struct-instances <elf文件路径> [数量]");
                return Ok(());
            }
            let path = PathBuf::from(&args[2]);
            let limit: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(20);
            list_struct_instances(&path, limit)?;
        }
        "debug-member" => {
            if args.len() < 4 {
                eprintln!("用法: a2l-cli debug-member <elf文件路径> <结构体名>");
                return Ok(());
            }
            let path = PathBuf::from(&args[2]);
            let struct_name = &args[3];
            debug_member_type(&path, struct_name)?;
        }
        "check-offset" => {
            if args.len() < 4 {
                eprintln!("用法: a2l-cli check-offset <elf文件路径> <偏移量(十六进制)>");
                return Ok(());
            }
            let path = PathBuf::from(&args[2]);
            let offset = u64::from_str_radix(args[3].trim_start_matches("0x"), 16)?;
            check_type_offset(&path, offset)?;
        }
        "bitfields" => {
            if args.len() < 3 {
                eprintln!("用法: a2l-cli bitfields <elf文件路径> [数量]");
                return Ok(());
            }
            let path = PathBuf::from(&args[2]);
            let limit: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(20);
            list_bitfields(&path, limit)?;
        }
        "entries" => {
            if args.len() < 3 {
                eprintln!("用法: a2l-cli entries <elf文件路径> [搜索词] [-n 数量]");
                return Ok(());
            }
            let path = PathBuf::from(&args[2]);
            let search = args
                .get(3)
                .filter(|s| !s.starts_with('-'))
                .map(|s| s.as_str());
            let limit: usize = args
                .iter()
                .position(|a| a == "-n")
                .and_then(|i| args.get(i + 1))
                .and_then(|s| s.parse().ok())
                .unwrap_or(50);
            list_a2l_entries(&path, search, limit)?;
        }
        "create-package" => {
            if args.len() < 3 {
                eprintln!("用法: a2l-cli create-package <elf文件路径> [-o 输出路径]");
                return Ok(());
            }
            let path = PathBuf::from(&args[2]);
            let output = args
                .iter()
                .position(|a| a == "-o")
                .and_then(|i| args.get(i + 1))
                .map(|s| PathBuf::from(s));
            create_package(&path, output.as_ref())?;
        }
        _ => {
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    println!("A2L Editor CLI 工具");
    println!();
    println!("用法:");
    println!("  a2l-cli parse <elf文件> [选项]        解析 ELF 文件");
    println!("    选项:");
    println!("      --deep      深度解析（DWARF 类型信息）");
    println!("  a2l-cli struct <elf文件> <名称> [选项]  查找结构体");
    println!("    选项:");
    println!("      --export    导出结构体成员到 A2L");
    println!("  a2l-cli export <elf文件> [选项]        导出为 A2L 文件");
    println!("    选项:");
    println!("      -o <文件>   输出文件路径 (默认: 输出到控制台)");
    println!("      -n <数量>   导出变量数量 (默认: 100)");
    println!("  a2l-cli create-package <elf文件> [选项]  创建数据包");
    println!("    选项:");
    println!("      -o <路径>   输出路径 (默认: <elf文件>.a2ldata)");
    println!("  a2l-cli type <elf文件> <变量名>        显示变量类型信息");
    println!("  a2l-cli arrays <elf文件> [数量]        列出数组类型及维度");
    println!("  a2l-cli enums <elf文件> [数量]         列出枚举类型及变体");
    println!("  a2l-cli dwarf-vars <elf文件> [数量]    列出 DWARF 变量及类型");
    println!("  a2l-cli struct-instances <elf文件> [数量]  列出结构体实例变量");
    println!("  a2l-cli bitfields <elf文件> [数量]     列出含位域的结构体");
    println!("  a2l-cli entries <elf文件> [搜索词] [-n 数量]  列出 A2L 条目");
    println!("  a2l-cli cache                          列出缓存");
    println!("  a2l-cli clear                          清除缓存");
}

fn search_struct(path: &PathBuf, name: &str, export: bool) -> Result<()> {
    println!("查找结构体: {}", name);
    println!();

    let elf_data = std::fs::read(path)?;
    let parser = DwarfParser::parse(&elf_data)?;

    if !parser.has_dwarf_info() {
        println!("错误: ELF 文件不包含 DWARF 调试信息");
        return Ok(());
    }

    let exact_match = parser.find_struct_by_name(name);

    if let Some(struct_info) = exact_match {
        println!("=== 找到结构体 ===");
        print_struct_info(struct_info);

        if export {
            export_struct_members(path, struct_info)?;
        }
    } else {
        let partial_matches = parser.find_structs_containing_member(name);

        if partial_matches.is_empty() {
            println!("未找到匹配的结构体");

            let all_structs = parser.list_structs();
            let similar: Vec<_> = all_structs
                .iter()
                .filter(|s| s.name.to_lowercase().contains(&name.to_lowercase()))
                .take(10)
                .collect();

            if !similar.is_empty() {
                println!();
                println!("相似的结构体:");
                for s in similar {
                    println!("  {} ({} 成员)", s.name, s.members.len());
                }
            }
        } else {
            println!(
                "=== 找到 {} 个包含 '{}' 的结构体 ===",
                partial_matches.len(),
                name
            );

            for (i, (struct_info, member)) in partial_matches.iter().take(10).enumerate() {
                println!();
                println!("{}. {}", i + 1, struct_info.name);
                println!("   大小: {} 字节", struct_info.size);
                println!("   成员数: {}", struct_info.members.len());
                println!("   匹配成员: {} (偏移: {})", member.name, member.offset);
            }

            if partial_matches.len() > 10 {
                println!();
                println!("  ... 还有 {} 个结果", partial_matches.len() - 10);
            }

            if export && partial_matches.len() == 1 {
                let (struct_info, _) = partial_matches[0];
                export_struct_members(path, struct_info)?;
            }
        }
    }

    Ok(())
}

fn print_struct_info(struct_info: &TypeInfo) {
    println!("名称: {}", struct_info.name);
    println!("大小: {} 字节", struct_info.size);
    println!("成员数: {}", struct_info.members.len());
    println!();

    if !struct_info.members.is_empty() {
        println!("成员列表:");
        for member in &struct_info.members {
            println!(
                "  {:30} @ +{:<4} ({} bytes, type: {})",
                member.name, member.offset, member.type_size, member.type_name
            );
        }
    }
}

fn export_struct_members(path: &PathBuf, struct_info: &TypeInfo) -> Result<()> {
    println!();
    println!("=== 导出结构体成员到 A2L ===");

    let hash = compute_file_hash(path)?;
    let cache = Cache::open()?;

    let variables = if cache.exists(&hash) {
        let (_, vars) = cache.get(&hash)?.expect("缓存应该存在");
        vars
    } else {
        println!("解析 ELF 文件...");
        let parser = ElfParser::parse(path)?;
        parser.variables().to_vec()
    };

    let matching_vars: Vec<_> = variables
        .iter()
        .filter(|v| {
            v.type_name == struct_info.name
                || v.name.contains(&struct_info.name)
                || struct_info
                    .members
                    .iter()
                    .any(|m| v.name.ends_with(&format!(".{}", m.name)))
        })
        .collect();

    if matching_vars.is_empty() {
        println!("未找到结构体实例变量");
        println!("提示: 结构体变量名通常包含结构体名");
        return Ok(());
    }

    println!("找到 {} 个结构体实例变量", matching_vars.len());

    let output_path = format!("/tmp/{}_members.a2l", struct_info.name);
    let output_path = PathBuf::from(&output_path);

    let mut generator = A2lGenerator::new("A2L_Editor_Project", "ECU_Module");

    for var in &matching_vars {
        let base_address = var.address;

        for member in &struct_info.members {
            let member_address = base_address + member.offset as u64;
            let member_name = format!("{}.{}", var.name, member.name);

            let mut member_var = a2l_editor::Variable::new(
                member_name,
                member_address,
                member.type_size,
                member.type_name.clone(),
                var.section.clone(),
            );

            let encoding = if member.type_name.starts_with('u') || member.type_name.starts_with('U')
            {
                a2l_editor::TypeEncoding::Unsigned
            } else if member.type_name.starts_with('s') || member.type_name.starts_with('i') {
                a2l_editor::TypeEncoding::Signed
            } else if member.type_name.contains("float") || member.type_name.contains("double") {
                a2l_editor::TypeEncoding::Float
            } else {
                a2l_editor::TypeEncoding::Unsigned
            };

            member_var.type_info = Some(a2l_editor::TypeInfo::primitive(
                member.type_name.clone(),
                member.type_size,
                encoding,
            ));

            generator.add_variable(member_var);
        }
    }

    generator.save(&output_path)?;

    let content = generator.generate();
    println!("已保存到: {}", output_path.display());
    println!("文件大小: {}", format_file_size(content.len() as u64));
    println!(
        "变量数: {} ({} 实例 x {} 成员)",
        matching_vars.len() * struct_info.members.len(),
        matching_vars.len(),
        struct_info.members.len()
    );

    Ok(())
}

fn parse_elf(path: &PathBuf, deep: bool) -> Result<()> {
    println!("解析文件: {}", path.display());

    let metadata = std::fs::metadata(path)?;
    let file_size = metadata.len();
    let modified = metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    println!("文件大小: {}", format_file_size(file_size));
    if deep {
        println!("解析模式: 深度解析（含 DWARF）");
    }
    println!();

    let hash = compute_file_hash(path)?;
    println!("文件指纹: {}...", &hash[..16]);

    let cache = Cache::open()?;

    if cache.exists(&hash) {
        let cached_has_dwarf = cache.has_dwarf(&hash);

        if deep && !cached_has_dwarf {
            println!("缓存状态: 存在但无 DWARF 信息，重新解析...");
            println!();

            let mut cache = cache;
            cache.delete(&hash)?;

            let start = Instant::now();
            let parser = ElfParser::parse_deep(path)?;
            let parse_time = start.elapsed();

            let has_dwarf = parser.has_dwarf();
            let variables = parser.variables().to_vec();

            println!("=== 解析结果 ===");
            println!("变量数量: {}", variables.len());
            println!("DWARF 信息: {}", if has_dwarf { "有" } else { "无" });
            println!("解析耗时: {} ms", parse_time.as_millis());
            println!();

            print_variable_stats(&variables);

            let entry = CacheEntry::new(
                hash.clone(),
                path.to_string_lossy().to_string(),
                file_size,
                modified,
                variables.len(),
                parse_time.as_millis() as u64,
                has_dwarf,
            );

            cache.save(&hash, &entry, &variables)?;

            if let Some(store) = parser.a2l_entries() {
                let entry_count = store.len();
                cache.save_a2l_entries(&hash, entry_count, store)?;
                println!("A2L 条目已缓存: {} 条", entry_count);
            }

            println!();
            println!("缓存已保存到: {}", cache.cache_dir().display());
        } else {
            println!("缓存状态: 命中");
            if cached_has_dwarf {
                println!("DWARF 信息: 已缓存");
            }
            println!();

            let start = Instant::now();
            let (entry, variables) = cache.get(&hash)?.expect("缓存应该存在");
            let load_time = start.elapsed();

            println!("=== 缓存信息 ===");
            println!("变量数量: {}", entry.variable_count);
            println!("原始解析耗时: {} ms", entry.parse_time_ms);
            println!("缓存加载耗时: {} ms", load_time.as_millis());
            if entry.has_dwarf {
                println!("DWARF 信息: 有");
            }
            println!();

            print_variable_stats(&variables);
        }
    } else {
        println!("缓存状态: 未命中，开始解析...");
        println!();

        let start = Instant::now();
        let parser = if deep {
            ElfParser::parse_deep(path)?
        } else {
            ElfParser::parse(path)?
        };
        let parse_time = start.elapsed();

        let has_dwarf = parser.has_dwarf();
        let variables = parser.variables().to_vec();

        println!("=== 解析结果 ===");
        println!("变量数量: {}", variables.len());
        if deep {
            println!("DWARF 信息: {}", if has_dwarf { "有" } else { "无" });
        }
        println!("解析耗时: {} ms", parse_time.as_millis());
        println!();

        print_variable_stats(&variables);

        let entry = CacheEntry::new(
            hash.clone(),
            path.to_string_lossy().to_string(),
            file_size,
            modified,
            variables.len(),
            parse_time.as_millis() as u64,
            has_dwarf,
        );

        let mut cache = cache;
        cache.save(&hash, &entry, &variables)?;

        if let Some(store) = parser.a2l_entries() {
            let entry_count = store.len();
            cache.save_a2l_entries(&hash, entry_count, store)?;
            println!("A2L 条目已缓存: {} 条", entry_count);
        }

        println!();
        println!("缓存已保存到: {}", cache.cache_dir().display());
    }

    Ok(())
}

fn show_variable_type(path: &PathBuf, var_name: &str) -> Result<()> {
    println!("查找变量: {}", var_name);
    println!();

    let hash = compute_file_hash(path)?;
    let cache = Cache::open()?;

    let variables = if cache.exists(&hash) {
        let (_, vars) = cache.get(&hash)?.expect("缓存应该存在");
        vars
    } else {
        println!("缓存未命中，解析文件...");
        let parser = ElfParser::parse_deep(path)?;
        parser.variables().to_vec()
    };

    let var = variables.iter().find(|v| v.name == var_name);

    if let Some(v) = var {
        println!("=== 变量信息 ===");
        println!("名称: {}", v.name);
        println!("地址: 0x{:08X}", v.address);
        println!("大小: {} 字节", v.size);
        println!("类型名: {}", v.type_name);
        println!("段: {}", v.section);

        if let Some(ref info) = v.type_info {
            println!();
            println!("=== 类型详情 ===");
            println!("类型: {} ({})", info.name, info.kind);
            println!("编码: {}", info.encoding);
            println!("大小: {} 字节", info.size);

            if !info.members.is_empty() {
                println!();
                println!("成员:");
                for m in &info.members {
                    println!(
                        "  {} @ +{} ({} 字节, {})",
                        m.name, m.offset, m.type_size, m.type_name
                    );
                }
            }

            if !info.variants.is_empty() {
                println!();
                println!("枚举值:");
                for v in &info.variants {
                    println!("  {} = {}", v.name, v.value);
                }
            }

            if !info.array_dims.is_empty() {
                println!();
                println!("数组维度: {:?}", info.array_dims);
            }
        }
    } else {
        println!("未找到变量: {}", var_name);

        let similar: Vec<_> = variables
            .iter()
            .filter(|v| v.name.to_lowercase().contains(&var_name.to_lowercase()))
            .take(5)
            .collect();

        if !similar.is_empty() {
            println!();
            println!("相似的变量:");
            for v in similar {
                println!("  {}", v.name);
            }
        }
    }

    Ok(())
}

fn export_a2l(path: &PathBuf, output: Option<&str>, limit: usize) -> Result<()> {
    println!("导出文件: {}", path.display());

    let hash = compute_file_hash(path)?;
    let cache = Cache::open()?;

    let variables = if cache.exists(&hash) {
        println!("从缓存加载...");
        let (_, vars) = cache.get(&hash)?.expect("缓存应该存在");
        vars
    } else {
        println!("解析 ELF 文件...");
        let parser = ElfParser::parse(path)?;
        parser.variables().to_vec()
    };

    let export_count = limit.min(variables.len());
    println!("导出变量: {} / {}", export_count, variables.len());

    let mut generator = A2lGenerator::new("A2L_Editor_Project", "ECU_Module");

    for var in variables.iter().take(export_count) {
        generator.add_variable(var.clone());
    }

    let content = generator.generate();

    if let Some(output_path) = output {
        let output_path = PathBuf::from(output_path);
        generator.save(&output_path)?;
        println!("已保存到: {}", output_path.display());
        println!("文件大小: {}", format_file_size(content.len() as u64));
    } else {
        println!();
        println!("=== A2L 内容 (前 2000 字符) ===");
        if content.len() > 2000 {
            println!("{}...", &content[..2000]);
        } else {
            println!("{}", content);
        }
    }

    Ok(())
}

fn print_variable_stats(variables: &[a2l_editor::Variable]) {
    println!("=== 变量统计 ===");

    let mut size_distribution = std::collections::HashMap::new();
    let mut section_distribution = std::collections::HashMap::new();
    let mut type_count_with_info = 0;

    for v in variables {
        *size_distribution.entry(v.size).or_insert(0) += 1;
        *section_distribution.entry(v.section.clone()).or_insert(0) += 1;
        if v.type_info.is_some() {
            type_count_with_info += 1;
        }
    }

    println!();
    if type_count_with_info > 0 {
        println!(
            "含类型信息的变量: {} / {}",
            type_count_with_info,
            variables.len()
        );
        println!();
    }

    println!("按大小分布:");
    let mut sizes: Vec<_> = size_distribution.iter().collect();
    sizes.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    for (size, count) in sizes.iter().take(10) {
        println!("  {} 字节: {} 个", size, count);
    }

    println!();
    println!("按段分布:");
    let mut sections: Vec<_> = section_distribution.iter().collect();
    sections.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    for (section, count) in sections.iter().take(10) {
        let section_name = if section.is_empty() {
            "(未知)"
        } else {
            section
        };
        println!("  {}: {} 个", section_name, count);
    }

    println!();
    println!("前 10 个变量:");
    for v in variables.iter().take(10) {
        let type_info_str = if let Some(ref info) = v.type_info {
            format!(" [{}]", info.name)
        } else {
            String::new()
        };
        println!(
            "  {:40} @ 0x{:08X} ({} 字节){}",
            v.name, v.address, v.size, type_info_str
        );
    }

    if variables.len() > 10 {
        println!("  ... 还有 {} 个变量", variables.len() - 10);
    }
}

fn list_cache() -> Result<()> {
    let cache = Cache::open()?;
    let entries = cache.list()?;

    if entries.is_empty() {
        println!("缓存为空");
        return Ok(());
    }

    println!("缓存目录: {}", cache.cache_dir().display());
    println!();
    println!("已缓存文件 ({} 个):", entries.len());
    println!();

    for entry in entries {
        let created = chrono::DateTime::from_timestamp(entry.created_at, 0)
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "?".to_string());

        let dwarf_str = if entry.has_dwarf { " [DWARF]" } else { "" };

        println!("  文件: {}{}", entry.file_path, dwarf_str);
        println!("  大小: {}", format_file_size(entry.file_size));
        println!("  变量: {}", entry.variable_count);
        println!("  解析耗时: {} ms", entry.parse_time_ms);
        println!("  缓存时间: {}", created);
        println!("  指纹: {}...", &entry.file_hash[..16]);
        println!();
    }

    Ok(())
}

fn clear_cache() -> Result<()> {
    let cache = Cache::open()?;
    let entries = cache.list()?;
    let count = entries.len();

    for entry in entries {
        cache.delete(&entry.file_hash)?;
    }

    println!("已清除 {} 个缓存", count);
    Ok(())
}

fn list_array_types(path: &PathBuf, limit: usize) -> Result<()> {
    println!("解析 DWARF 数组类型...");
    println!();

    let parser = DwarfParser::parse_from_file(path)?;

    if !parser.has_dwarf_info() {
        println!("错误: ELF 文件不包含 DWARF 调试信息");
        return Ok(());
    }

    let all_types = parser.all_types();
    let arrays: Vec<_> = all_types
        .iter()
        .filter(|t| t.kind == a2l_editor::TypeKind::Array && !t.array_dims.is_empty())
        .take(limit)
        .collect();

    if arrays.is_empty() {
        println!("未找到有维度信息的数组类型");
        return Ok(());
    }

    println!("=== 数组类型 (前 {} 个) ===", arrays.len());
    println!();

    for arr in arrays {
        let total_elements: usize = arr.array_dims.iter().product();
        let element_size = if total_elements > 0 {
            arr.size / total_elements
        } else {
            0
        };
        let dims_str: Vec<String> = arr.array_dims.iter().map(|d| d.to_string()).collect();
        println!(
            "{} - {} bytes (element: {} bytes)",
            dims_str.join("x"),
            arr.size,
            element_size
        );
    }

    Ok(())
}

fn list_enum_types(path: &PathBuf, limit: usize) -> Result<()> {
    println!("解析 DWARF 枚举类型...");
    println!();

    let parser = DwarfParser::parse_from_file(path)?;

    if !parser.has_dwarf_info() {
        println!("错误: ELF 文件不包含 DWARF 调试信息");
        return Ok(());
    }

    let all_types = parser.all_types();
    let enums: Vec<_> = all_types
        .iter()
        .filter(|t| t.kind == a2l_editor::TypeKind::Enum && !t.variants.is_empty())
        .take(limit)
        .collect();

    if enums.is_empty() {
        println!("未找到有变体信息的枚举类型");
        return Ok(());
    }

    println!("=== 枚举类型 (前 {} 个) ===", enums.len());
    println!();

    for en in enums {
        println!(
            "{} ({} bytes, {} variants):",
            en.name,
            en.size,
            en.variants.len()
        );
        for v in &en.variants {
            println!("  {} = {}", v.name, v.value);
        }
        println!();
    }

    Ok(())
}

fn list_dwarf_variables(path: &PathBuf, limit: usize) -> Result<()> {
    println!("解析 DWARF 变量...");
    println!();

    let parser = DwarfParser::parse_from_file(path)?;

    if !parser.has_dwarf_info() {
        println!("错误: ELF 文件不包含 DWARF 调试信息");
        return Ok(());
    }

    let (total, resolved, unresolved_offsets) = parser.debug_type_resolution();
    println!("类型缓存大小: {}", parser.get_type_cache_size());
    println!("变量总数: {}", total);
    println!("已解析类型: {}", resolved);
    println!("未解析偏移量数: {}", unresolved_offsets);
    println!();

    let variables = parser.list_variables_with_types();

    if variables.is_empty() {
        println!("未找到 DWARF 变量");
        return Ok(());
    }

    println!("=== DWARF 变量 (前 {} 个) ===", limit.min(variables.len()));
    println!();

    for (name, type_name) in variables.iter().take(limit) {
        println!("{}: {}", name, type_name);
    }

    println!();
    println!("共 {} 个变量", variables.len());

    Ok(())
}

fn list_struct_instances(path: &PathBuf, limit: usize) -> Result<()> {
    println!("查找结构体实例变量...");
    println!();

    let parser = DwarfParser::parse_from_file(path)?;

    if !parser.has_dwarf_info() {
        println!("错误: ELF 文件不包含 DWARF 调试信息");
        return Ok(());
    }

    let struct_instances = parser.list_struct_instance_variables();

    if struct_instances.is_empty() {
        println!("未找到结构体实例变量");
        return Ok(());
    }

    println!(
        "=== 结构体实例变量 (前 {} 个) ===",
        limit.min(struct_instances.len())
    );
    println!();

    for (name, type_info) in struct_instances.iter().take(limit) {
        println!(
            "{}: {} ({} bytes, {} members)",
            name,
            type_info.name,
            type_info.size,
            type_info.members.len()
        );
    }

    println!();
    println!("共 {} 个结构体实例变量", struct_instances.len());

    Ok(())
}

fn debug_member_type(path: &PathBuf, struct_name: &str) -> Result<()> {
    let parser = DwarfParser::parse_from_file(path)?;

    if !parser.has_dwarf_info() {
        println!("错误: ELF 文件不包含 DWARF 调试信息");
        return Ok(());
    }

    parser.debug_member_type(struct_name);

    Ok(())
}

fn check_type_offset(path: &PathBuf, offset: u64) -> Result<()> {
    let parser = DwarfParser::parse_from_file(path)?;

    if !parser.has_dwarf_info() {
        println!("错误: ELF 文件不包含 DWARF 调试信息");
        return Ok(());
    }

    parser.check_type_at_offset(offset);

    Ok(())
}

fn list_bitfields(path: &PathBuf, limit: usize) -> Result<()> {
    println!("查找含位域的结构体...");
    println!();

    let parser = DwarfParser::parse_from_file(path)?;

    if !parser.has_dwarf_info() {
        println!("错误: ELF 文件不包含 DWARF 调试信息");
        return Ok(());
    }

    let structs = parser.list_structs();
    let mut bitfield_structs = Vec::new();

    for s in structs {
        let bitfield_members: Vec<_> = s.members.iter().filter(|m| m.is_bitfield()).collect();
        if !bitfield_members.is_empty() {
            bitfield_structs.push((s, bitfield_members));
        }
    }

    if bitfield_structs.is_empty() {
        println!("未找到含位域的结构体");
        return Ok(());
    }

    println!(
        "=== 含位域的结构体 (前 {} 个) ===",
        limit.min(bitfield_structs.len())
    );
    println!();

    for (i, (s, members)) in bitfield_structs.iter().take(limit).enumerate() {
        println!("{}. {} ({} bytes)", i + 1, s.name, s.size);
        for m in members {
            let (bit_offset, bit_size) = (m.bit_offset.unwrap_or(0), m.bit_size.unwrap_or(0));
            println!(
                "   {} @ +{} bits [{},{}] ({} bits)",
                m.name,
                m.offset,
                bit_offset,
                bit_offset + bit_size - 1,
                bit_size
            );
        }
        println!();
    }

    println!("共 {} 个含位域的结构体", bitfield_structs.len());

    Ok(())
}

fn list_a2l_entries(path: &PathBuf, search: Option<&str>, limit: usize) -> Result<()> {
    println!("加载 A2L 条目...");
    let start = Instant::now();

    let store = if DataPackage::exists(path) {
        println!("从数据包加载...");
        let pkg = DataPackage::open(path)?;
        pkg.load_entries()?
    } else {
        let hash = compute_file_hash(path)?;
        let cache = Cache::open()?;

        if let Some(cached_store) = cache.get_a2l_entries(&hash)? {
            let elapsed = start.elapsed();
            println!("从旧缓存加载: {:?}", elapsed);
            cached_store
        } else {
            println!("数据包不存在，执行深度解析...");
            let parser = ElfParser::parse_deep(path)?;

            if let Some(store) = parser.a2l_entries() {
                let entry_count = store.len();

                let mut cache = cache;

                let metadata = std::fs::metadata(path)?;
                let file_size = metadata.len();
                let modified = metadata
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs();

                let cache_entry = CacheEntry::new(
                    hash.clone(),
                    path.to_string_lossy().to_string(),
                    file_size,
                    modified,
                    parser.variable_count(),
                    start.elapsed().as_millis() as u64,
                    parser.has_dwarf(),
                );

                let variables = parser.variables().to_vec();
                cache.save(&hash, &cache_entry, &variables)?;
                cache.save_a2l_entries(&hash, entry_count, &store)?;

                let elapsed = start.elapsed();
                println!("解析并缓存: {:?}", elapsed);
                store.clone()
            } else {
                println!("未找到 A2L 条目（需要 DWARF 信息）");
                return Ok(());
            }
        }
    };

    println!();
    println!("=== A2L 条目列表 ===");
    println!("总条目数: {}", store.len());
    println!();

    let entries = if let Some(query) = search {
        store.search(query)
    } else {
        store.entries.iter().collect()
    };

    println!("匹配条目数: {}", entries.len());
    println!();

    for entry in entries.iter().take(limit) {
        let bit_info = match (entry.bit_offset, entry.bit_size) {
            (Some(bo), Some(bs)) => format!(" bits[{},{}]", bo, bo + bs - 1),
            _ => String::new(),
        };
        let arr_info = entry
            .array_index
            .as_ref()
            .map(|idx| {
                format!(
                    " [{}]",
                    idx.iter()
                        .map(|i| i.to_string())
                        .collect::<Vec<_>>()
                        .join("][")
                )
            })
            .unwrap_or_default();

        println!(
            "{:50} @ 0x{:08X} {:3}B {}{}{}",
            entry.full_name, entry.address, entry.size, entry.a2l_type, arr_info, bit_info
        );
    }

    if entries.len() > limit {
        println!("... 还有 {} 条未显示", entries.len() - limit);
    }

    Ok(())
}

fn create_package(elf_path: &PathBuf, output_path: Option<&PathBuf>) -> Result<()> {
    let metadata = std::fs::metadata(elf_path)?;
    let file_size = metadata.len();

    println!("解析 ELF 文件: {}", elf_path.display());
    println!("文件大小: {}", format_file_size(file_size));
    println!();

    let start = Instant::now();
    println!("深度解析中...");

    let parser = ElfParser::parse_deep(elf_path)?;

    let store = match parser.a2l_entries() {
        Some(s) => s,
        None => {
            println!("错误: 未找到 A2L 条目（需要 DWARF 信息）");
            return Ok(());
        }
    };

    let entry_count = store.len();
    println!("解析完成: {} 条目", entry_count);

    println!();
    println!("保存数据包...");

    let pkg = if let Some(custom_path) = output_path {
        DataPackage::create_at(custom_path, elf_path)?
    } else {
        DataPackage::create(elf_path)?
    };

    let pkg_path = pkg.path().to_path_buf();

    let mut pkg = pkg;
    pkg.save_entries(&store)?;
    let pkg_metadata = std::fs::metadata(&pkg_path)?;
    let pkg_size = pkg_metadata.len();

    let elapsed = start.elapsed();

    println!();
    println!("=== 结果 ===");
    println!("数据包路径: {}", pkg_path.display());
    println!("数据包大小: {}", format_file_size(pkg_size));
    println!("条目数量: {}", entry_count);
    println!("耗时: {:.1} 秒", elapsed.as_secs_f64());

    Ok(())
}
