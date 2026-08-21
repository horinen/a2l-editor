use crate::types::{A2lEntry, A2lEntryStore};
use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection};
use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime};

pub struct DataPackage {
    db: Connection,
    path: PathBuf,
    lock_path: Option<PathBuf>,
}

#[derive(Clone)]
pub struct PackageMeta {
    pub file_name: String,
    pub elf_path: Option<String>,
    pub entry_count: usize,
    pub created_at: i64,
    pub parser_version: String,
    pub elf_mtime: i64,
}

fn elf_mtime(elf_path: &Path) -> Result<i64> {
    let mtime = std::fs::metadata(elf_path)?
        .modified()
        .context("无法读取 ELF 修改时间")?;
    Ok(mtime
        .duration_since(SystemTime::UNIX_EPOCH)
        .context("ELF 修改时间早于 Unix 纪元")?
        .as_secs() as i64)
}

impl DataPackage {
    const PARSER_VERSION: &str = env!("CARGO_PKG_VERSION");

    pub fn get_package_path(elf_path: &Path) -> PathBuf {
        elf_path.with_extension("elf.a2ldata")
    }

    pub fn exists(elf_path: &Path) -> bool {
        let package_path = Self::get_package_path(elf_path);
        package_path.exists()
    }

    pub fn open(elf_path: &Path) -> Result<Self> {
        let package_path = Self::get_package_path(elf_path);
        let package = Self::open_path(&package_path)?;

        // 校验 ELF 修改时间与数据包记录是否一致
        let meta = package.get_meta()?;
        if elf_mtime(elf_path)? != meta.elf_mtime {
            bail!("ELF 已修改，与数据包不匹配，请重新生成缓存");
        }

        Ok(package)
    }

    pub fn open_path(path: &Path) -> Result<Self> {
        let db = Connection::open(path).context("无法打开数据包")?;

        db.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS meta (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                file_name TEXT,
                elf_path TEXT,
                entry_count INTEGER DEFAULT 0,
                created_at INTEGER,
                parser_version TEXT NOT NULL,
                elf_mtime INTEGER
            );
            
            CREATE TABLE IF NOT EXISTS a2l_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                full_name TEXT NOT NULL,
                address INTEGER NOT NULL,
                size INTEGER NOT NULL,
                a2l_type TEXT NOT NULL,
                type_name TEXT NOT NULL,
                bit_offset INTEGER,
                bit_size INTEGER,
                array_index TEXT,
                symbol_link_name TEXT,
                symbol_link_offset INTEGER
            );
            
