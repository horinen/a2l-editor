use crate::types::{A2lEntry, A2lEntryStore, CacheEntry, TypeInfo, Variable};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::PathBuf;

pub struct Cache {
    db: Connection,
    cache_dir: PathBuf,
}

impl Cache {
    pub fn open() -> Result<Self> {
        let cache_dir = Self::get_cache_dir()?;
        std::fs::create_dir_all(&cache_dir).context("无法创建缓存目录")?;

        let db_path = cache_dir.join("cache.db");
        let db = Connection::open(&db_path).context("无法打开缓存数据库")?;

        db.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS cache_entries (
                file_hash TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                modified_time INTEGER NOT NULL,
                variable_count INTEGER NOT NULL,
                parse_time_ms INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                has_dwarf INTEGER DEFAULT 0,
                a2l_entry_count INTEGER DEFAULT 0
            );
            
            CREATE TABLE IF NOT EXISTS variables (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_hash TEXT NOT NULL,
                name TEXT NOT NULL,
                address INTEGER NOT NULL,
                size INTEGER NOT NULL,
                type_name TEXT NOT NULL,
                section TEXT NOT NULL,
                type_info BLOB,
                FOREIGN KEY (file_hash) REFERENCES cache_entries(file_hash)
            );
            
            CREATE TABLE IF NOT EXISTS a2l_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_hash TEXT NOT NULL,
                full_name TEXT NOT NULL,
                address INTEGER NOT NULL,
                size INTEGER NOT NULL,
                a2l_type TEXT NOT NULL,
                type_name TEXT NOT NULL,
                bit_offset INTEGER,
                bit_size INTEGER,
                array_index TEXT,
                FOREIGN KEY (file_hash) REFERENCES cache_entries(file_hash)
            );
            
