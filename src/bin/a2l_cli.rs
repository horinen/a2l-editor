use a2l_editor::{
    A2lEntryStore, A2lGenerator, DataPackage, DwarfParser, ElfParser, Endianness, ExportKind,
    TypeInfo, TypeKind,
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
                eprintln!("用法: a2l-cli parse <elf文件>");
                return Ok(());
            }
            cmd_parse(&PathBuf::from(&args[2]))?;
        }
        "entries" => {
            if args.len() < 3 {
                eprintln!("用法: a2l-cli entries <elf文件> [搜索词] [-n 数量] [--a2l]");
                return Ok(());
            }
            let elf_path = PathBuf::from(&args[2]);
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
            let show_a2l = args.contains(&"--a2l".to_string());
            cmd_entries(&elf_path, search, limit, show_a2l)?;
        }
        "export" => {
            let output = args
                .iter()
                .position(|a| a == "-o")
                .and_then(|i| args.get(i + 1))
                .map(|s| s.to_string());
            let mode = args
                .iter()
                .position(|a| a == "-m")
                .and_then(|i| args.get(i + 1))
                .map(|s| s.as_str())
                .unwrap_or("measurement");
            if args.len() < 3 || output.is_none() {
                eprintln!(
                    "用法: a2l-cli export <elf文件> -o <a2l文件> [-m measurement|characteristic]"
                );
                return Ok(());
            }
            cmd_export(
                &PathBuf::from(&args[2]),
                &PathBuf::from(output.unwrap()),
                mode,
            )?;
        }
        "inspect" => {
            if args.len() < 3 {
                eprintln!("用法: a2l-cli inspect <elf文件> [选项]");
                eprintln!("  --structs [名称]         查找/列出结构体");
                eprintln!("  --vars [数量]            列出 DWARF 变量");
                eprintln!("  --types [数量]           按 kind 列出类型");
                eprintln!("  --arrays [数量]          列出数组类型");
                eprintln!("  --enums [数量]           列出枚举类型");
                eprintln!("  --bitfields [数量]       列出含位域的结构体");
                eprintln!("  --struct-instances [数量] 列出结构体实例变量");
                eprintln!("  --offset 0xHH            查看指定偏移处的类型");
                return Ok(());
            }
            cmd_inspect(&PathBuf::from(&args[2]), &args[3..])?;
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
    println!("  a2l-cli parse <elf文件>                     解析 ELF，显示变量统计与条目数");
    println!("  a2l-cli entries <elf文件> [搜索词] [选项]    列出 A2L 条目");
    println!("    -n <数量>    显示数量 (默认: 50)");
    println!("    --a2l        显示 A2L 块格式");
    println!("  a2l-cli export <elf文件> -o <a2l文件> [选项]  导出条目到 A2L 文件");
    println!("    -m <模式>    measurement (默认) 或 characteristic");
    println!("  a2l-cli inspect <elf文件> [选项]             DWARF 调试信息（诊断用）");
    println!("    --structs [名称]        查找/列出结构体");
    println!("    --vars [数量]           列出 DWARF 变量");
    println!("    --types [数量]          按 kind 列出类型");
    println!("    --arrays [数量]         列出数组类型");
    println!("    --enums [数量]          列出枚举类型");
    println!("    --bitfields [数量]      列出含位域的结构体");
    println!("    --struct-instances [数量] 列出结构体实例变量");
    println!("    --offset 0xHH           查看指定偏移处的类型");
}

fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/// 获取或生成数据包 + A2lEntryStore
/// 有数据包 → 加载；无数据包 → parse_deep → 生成数据包
fn ensure_package(elf_path: &PathBuf) -> Result<(A2lEntryStore, bool)> {
    if DataPackage::exists(elf_path) {
        let start = Instant::now();
        let pkg = DataPackage::open(elf_path)?;
        let store = pkg.load_entries()?;
        let elapsed = start.elapsed();
        println!(
            "从数据包加载: {} 条, 耗时 {} ms",
            store.len(),
            elapsed.as_millis()
        );
        Ok((store, true))
    } else {
        println!("数据包不存在，执行深度解析...");
        let start = Instant::now();

        let parser = ElfParser::parse_deep(elf_path)?;

        let store = parser
            .a2l_entries()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("未找到 A2L 条目（需要 DWARF 调试信息）"))?;

        let parse_time = start.elapsed();
        println!(
            "深度解析完成: {} 条目, 耗时 {} ms",
            store.len(),
            parse_time.as_millis()
        );

        let mut pkg = DataPackage::create(elf_path)?;
        pkg.save_entries(&store)?;
        let pkg_path = pkg.path().to_path_buf();
        let pkg_size = std::fs::metadata(&pkg_path)?.len();
        println!(
            "数据包已保存: {} ({})",
            pkg_path.display(),
            format_size(pkg_size)
        );

        Ok((store, false))
    }
}

fn cmd_parse(elf_path: &PathBuf) -> Result<()> {
    let file_size = std::fs::metadata(elf_path)?.len();
    println!("文件: {}", elf_path.display());
    println!("大小: {}", format_size(file_size));
    println!();

    let total_start = Instant::now();

    if DataPackage::exists(elf_path) {
        let start = Instant::now();
        let pkg = DataPackage::open(elf_path)?;
        let _meta = pkg.get_meta()?;
        let store = pkg.load_entries()?;
        let elapsed = start.elapsed();

        println!(
            "从数据包加载: {} 条, 耗时 {} ms",
            store.len(),
            elapsed.as_millis()
        );
        println!();
        println!("=== 统计 ===");
        println!("A2L 条目数: {}", store.len());
        println!(
            "耗时: {:.1} 秒 (从数据包)",
            total_start.elapsed().as_secs_f64()
        );
        return Ok(());
    }

    println!("数据包不存在，执行深度解析...");
    let start = Instant::now();
    let parser = ElfParser::parse_deep(elf_path)?;
    let parse_time = start.elapsed();

    let variables = parser.variables();
    let stats = parser.dwarf_stats();
    let store = parser
        .a2l_entries()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("未找到 A2L 条目（需要 DWARF 调试信息）"))?;

    println!(
        "深度解析完成: {} 条目, 耗时 {} ms",
        store.len(),
        parse_time.as_millis()
    );

    let mut pkg = DataPackage::create(elf_path)?;
    pkg.save_entries(&store)?;
    let pkg_path = pkg.path().to_path_buf();
    let pkg_size = std::fs::metadata(&pkg_path)?.len();
    println!(
        "数据包已保存: {} ({})",
        pkg_path.display(),
        format_size(pkg_size)
    );

    println!();
    println!("=== 统计 ===");
    println!("变量数: {}", variables.len());
    println!("A2L 条目数: {}", store.len());

    if let Some(ds) = stats {
        println!(
            "DWARF: {} 基础类型, {} 结构体, {} 联合体, {} 枚举, {} 数组",
            ds.base_types, ds.structs, ds.unions, ds.enums, ds.arrays
        );
        println!(
            "       {} 指针, {} typedef, {} 变量, {} 成员, {} 枚举值",
            ds.pointers, ds.typedefs, ds.variables, ds.struct_members, ds.enum_values
        );
    }

    let mut size_dist = std::collections::HashMap::new();
    let mut section_dist = std::collections::HashMap::new();
    let mut with_type_info = 0usize;
    for v in variables {
        *size_dist.entry(v.size).or_insert(0usize) += 1;
        *section_dist.entry(v.section.clone()).or_insert(0usize) += 1;
        if v.type_info.is_some() {
            with_type_info += 1;
        }
    }

    println!();
    if with_type_info > 0 {
        println!("含类型信息: {} / {}", with_type_info, variables.len());
    }

    println!("按大小分布:");
    let mut sizes: Vec<_> = size_dist.iter().collect();
    sizes.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
    for (size, count) in sizes.iter().take(10) {
        println!("  {} 字节: {} 个", size, count);
    }

    println!("按段分布:");
    let mut sections: Vec<_> = section_dist.iter().collect();
    sections.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
    for (section, count) in sections.iter().take(10) {
        let name = if section.is_empty() {
            "(未知)"
        } else {
            section
        };
        println!("  {}: {} 个", name, count);
    }

    println!();
    println!("总耗时: {:.1} 秒", total_start.elapsed().as_secs_f64());

    Ok(())
}