            CREATE INDEX IF NOT EXISTS idx_a2l_entries_name ON a2l_entries(full_name);
            "#,
        )
        .context("无法创建数据包表")?;

        let package = Self {
            db,
            path: path.to_path_buf(),
            lock_path: None,
        };
        package.validate_parser_version()?;
        Ok(package)
    }

    pub fn create(elf_path: &Path) -> Result<Self> {
        let package_path = Self::get_package_path(elf_path);
        let lock_path = Self::acquire_create_lock(&package_path)?;
        let file_name = elf_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        if package_path.exists() {
            std::fs::remove_file(&package_path).context("无法替换旧数据包")?;
        }
        let db = Connection::open(&package_path).context("无法创建数据包")?;

        db.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS meta (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                file_name TEXT,
                elf_path TEXT,
                entry_count INTEGER DEFAULT 0,
                created_at INTEGER,
                parser_version TEXT NOT NULL,
                elf_mtime INTEGER
            );
            
            CREATE TABLE IF NOT EXISTS a2l_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                full_name TEXT NOT NULL,
                address INTEGER NOT NULL,
                size INTEGER NOT NULL,
                a2l_type TEXT NOT NULL,
                type_name TEXT NOT NULL,
                bit_offset INTEGER,
                bit_size INTEGER,
                array_index TEXT,
                symbol_link_name TEXT,
                symbol_link_offset INTEGER
            );
            
            CREATE INDEX IF NOT EXISTS idx_a2l_entries_name ON a2l_entries(full_name);
            "#,
        )
        .context("无法初始化数据包")?;

        let created_at = chrono::Utc::now().timestamp();
        db.execute(
            "INSERT OR REPLACE INTO meta (id, file_name, elf_path, created_at, parser_version, elf_mtime) VALUES (1, ?1, ?2, ?3, ?4, ?5)",
            params![
                file_name,
                elf_path.to_string_lossy().to_string(),
                created_at,
                Self::PARSER_VERSION,
                elf_mtime(elf_path)?,
            ],
        )?;

        Ok(Self {
            db,
            path: package_path,
            lock_path: Some(lock_path),
        })
    }

    pub fn create_at(path: &Path, elf_path: &Path) -> Result<Self> {
        let lock_path = Self::acquire_create_lock(path)?;
        let file_name = elf_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        if path.exists() {
            std::fs::remove_file(path).context("无法替换旧数据包")?;
        }
        let db = Connection::open(path).context("无法创建数据包")?;

        db.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS meta (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                file_name TEXT,
                elf_path TEXT,
                entry_count INTEGER DEFAULT 0,
                created_at INTEGER,
                parser_version TEXT NOT NULL,
                elf_mtime INTEGER
            );
            
            CREATE TABLE IF NOT EXISTS a2l_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                full_name TEXT NOT NULL,
                address INTEGER NOT NULL,
                size INTEGER NOT NULL,
                a2l_type TEXT NOT NULL,
                type_name TEXT NOT NULL,
                bit_offset INTEGER,
                bit_size INTEGER,
                array_index TEXT,
                symbol_link_name TEXT,
                symbol_link_offset INTEGER
            );
            
            CREATE INDEX IF NOT EXISTS idx_a2l_entries_name ON a2l_entries(full_name);
            "#,
        )
        .context("无法初始化数据包")?;

        let created_at = chrono::Utc::now().timestamp();
        db.execute(
            "INSERT OR REPLACE INTO meta (id, file_name, elf_path, created_at, parser_version, elf_mtime) VALUES (1, ?1, ?2, ?3, ?4, ?5)",
            params![
                file_name,
                elf_path.to_string_lossy().to_string(),
                created_at,
                Self::PARSER_VERSION,
                elf_mtime(elf_path)?,
            ],
        )?;

        Ok(Self {
            db,
            path: path.to_path_buf(),
            lock_path: Some(lock_path),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn lock_path_for(path: &Path) -> PathBuf {
        path.with_extension("a2ldata.lock")
    }

    fn acquire_create_lock(path: &Path) -> Result<PathBuf> {
        let lock_path = Self::lock_path_for(path);
        let timeout = Duration::from_secs(120);
        let start = SystemTime::now();

        loop {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(file) => {
                    drop(file);
                    return Ok(lock_path);
                }
                Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                    if start.elapsed().unwrap_or_default() > timeout {
                        bail!("等待数据包生成锁超时: {}", lock_path.display());
                    }
                    thread::sleep(Duration::from_millis(200));
                }
                Err(err) => {
                    return Err(err)
                        .with_context(|| format!("无法创建数据包生成锁: {}", lock_path.display()));
                }
            }
        }
    }

    fn release_create_lock(&mut self) {
        if let Some(lock_path) = self.lock_path.take() {
            let _ = std::fs::remove_file(lock_path);
        }
    }

    fn validate_parser_version(&self) -> Result<()> {
        let version: String = self
            .db
            .query_row("SELECT parser_version FROM meta WHERE id = 1", [], |row| {
                row.get(0)
            })
            .context("数据包版本过旧，请重新生成")?;

        if version != Self::PARSER_VERSION {
            bail!(
                "数据包由解析器版本 {} 生成，当前版本为 {}，请重新生成",
                version,
                Self::PARSER_VERSION
            );
        }

        Ok(())
    }

    pub fn get_meta(&self) -> Result<PackageMeta> {
        let meta = self
            .db
            .query_row(
                "SELECT file_name, elf_path, entry_count, created_at, parser_version, elf_mtime FROM meta WHERE id = 1",
                [],
                |row| {
                    Ok(PackageMeta {
                        file_name: row.get(0)?,
                        elf_path: row.get(1)?,
                        entry_count: row.get::<_, i64>(2)? as usize,
                        created_at: row.get(3)?,
                        parser_version: row.get(4)?,
                        elf_mtime: row.get(5)?,
                    })
                },
            )
            .context("无法读取数据包元信息")?;

        Ok(meta)
    }

    pub fn save_entries(&mut self, store: &A2lEntryStore) -> Result<()> {
        let entry_count = store.len();

        let tx = self.db.transaction().context("无法开始事务")?;

        tx.execute("DELETE FROM a2l_entries", [])
            .context("无法清除旧条目")?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO a2l_entries 
                     (full_name, address, size, a2l_type, type_name, bit_offset, bit_size, array_index, symbol_link_name, symbol_link_offset)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                )
                .context("无法准备插入语句")?;

            for entry in &store.entries {
                let array_index_str = entry
                    .array_index
                    .as_ref()
                    .filter(|v| !v.is_empty())
                    .map(|v| serde_json::to_string(v).unwrap_or_default());

                stmt.execute(params![
                    entry.full_name,
                    entry.address,
                    entry.size,
                    entry.a2l_type,
                    entry.type_name,
                    entry.bit_offset,
                    entry.bit_size,
                    array_index_str,
                    entry.symbol_link_name,
                    entry.symbol_link_offset,
                ])
                .context("无法插入条目")?;
            }
        }

        tx.execute(
            "UPDATE meta SET entry_count = ?1 WHERE id = 1",
            params![entry_count as i64],
        )?;

        tx.commit().context("无法提交事务")?;
        self.release_create_lock();

        Ok(())
    }

    pub fn load_entries(&self) -> Result<A2lEntryStore> {
        let mut stmt = self.db.prepare(
            "SELECT full_name, address, size, a2l_type, type_name, bit_offset, bit_size, array_index, symbol_link_name, symbol_link_offset 
             FROM a2l_entries ORDER BY full_name"
        ).context("无法准备查询")?;

        let entries = stmt
            .query_map([], |row| {
                let full_name: String = row.get(0)?;
                let address: u64 = row.get(1)?;
                let size: usize = row.get(2)?;
                let a2l_type: String = row.get(3)?;
                let type_name: String = row.get(4)?;
                let bit_offset: Option<usize> = row.get(5)?;
                let bit_size: Option<usize> = row.get(6)?;
                let array_index_str: Option<String> = row.get(7)?;
                let symbol_link_name: Option<String> = row.get(8)?;
                let symbol_link_offset: Option<u64> = row.get(9)?;

                let array_index =
                    array_index_str.and_then(|s| serde_json::from_str::<Vec<usize>>(&s).ok());

                let mut entry = A2lEntry::new(full_name, address, size, a2l_type, type_name);
                if let (Some(bo), Some(bs)) = (bit_offset, bit_size) {
                    entry = entry.with_bitfield(bo, bs);
                }
                if let Some(idx) = array_index {
                    if !idx.is_empty() {
                        entry = entry.with_array_index(idx);
                    }
                }
                if let (Some(name), Some(offset)) = (symbol_link_name, symbol_link_offset) {
                    entry = entry.with_symbol_link(name, offset);
                }

                Ok(entry)
            })
            .context("无法查询条目")?
            .collect::<Result<Vec<_>, _>>()
            .context("无法解析条目")?;

        let mut store = A2lEntryStore::new();
        for entry in entries {
            store.add(entry);
        }

        Ok(store)
    }

    pub fn entry_count(&self) -> Result<usize> {
        let count: i64 = self
            .db
            .query_row("SELECT entry_count FROM meta WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        Ok(count as usize)
    }
}