            CREATE INDEX IF NOT EXISTS idx_variables_hash ON variables(file_hash);
            CREATE INDEX IF NOT EXISTS idx_variables_name ON variables(name);
            CREATE INDEX IF NOT EXISTS idx_a2l_entries_hash ON a2l_entries(file_hash);
            CREATE INDEX IF NOT EXISTS idx_a2l_entries_name ON a2l_entries(full_name);
            "#,
        )
        .context("无法创建缓存表")?;

        Ok(Self { db, cache_dir })
    }

    fn get_cache_dir() -> Result<PathBuf> {
        let base = if cfg!(target_os = "windows") {
            std::env::var("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."))
        } else if cfg!(target_os = "macos") {
            dirs::data_dir().unwrap_or_else(|| PathBuf::from("."))
        } else {
            std::env::var("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    std::env::var("HOME")
                        .map(|h| PathBuf::from(h).join(".cache"))
                        .unwrap_or_else(|_| PathBuf::from("."))
                })
        };

        Ok(base.join("a2l-editor"))
    }

    pub fn get(&self, hash: &str) -> Result<Option<(CacheEntry, Vec<Variable>)>> {
        let mut stmt = self
            .db
            .prepare(
                "SELECT file_path, file_size, modified_time, variable_count, parse_time_ms, created_at, COALESCE(has_dwarf, 0)
             FROM cache_entries WHERE file_hash = ?1",
            )
            .context("无法准备查询语句")?;

        let entry = stmt.query_row(params![hash], |row| {
            Ok(CacheEntry {
                file_hash: hash.to_string(),
                file_path: row.get(0)?,
                file_size: row.get(1)?,
                modified_time: row.get(2)?,
                variable_count: row.get(3)?,
                parse_time_ms: row.get(4)?,
                created_at: row.get(5)?,
                has_dwarf: row.get::<_, i64>(6)? != 0,
            })
        });

        let entry = match entry {
            Ok(e) => e,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e).context("无法查询缓存条目"),
        };

        let mut stmt = self
            .db
            .prepare(
                "SELECT name, address, size, type_name, section, type_info FROM variables WHERE file_hash = ?1 ORDER BY name",
            )
            .context("无法准备变量查询")?;

        let variables = stmt
            .query_map(params![hash], |row| {
                let name: String = row.get(0)?;
                let address: u64 = row.get(1)?;
                let size: usize = row.get(2)?;
                let type_name: String = row.get(3)?;
                let section: String = row.get(4)?;
                let type_info_blob: Option<Vec<u8>> = row.get(5)?;

                let type_info =
                    type_info_blob.and_then(|blob| bincode::deserialize::<TypeInfo>(&blob).ok());

                let mut var = Variable::new(name, address, size, type_name, section);
                var.type_info = type_info;
                Ok(var)
            })
            .context("无法查询变量")?
            .collect::<Result<Vec<_>, _>>()
            .context("无法解析变量")?;

        Ok(Some((entry, variables)))
    }

    pub fn save(&mut self, hash: &str, entry: &CacheEntry, variables: &[Variable]) -> Result<()> {
        let tx = self.db.transaction().context("无法开始事务")?;

        tx.execute(
            "INSERT OR REPLACE INTO cache_entries 
             (file_hash, file_path, file_size, modified_time, variable_count, parse_time_ms, created_at, has_dwarf)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                hash,
                entry.file_path,
                entry.file_size,
                entry.modified_time,
                entry.variable_count,
                entry.parse_time_ms,
                entry.created_at,
                entry.has_dwarf as i64,
            ],
        )
        .context("无法保存缓存条目")?;

        tx.execute("DELETE FROM variables WHERE file_hash = ?1", params![hash])
            .context("无法清除旧变量")?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO variables (file_hash, name, address, size, type_name, section, type_info)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                )
                .context("无法准备变量插入语句")?;

            for var in variables {
                let type_info_blob = var
                    .type_info
                    .as_ref()
                    .and_then(|t| bincode::serialize(t).ok());

                stmt.execute(params![
                    hash,
                    var.name,
                    var.address,
                    var.size,
                    var.type_name,
                    var.section,
                    type_info_blob,
                ])
                .context("无法插入变量")?;
            }
        }

        tx.commit().context("无法提交事务")?;

        Ok(())
    }

    pub fn exists(&self, hash: &str) -> bool {
        self.db
            .query_row(
                "SELECT 1 FROM cache_entries WHERE file_hash = ?1",
                params![hash],
                |_| Ok(()),
            )
            .is_ok()
    }

    pub fn has_dwarf(&self, hash: &str) -> bool {
        self.db
            .query_row(
                "SELECT has_dwarf FROM cache_entries WHERE file_hash = ?1",
                params![hash],
                |row| Ok(row.get::<_, i64>(0)? != 0),
            )
            .unwrap_or(false)
    }

    pub fn list(&self) -> Result<Vec<CacheEntry>> {
        let mut stmt = self
            .db
            .prepare(
                "SELECT file_hash, file_path, file_size, modified_time, variable_count, parse_time_ms, created_at, COALESCE(has_dwarf, 0)
             FROM cache_entries ORDER BY created_at DESC",
            )
            .context("无法准备列表查询")?;

        let entries = stmt
            .query_map([], |row| {
                Ok(CacheEntry {
                    file_hash: row.get(0)?,
                    file_path: row.get(1)?,
                    file_size: row.get(2)?,
                    modified_time: row.get(3)?,
                    variable_count: row.get(4)?,
                    parse_time_ms: row.get(5)?,
                    created_at: row.get(6)?,
                    has_dwarf: row.get::<_, i64>(7)? != 0,
                })
            })
            .context("无法查询缓存列表")?
            .collect::<Result<Vec<_>, _>>()
            .context("无法解析缓存列表")?;

        Ok(entries)
    }

    pub fn delete(&self, hash: &str) -> Result<()> {
        self.db
            .execute(
                "DELETE FROM a2l_entries WHERE file_hash = ?1",
                params![hash],
            )
            .context("无法删除 A2L 条目")?;
        self.db
            .execute("DELETE FROM variables WHERE file_hash = ?1", params![hash])
            .context("无法删除变量")?;
        self.db
            .execute(
                "DELETE FROM cache_entries WHERE file_hash = ?1",
                params![hash],
            )
            .context("无法删除缓存条目")?;
        Ok(())
    }

    pub fn clear_old(&self, days: u32) -> Result<usize> {
        let cutoff = chrono::Utc::now().timestamp() - (days as i64 * 24 * 60 * 60);

        let hashes: Vec<String> = self
            .db
            .prepare("SELECT file_hash FROM cache_entries WHERE created_at < ?1")?
            .query_map(params![cutoff], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        let count = hashes.len();
        for hash in hashes {
            self.delete(&hash)?;
        }

        Ok(count)
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    pub fn get_a2l_entries(&self, hash: &str) -> Result<Option<A2lEntryStore>> {
        let count: i64 = self
            .db
            .query_row(
                "SELECT COALESCE(a2l_entry_count, 0) FROM cache_entries WHERE file_hash = ?1",
                params![hash],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if count == 0 {
            return Ok(None);
        }

        let mut stmt = self
            .db
            .prepare(
                "SELECT full_name, address, size, a2l_type, type_name, bit_offset, bit_size, array_index 
                 FROM a2l_entries WHERE file_hash = ?1 ORDER BY full_name",
            )
            .context("无法准备 A2L 条目查询")?;

        let entries = stmt
            .query_map(params![hash], |row| {
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
            .context("无法查询 A2L 条目")?
            .collect::<Result<Vec<_>, _>>()
            .context("无法解析 A2L 条目")?;

        let mut store = A2lEntryStore::new();
        for entry in entries {
            store.add(entry);
        }

        Ok(Some(store))
    }

    pub fn save_a2l_entries(
        &mut self,
        hash: &str,
        entry_count: usize,
        store: &A2lEntryStore,
    ) -> Result<()> {
        self.db
            .execute(
                "UPDATE cache_entries SET a2l_entry_count = ?1 WHERE file_hash = ?2",
                params![entry_count as i64, hash],
            )
            .context("无法更新条目计数")?;

        self.db
            .execute(
                "DELETE FROM a2l_entries WHERE file_hash = ?1",
                params![hash],
            )
            .context("无法清除旧 A2L 条目")?;

        let tx = self.db.transaction().context("无法开始事务")?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO a2l_entries 
                     (file_hash, full_name, address, size, a2l_type, type_name, bit_offset, bit_size, array_index)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                )
                .context("无法准备 A2L 条目插入语句")?;

            for entry in &store.entries {
                let array_index_str = entry
                    .array_index
                    .as_ref()
                    .filter(|v| !v.is_empty())
                    .map(|v| serde_json::to_string(v).unwrap_or_default());

                stmt.execute(params![
                    hash,
                    entry.full_name,
                    entry.address,
                    entry.size,
                    entry.a2l_type,
                    entry.type_name,
                    entry.bit_offset,
                    entry.bit_size,
                    array_index_str,
                ])
                .context("无法插入 A2L 条目")?;
            }
        }

        tx.commit().context("无法提交事务")?;

        Ok(())
    }
}