fn cmd_entries(
    elf_path: &PathBuf,
    search: Option<&str>,
    limit: usize,
    show_a2l: bool,
) -> Result<()> {
    let (store, _) = ensure_package(elf_path)?;

    println!();
    println!("总条目数: {}", store.len());

    let entries = if let Some(query) = search {
        store.search(query)
    } else {
        store.entries.iter().collect()
    };

    println!("匹配条目: {}", entries.len());
    println!();

    for entry in entries.iter().take(limit) {
        if show_a2l {
            let block = A2lGenerator::generate_measurement_block_with_compu(
                entry,
                None,
                None,
                Endianness::Little,
            );
            print!("{}", block);
        } else {
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
    }

    if entries.len() > limit {
        println!("... 还有 {} 条未显示", entries.len() - limit);
    }

    Ok(())
}

fn cmd_export(elf_path: &PathBuf, a2l_path: &PathBuf, mode: &str) -> Result<()> {
    let (store, _) = ensure_package(elf_path)?;

    let export_kind = match mode {
        "characteristic" => ExportKind::Characteristic,
        _ => ExportKind::Measurement,
    };

    println!();
    println!("导出 {} 条目到: {}", store.len(), a2l_path.display());

    let result =
        A2lGenerator::append_to_file(&store.entries, a2l_path, export_kind, Endianness::Little)?;

    println!("已添加: {}, 已跳过(重复): {}", result.added, result.skipped);

    Ok(())
}

fn cmd_inspect(elf_path: &PathBuf, opts: &[String]) -> Result<()> {
    let parser = DwarfParser::parse_from_file(elf_path)?;

    if !parser.has_dwarf_info() {
        println!("错误: ELF 文件不包含 DWARF 调试信息");
        return Ok(());
    }

    let did_something = inspect_structs(&parser, opts)
        | inspect_vars(&parser, opts)
        | inspect_types(&parser, opts)
        | inspect_arrays(&parser, opts)
        | inspect_enums(&parser, opts)
        | inspect_bitfields(&parser, opts)
        | inspect_struct_instances(&parser, opts)
        | inspect_offset(&parser, opts);

    if !did_something {
        println!("请指定至少一个检查选项（--structs, --vars, --types, --arrays, --enums, --bitfields, --struct-instances, --offset）");
    }

    Ok(())
}

fn opt_value<'a>(opts: &'a [String], flag: &str) -> Option<&'a str> {
    opts.iter().position(|a| a == flag).and_then(|i| {
        opts.get(i + 1)
            .filter(|v| !v.starts_with('-'))
            .map(|s| s.as_str())
    })
}

