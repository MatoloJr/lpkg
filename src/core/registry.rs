use crate::models::RegistryEntry;
use crate::config::registry_path;
use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};

pub struct InstallRegistry {
    conn: Connection,
}

impl InstallRegistry {
    pub fn open() -> Result<Self> {
        let path = registry_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)
            .with_context(|| format!("failed to open registry at {}", path.display()))?;
        let registry = Self { conn };
        registry.init_schema()?;
        Ok(registry)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS installs (
                canonical_id TEXT NOT NULL,
                backend TEXT NOT NULL,
                backend_id TEXT NOT NULL,
                version TEXT NOT NULL,
                installed_at TEXT NOT NULL,
                binary_paths TEXT DEFAULT '[]',
                desktop_file TEXT,
                PRIMARY KEY (canonical_id, backend, backend_id)
            );
            CREATE TABLE IF NOT EXISTS pins (
                canonical_id TEXT NOT NULL,
                backend TEXT NOT NULL,
                backend_id TEXT NOT NULL,
                reason TEXT,
                PRIMARY KEY (canonical_id, backend, backend_id)
            );
            CREATE TABLE IF NOT EXISTS scan_cache (
                backend TEXT PRIMARY KEY,
                data TEXT NOT NULL,
                cached_at TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    pub fn record_install(&self, entry: &RegistryEntry) -> Result<()> {
        let paths_json = serde_json::to_string(&entry.binary_paths)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO installs
             (canonical_id, backend, backend_id, version, installed_at, binary_paths, desktop_file)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.canonical_id,
                entry.backend,
                entry.backend_id,
                entry.version,
                entry.installed_at.to_rfc3339(),
                paths_json,
                entry.desktop_file,
            ],
        )?;
        Ok(())
    }

    pub fn remove_install(&self, canonical_id: &str, backend: &str, backend_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM installs WHERE canonical_id = ?1 AND backend = ?2 AND backend_id = ?3",
            params![canonical_id, backend, backend_id],
        )?;
        Ok(())
    }

    pub fn find_by_canonical(&self, canonical_id: &str) -> Result<Vec<RegistryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT canonical_id, backend, backend_id, version, installed_at, binary_paths, desktop_file
             FROM installs WHERE canonical_id = ?1",
        )?;
        let rows = stmt.query_map(params![canonical_id], |row| {
            let paths_json: String = row.get(5)?;
            let binary_paths: Vec<String> = serde_json::from_str(&paths_json).unwrap_or_default();
            let installed_at: String = row.get(4)?;
            Ok(RegistryEntry {
                canonical_id: row.get(0)?,
                backend: row.get(1)?,
                backend_id: row.get(2)?,
                version: row.get(3)?,
                installed_at: installed_at.parse().unwrap_or_else(|_| Utc::now()),
                binary_paths,
                desktop_file: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_all(&self) -> Result<Vec<RegistryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT canonical_id, backend, backend_id, version, installed_at, binary_paths, desktop_file
             FROM installs",
        )?;
        let rows = stmt.query_map([], |row| {
            let paths_json: String = row.get(5)?;
            let binary_paths: Vec<String> = serde_json::from_str(&paths_json).unwrap_or_default();
            let installed_at: String = row.get(4)?;
            Ok(RegistryEntry {
                canonical_id: row.get(0)?,
                backend: row.get(1)?,
                backend_id: row.get(2)?,
                version: row.get(3)?,
                installed_at: installed_at.parse().unwrap_or_else(|_| Utc::now()),
                binary_paths,
                desktop_file: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn add_pin(&self, canonical_id: &str, backend: &str, backend_id: &str, reason: Option<&str>) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO pins (canonical_id, backend, backend_id, reason) VALUES (?1, ?2, ?3, ?4)",
            params![canonical_id, backend, backend_id, reason],
        )?;
        Ok(())
    }

    pub fn remove_pin(&self, canonical_id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM pins WHERE canonical_id = ?1", params![canonical_id])?;
        Ok(())
    }

    pub fn list_pins(&self) -> Result<Vec<(String, String, String, Option<String>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT canonical_id, backend, backend_id, reason FROM pins",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn cache_scan(&self, backend: &str, data: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO scan_cache (backend, data, cached_at) VALUES (?1, ?2, ?3)",
            params![backend, data, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn get_cached_scan(&self, backend: &str) -> Result<Option<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT data, cached_at FROM scan_cache WHERE backend = ?1",
        )?;
        let mut rows = stmt.query(params![backend])?;
        if let Some(row) = rows.next()? {
            Ok(Some((row.get(0)?, row.get(1)?)))
        } else {
            Ok(None)
        }
    }
}