impl Drop for DataPackage {
    fn drop(&mut self) {
        self.release_create_lock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::fs;

    fn temp_path(name: &str) -> PathBuf {
        let unique = format!(
            "a2l-editor-{}-{}-{}",
            name,
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        std::env::temp_dir().join(unique)
    }

    fn temp_elf(name: &str) -> PathBuf {
        let elf_path = temp_path(name).with_extension("elf");
        fs::write(&elf_path, b"elf").unwrap();
        elf_path
    }

    #[test]
    fn new_package_records_parser_version() {
        let elf_path = temp_elf("package-version");
        let db_path = elf_path.with_extension("elf.a2ldata");

        let package = DataPackage::create_at(&db_path, &elf_path).unwrap();        let meta = package.get_meta().unwrap();

        assert_eq!(meta.parser_version, env!("CARGO_PKG_VERSION"));
        drop(package);
        let reopened = DataPackage::open_path(&db_path).unwrap();
        assert_eq!(
            reopened.get_meta().unwrap().parser_version,
            env!("CARGO_PKG_VERSION")
        );
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn create_at_replaces_old_schema_package() {
        let elf_path = temp_elf("replace-old-package");
        let db_path = elf_path.with_extension("elf.a2ldata");
        let db = Connection::open(&db_path).unwrap();
        db.execute_batch(
            r#"
            CREATE TABLE meta (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                file_name TEXT,
                elf_path TEXT,
                entry_count INTEGER DEFAULT 0,
                created_at INTEGER
            );
            INSERT INTO meta (id, file_name, elf_path, entry_count, created_at)
            VALUES (1, "old.elf", "old.elf", 0, 0);
            "#,
        )
        .unwrap();
        drop(db);

        let package = DataPackage::create_at(&db_path, &elf_path).unwrap();
        assert_eq!(
            package.get_meta().unwrap().parser_version,
            env!("CARGO_PKG_VERSION")
        );
        drop(package);
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn create_lock_is_removed_after_save() {
        let elf_path = temp_elf("locked-package");
        let db_path = elf_path.with_extension("elf.a2ldata");
        let lock_path = DataPackage::lock_path_for(&db_path);

        let mut package = DataPackage::create_at(&db_path, &elf_path).unwrap();
        assert!(lock_path.exists());

        package.save_entries(&A2lEntryStore::new()).unwrap();
        assert!(!lock_path.exists());

        drop(package);
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn open_rejects_modified_elf() {
        let dir = temp_path("mtime-check");
        fs::create_dir_all(&dir).unwrap();
        let elf_path = dir.join("sample.elf");
        fs::write(&elf_path, b"elf").unwrap();
        let db_path = dir.join("sample.elf.a2ldata");

        let mut package = DataPackage::create(&elf_path).unwrap();
        let _ = package.save_entries(&A2lEntryStore::new()).unwrap();
        drop(package);

        // ELF 未修改时可正常打开
        assert!(DataPackage::open(&elf_path).is_ok());

        // 模拟 ELF 重新编译：修改内容并更新时间戳
        let new_mtime = SystemTime::now() + Duration::from_secs(10);
        fs::write(&elf_path, b"elf-new").unwrap();
        let f = fs::File::options().write(true).open(&elf_path).unwrap();
        f.set_modified(new_mtime).unwrap();
        drop(f);

        let err = match DataPackage::open(&elf_path) {
            Ok(_) => panic!("ELF 修改后不应打开数据包"),
            Err(err) => err.to_string(),
        };
        assert!(err.contains("请重新生成缓存"));

        let _ = fs::remove_file(&db_path);
        let _ = fs::remove_file(&elf_path);
    }

    #[test]
    fn old_package_without_parser_version_is_rejected() {
        let db_path = temp_path("old-package.a2ldata");
        let db = Connection::open(&db_path).unwrap();
        db.execute_batch(
            r#"
            CREATE TABLE meta (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                file_name TEXT,
                elf_path TEXT,
                entry_count INTEGER DEFAULT 0,
                created_at INTEGER
            );
            INSERT INTO meta (id, file_name, elf_path, entry_count, created_at)
            VALUES (1, "sample.elf", "sample.elf", 0, 0);
            "#,
        )
        .unwrap();
        drop(db);

        let err = match DataPackage::open_path(&db_path) {
            Ok(_) => panic!("旧数据包不应被打开"),
            Err(err) => err.to_string(),
        };
        assert!(err.contains("数据包版本过旧"));
        let _ = fs::remove_file(db_path);
    }
}