fn opt_count(opts: &[String], flag: &str, default: usize) -> usize {
    opts.iter()
        .position(|a| a == flag)
        .and_then(|i| {
            opts.get(i + 1)
                .filter(|v| !v.starts_with('-'))
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(default)
}

fn has_flag(opts: &[String], flag: &str) -> bool {
    opts.iter().any(|a| a == flag)
}

fn inspect_structs(parser: &DwarfParser, opts: &[String]) -> bool {
    if !has_flag(opts, "--structs") {
        return false;
    }

    if let Some(name) = opt_value(opts, "--structs") {
        println!("查找结构体: {}", name);
        println!();

        if let Some(si) = parser.find_struct_by_name(name) {
            print_struct_detail(si);
        } else {
            let partial = parser.find_structs_containing_member(name);
            if partial.is_empty() {
                println!("未找到匹配的结构体");
                let all = parser.list_structs();
                let similar: Vec<_> = all
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
                println!("找到 {} 个包含 '{}' 的结构体:", partial.len(), name);
                for (i, (si, member)) in partial.iter().take(10).enumerate() {
                    println!(
                        "{}. {} ({} 字节, {} 成员) 匹配: {} @ +{}",
                        i + 1,
                        si.name,
                        si.size,
                        si.members.len(),
                        member.name,
                        member.offset
                    );
                }
            }
        }
    } else {
        let structs = parser.list_structs();
        println!("=== 结构体 ({} 个) ===", structs.len());
        println!();
        for s in structs.iter().take(30) {
            println!("  {} ({} 字节, {} 成员)", s.name, s.size, s.members.len());
        }
        if structs.len() > 30 {
            println!("  ... 还有 {} 个", structs.len() - 30);
        }
    }

    println!();
    true
}

fn inspect_vars(parser: &DwarfParser, opts: &[String]) -> bool {
    if !has_flag(opts, "--vars") {
        return false;
    }

    let limit = opt_count(opts, "--vars", 20);

    let (total, resolved, unresolved) = parser.debug_type_resolution();
    println!("=== DWARF 变量 ===");
    println!("类型缓存: {}", parser.get_type_cache_size());
    println!("DWARF 变量: {}", total);
    println!("全局变量: {}", parser.global_variable_count());
    println!("已解析类型: {}, 未解析偏移: {}", resolved, unresolved);
    println!();

    let vars = parser.list_variables_with_types();
    for (name, type_name) in vars.iter().take(limit) {
        println!("{}: {}", name, type_name);
    }
    if vars.len() > limit {
        println!("... 共 {} 个", vars.len());
    }

    println!();
    true
}

fn inspect_types(parser: &DwarfParser, opts: &[String]) -> bool {
    if !has_flag(opts, "--types") {
        return false;
    }

    let limit = opt_count(opts, "--types", 30);
    let all = parser.all_types();

    println!("=== 类型统计 (共 {} 个) ===", all.len());
    println!();

    let mut kind_counts = std::collections::HashMap::new();
    for t in &all {
        *kind_counts.entry(format!("{:?}", t.kind)).or_insert(0usize) += 1;
    }
    let mut kinds: Vec<_> = kind_counts.into_iter().collect();
    kinds.sort_by(|a, b| b.1.cmp(&a.1));
    for (kind, count) in &kinds {
        println!("  {}: {}", kind, count);
    }

    println!();
    println!("前 {} 个类型:", limit);
    for t in all.iter().take(limit) {
        let extra = match t.kind {
            TypeKind::Array if !t.array_dims.is_empty() => format!(
                " dims=[{}]",
                t.array_dims
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join("x")
            ),
            TypeKind::Enum if !t.variants.is_empty() => format!(" {} variants", t.variants.len()),
            TypeKind::Struct | TypeKind::Union if !t.members.is_empty() => {
                format!(" {} members", t.members.len())
            }
            _ => String::new(),
        };
        println!("  {} ({} bytes, {:?}{})", t.name, t.size, t.kind, extra);
    }

    println!();
    true
}

fn inspect_arrays(parser: &DwarfParser, opts: &[String]) -> bool {
    if !has_flag(opts, "--arrays") {
        return false;
    }

    let limit = opt_count(opts, "--arrays", 20);
    let all = parser.all_types();
    let arrays: Vec<_> = all
        .iter()
        .filter(|t| t.kind == TypeKind::Array && !t.array_dims.is_empty())
        .take(limit)
        .collect();

    if arrays.is_empty() {
        println!("未找到有维度信息的数组类型");
    } else {
        println!("=== 数组类型 (前 {} 个) ===", arrays.len());
        println!();
        for arr in arrays {
            let total: usize = arr.array_dims.iter().product();
            let elem_size = if total > 0 { arr.size / total } else { 0 };
            let dims: Vec<String> = arr.array_dims.iter().map(|d| d.to_string()).collect();
            println!(
                "{} - {} bytes (元素: {} bytes)",
                dims.join("x"),
                arr.size,
                elem_size
            );
        }
    }

    println!();
    true
}

fn inspect_enums(parser: &DwarfParser, opts: &[String]) -> bool {
    if !has_flag(opts, "--enums") {
        return false;
    }

    let limit = opt_count(opts, "--enums", 20);
    let all = parser.all_types();
    let enums: Vec<_> = all
        .iter()
        .filter(|t| t.kind == TypeKind::Enum && !t.variants.is_empty())
        .take(limit)
        .collect();

    if enums.is_empty() {
        println!("未找到有变体信息的枚举类型");
    } else {
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
    }

    true
}

fn inspect_bitfields(parser: &DwarfParser, opts: &[String]) -> bool {
    if !has_flag(opts, "--bitfields") {
        return false;
    }

    let limit = opt_count(opts, "--bitfields", 20);
    let structs = parser.list_structs();
    let mut bf_structs = Vec::new();
    for s in structs {
        let bf_members: Vec<_> = s.members.iter().filter(|m| m.is_bitfield()).collect();
        if !bf_members.is_empty() {
            bf_structs.push((s, bf_members));
        }
    }

    if bf_structs.is_empty() {
        println!("未找到含位域的结构体");
    } else {
        println!(
            "=== 含位域的结构体 (前 {} 个) ===",
            limit.min(bf_structs.len())
        );
        println!();
        for (i, (s, members)) in bf_structs.iter().take(limit).enumerate() {
            println!("{}. {} ({} bytes)", i + 1, s.name, s.size);
            for m in members {
                let (bo, bs) = (m.bit_offset.unwrap_or(0), m.bit_size.unwrap_or(0));
                println!(
                    "   {} @ +{} bits [{},{}] ({} bits)",
                    m.name,
                    m.offset,
                    bo,
                    bo + bs - 1,
                    bs
                );
            }
            println!();
        }
        println!("共 {} 个含位域的结构体", bf_structs.len());
    }

    true
}

fn inspect_struct_instances(parser: &DwarfParser, opts: &[String]) -> bool {
    if !has_flag(opts, "--struct-instances") {
        return false;
    }

    let limit = opt_count(opts, "--struct-instances", 20);
    let instances = parser.list_struct_instance_variables();

    if instances.is_empty() {
        println!("未找到结构体实例变量");
    } else {
        println!(
            "=== 结构体实例变量 (前 {} 个) ===",
            limit.min(instances.len())
        );
        println!();
        for (name, ti) in instances.iter().take(limit) {
            println!(
                "{}: {} ({} bytes, {} members)",
                name,
                ti.name,
                ti.size,
                ti.members.len()
            );
        }
        println!();
        println!("共 {} 个结构体实例变量", instances.len());
    }

    println!();
    true
}

fn inspect_offset(parser: &DwarfParser, opts: &[String]) -> bool {
    if !has_flag(opts, "--offset") {
        return false;
    }

    let offset_str = match opt_value(opts, "--offset") {
        Some(s) => s,
        None => {
            println!("--offset 需要指定偏移量，如 --offset 0x1A2B");
            return true;
        }
    };

    let offset = match u64::from_str_radix(offset_str.trim_start_matches("0x"), 16) {
        Ok(v) => v,
        Err(_) => {
            println!("无效偏移量: {}", offset_str);
            return true;
        }
    };

    parser.check_type_at_offset(offset);
    true
}

fn print_struct_detail(si: &TypeInfo) {
    println!("=== {} ===", si.name);
    println!("大小: {} 字节", si.size);
    println!("成员数: {}", si.members.len());
    println!();
    for m in &si.members {
        println!(
            "  {:30} @ +{:<4} ({} bytes, type: {})",
            m.name, m.offset, m.type_size, m.type_name
        );
    }
}
