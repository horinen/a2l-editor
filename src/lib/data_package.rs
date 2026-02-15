use crate::types::{A2lEntry, A2lEntryStore};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

pub struct DataPackage {
    db: Connection,
    path: PathBuf,
}

#[derive(Clone)]
pub struct PackageMeta {
    pub file_name: String,
    pub elf_path: Option<String>,
    pub entry_count: usize,
    pub created_at: i64,
}

impl DataPackage {
    pub fn get_package_path(elf_path: &Path) -> PathBuf {
        elf_path.with_extension("elf.a2ldata")
    }

    pub fn exists(elf_path: &Path) -> bool {
        let package_path = Self::get_package_path(elf_path);
        package_path.exists()
    }

    pub fn open(elf_path: &Path) -> Result<Self> {
        let package_path = Self::get_package_path(elf_path);
        Self::open_path(&package_path)
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
                created_at INTEGER
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
                array_index TEXT
            );
            
            CREATE INDEX IF NOT EXISTS idx_a2l_entries_name ON a2l_entries(full_name);
            "#,
        )
        .context("无法创建数据包表")?;

        Ok(Self {
            db,
            path: path.to_path_buf(),
        })
    }

    pub fn create(elf_path: &Path) -> Result<Self> {
        let package_path = Self::get_package_path(elf_path);
        let file_name = elf_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let db = Connection::open(&package_path).context("无法创建数据包")?;

        db.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS meta (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                file_name TEXT,
                elf_path TEXT,
                entry_count INTEGER DEFAULT 0,
                created_at INTEGER
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
                array_index TEXT
            );
            
            CREATE INDEX IF NOT EXISTS idx_a2l_entries_name ON a2l_entries(full_name);
            
            INSERT OR REPLACE INTO meta (id, file_name, elf_path, created_at)
            VALUES (1, ?1, ?2, ?3);
            "#,
        )
        .context("无法初始化数据包")?;

        let created_at = chrono::Utc::now().timestamp();
        db.execute(
            "INSERT OR REPLACE INTO meta (id, file_name, elf_path, created_at) VALUES (1, ?1, ?2, ?3)",
            params![file_name, elf_path.to_string_lossy().to_string(), created_at],
        )?;

        Ok(Self {
            db,
            path: package_path,
        })
    }

    pub fn create_at(path: &Path, elf_path: &Path) -> Result<Self> {
        let file_name = elf_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let db = Connection::open(path).context("无法创建数据包")?;

        db.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS meta (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                file_name TEXT,
                elf_path TEXT,
                entry_count INTEGER DEFAULT 0,
                created_at INTEGER
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
                array_index TEXT
            );
            
            CREATE INDEX IF NOT EXISTS idx_a2l_entries_name ON a2l_entries(full_name);
            "#,
        )
        .context("无法初始化数据包")?;

        let created_at = chrono::Utc::now().timestamp();
        db.execute(
            "INSERT OR REPLACE INTO meta (id, file_name, elf_path, created_at) VALUES (1, ?1, ?2, ?3)",
            params![file_name, elf_path.to_string_lossy().to_string(), created_at],
        )?;

        Ok(Self {
            db,
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get_meta(&self) -> Result<PackageMeta> {
        let meta = self
            .db
            .query_row(
                "SELECT file_name, elf_path, entry_count, created_at FROM meta WHERE id = 1",
                [],
                |row| {
                    Ok(PackageMeta {
                        file_name: row.get(0)?,
                        elf_path: row.get(1)?,
                        entry_count: row.get::<_, i64>(2)? as usize,
                        created_at: row.get(3)?,
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
                     (full_name, address, size, a2l_type, type_name, bit_offset, bit_size, array_index)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
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
                ])
                .context("无法插入条目")?;
            }
        }

        tx.execute(
            "UPDATE meta SET entry_count = ?1 WHERE id = 1",
            params![entry_count as i64],
        )?;

        tx.commit().context("无法提交事务")?;

        Ok(())
    }

    pub fn load_entries(&self) -> Result<A2lEntryStore> {
        let mut stmt = self.db.prepare(
            "SELECT full_name, address, size, a2l_type, type_name, bit_offset, bit_size, array_index 
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
