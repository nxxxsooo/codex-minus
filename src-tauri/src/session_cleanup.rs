//! Permanent, explicitly selected session deletion. No backup store is involved.
//! All known databases participate in one transaction per ID. SQL changes run
//! before unlinking; records commit only after file removal succeeds. A failed
//! removal/commit can leave some files removed, but keeps the rows for retry.
//! Permanent deletion deliberately cannot promise cross-file rollback.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, bail};
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CleanupRequest {
    pub session_ids: Vec<String>,
    pub archived_only: bool,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupOutcome {
    pub deleted_count: usize,
    pub failures: Vec<CleanupFailure>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupFailure {
    pub session_id: String,
    pub message: String,
}

pub fn permanently_delete(
    home: &Path,
    db_paths: &[PathBuf],
    request: &CleanupRequest,
) -> anyhow::Result<CleanupOutcome> {
    let ids: BTreeSet<_> = request.session_ids.iter().map(|id| id.trim()).collect();
    if ids.is_empty() || ids.contains("") {
        bail!("请选择有效的会话 ID。");
    }
    let home = home.canonicalize()?;
    let paths = db_paths
        .iter()
        .filter(|path| path.exists())
        .map(|path| {
            let path = path.canonicalize()?;
            if !path.starts_with(&home) {
                bail!("会话数据库不在 Codex 目录内。");
            }
            Ok(path)
        })
        .collect::<anyhow::Result<BTreeSet<_>>>()?;
    let mut paths = paths.into_iter();
    let first = paths.next().context("未找到会话数据库。")?;
    let mut db = Connection::open_with_flags(first, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    db.busy_timeout(Duration::from_secs(2))?;
    db.pragma_update(None, "foreign_keys", true)?;
    let mut schemas = vec!["main".to_string()];
    for (index, path) in paths.enumerate() {
        let schema = format!("session_db_{index}");
        db.execute(
            &format!("ATTACH DATABASE ?1 AS {schema}"),
            [path.to_string_lossy()],
        )?;
        schemas.push(schema);
    }
    let mut outcome = CleanupOutcome::default();
    for id in ids {
        match delete_one(&mut db, &schemas, &home, id, request.archived_only) {
            Ok(()) => outcome.deleted_count += 1,
            Err(error) => outcome.failures.push(CleanupFailure {
                session_id: id.to_string(),
                message: format!("{error:#}"),
            }),
        }
    }
    Ok(outcome)
}

fn has_table(db: &Connection, schema: &str, table: &str) -> anyhow::Result<bool> {
    Ok(db
        .query_row(
            &format!("SELECT 1 FROM {schema}.sqlite_master WHERE type = 'table' AND name = ?1"),
            [table],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn delete_one(
    db: &mut Connection,
    schemas: &[String],
    home: &Path,
    id: &str,
    archived_only: bool,
) -> anyhow::Result<()> {
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let mut found = false;
    let mut rollouts = BTreeSet::new();
    for schema in schemas {
        if has_table(&tx, schema, "threads")? {
            let archived = if archived_only {
                "COALESCE(archived, 0)"
            } else {
                "1"
            };
            let row = tx
                .query_row(
                    &format!("SELECT rollout_path, {archived} FROM {schema}.threads WHERE id = ?1"),
                    [id],
                    |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?;
            if let Some((rollout, archived)) = row {
                if archived == 0 {
                    bail!("会话已恢复为活动状态，请刷新列表后重试。");
                }
                found = true;
                if let Some(path) = rollout.filter(|path| !path.trim().is_empty()) {
                    if let Some(path) = checked_rollout(home, &path)? {
                        rollouts.insert(path);
                    }
                }
            }
        }
        if has_table(&tx, schema, "automation_runs")? {
            let state = if archived_only {
                "COALESCE(status, '')"
            } else {
                "'archived'"
            };
            let mut query = tx.prepare(&format!(
                "SELECT {state} FROM {schema}.automation_runs WHERE thread_id = ?1"
            ))?;
            for state in query.query_map([id], |row| row.get::<_, String>(0))? {
                if !state?.eq_ignore_ascii_case("archived") {
                    bail!("会话仍有关联的活动自动化记录，未删除。");
                }
                found = true;
            }
        }
    }
    if !found {
        bail!("未找到会话，请刷新列表。");
    }
    // Check every database, including references using a different spelling of
    // the same path. A file shared with an unselected ID must never be removed.
    if !rollouts.is_empty() {
        for schema in schemas {
            if !has_table(&tx, schema, "threads")? {
                continue;
            }
            let mut query = tx.prepare(&format!(
                "SELECT rollout_path FROM {schema}.threads WHERE id <> ?1"
            ))?;
            for value in query.query_map([id], |row| row.get::<_, Option<String>>(0))? {
                if let Some(path) = value?.and_then(|path| fs::canonicalize(path).ok()) {
                    if rollouts.contains(&path) {
                        bail!("会话文件被其他会话引用，未删除。");
                    }
                }
            }
        }
    }
    for schema in schemas {
        for (table, predicate) in [
            ("thread_dynamic_tools", "thread_id = ?1"),
            ("thread_goals", "thread_id = ?1"),
            (
                "thread_spawn_edges",
                "parent_thread_id = ?1 OR child_thread_id = ?1",
            ),
            ("stage1_outputs", "thread_id = ?1"),
            ("inbox_items", "thread_id = ?1"),
            ("automation_runs", "thread_id = ?1"),
        ] {
            if has_table(&tx, schema, table)? {
                tx.execute(
                    &format!("DELETE FROM {schema}.{table} WHERE {predicate}"),
                    [id],
                )?;
            }
        }
        if has_table(&tx, schema, "agent_job_items")? {
            tx.execute(&format!("UPDATE {schema}.agent_job_items SET assigned_thread_id = NULL WHERE assigned_thread_id = ?1"), [id])?;
        }
        if has_table(&tx, schema, "threads")? {
            tx.execute(&format!("DELETE FROM {schema}.threads WHERE id = ?1"), [id])?;
        }
    }
    for path in rollouts {
        fs::remove_file(&path)
            .with_context(|| format!("会话文件清理未完成，记录已保留供重试：{}", path.display()))?;
    }
    tx.commit()
        .context("会话文件已清理，但数据库提交失败，请重试清理")?;
    Ok(())
}

fn checked_rollout(home: &Path, value: &str) -> anyhow::Result<Option<PathBuf>> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        bail!("会话文件路径必须为绝对路径。");
    }
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        bail!("会话文件不是普通文件，未删除。");
    }
    let canonical = path.canonicalize()?;
    if ![home.join("sessions"), home.join("archived_sessions")]
        .iter()
        .any(|root| canonical.starts_with(root))
        || canonical.extension().and_then(|ext| ext.to_str()) != Some("jsonl")
    {
        bail!("会话文件不在 Codex 会话目录内，未删除。");
    }
    Ok(Some(canonical))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        home: tempfile::TempDir,
        paths: Vec<PathBuf>,
    }

    impl Fixture {
        fn new() -> Self {
            let home = tempfile::tempdir().unwrap();
            fs::create_dir(home.path().join("sessions")).unwrap();
            fs::create_dir(home.path().join("archived_sessions")).unwrap();
            fs::create_dir(home.path().join("sqlite")).unwrap();
            let paths = vec![
                home.path().join("state_5.sqlite"),
                home.path().join("sqlite/codex-dev.db"),
            ];
            let db = Connection::open(&paths[0]).unwrap();
            db.execute_batch("CREATE TABLE threads(id TEXT PRIMARY KEY, title TEXT, rollout_path TEXT, archived INTEGER);
                CREATE TABLE thread_dynamic_tools(thread_id TEXT REFERENCES threads(id) ON DELETE CASCADE);
                CREATE TABLE thread_attachments(thread_id TEXT REFERENCES threads(id) ON DELETE CASCADE);
                CREATE TABLE thread_spawn_edges(parent_thread_id TEXT, child_thread_id TEXT);
                CREATE TABLE thread_goals(thread_id TEXT);
                CREATE TABLE stage1_outputs(thread_id TEXT);
                CREATE TABLE agent_job_items(id TEXT, assigned_thread_id TEXT REFERENCES threads(id));").unwrap();
            Connection::open(&paths[1])
                .unwrap()
                .execute_batch(
                    "CREATE TABLE automation_runs(thread_id TEXT PRIMARY KEY, status TEXT);
                 CREATE TABLE inbox_items(thread_id TEXT);",
                )
                .unwrap();
            Self { home, paths }
        }

        fn seed(&self, id: &str, archived: bool) -> PathBuf {
            let dir = if archived {
                "archived_sessions"
            } else {
                "sessions"
            };
            let path = self.home.path().join(dir).join(format!("{id}.jsonl"));
            fs::write(&path, format!("{{\"id\":\"{id}\"}}\n")).unwrap();
            self.db(0)
                .execute(
                    "INSERT INTO threads VALUES (?1, ?1, ?2, ?3)",
                    rusqlite::params![id, path.to_string_lossy(), archived],
                )
                .unwrap();
            path
        }

        fn db(&self, index: usize) -> Connection {
            Connection::open(&self.paths[index]).unwrap()
        }

        fn cleanup(&self, ids: &[&str], archived_only: bool) -> CleanupOutcome {
            permanently_delete(
                self.home.path(),
                &self.paths,
                &CleanupRequest {
                    session_ids: ids.iter().map(|id| id.to_string()).collect(),
                    archived_only,
                },
            )
            .unwrap()
        }

        fn exists(&self, id: &str) -> bool {
            self.db(0)
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM threads WHERE id = ?1)",
                    [id],
                    |row| row.get(0),
                )
                .unwrap()
        }
    }

    #[test]
    fn deletes_selected_files_and_related_rows_without_backups() {
        let f = Fixture::new();
        let removed = f.seed("selected", true);
        let kept = f.seed("keep", false);
        let before = fs::read(&kept).unwrap();
        f.db(0).execute_batch("INSERT INTO thread_dynamic_tools VALUES ('selected'), ('keep');
            INSERT INTO thread_attachments VALUES ('selected'), ('keep');
            INSERT INTO thread_goals VALUES ('selected'), ('keep');
            INSERT INTO stage1_outputs VALUES ('selected'), ('keep');
            INSERT INTO thread_spawn_edges VALUES ('selected', 'keep'), ('keep', 'selected'), ('keep', 'other');
            INSERT INTO agent_job_items VALUES ('job', 'selected');").unwrap();
        f.db(1)
            .execute_batch(
                "INSERT INTO automation_runs VALUES ('selected', 'archived'), ('keep', 'pending');
            INSERT INTO inbox_items VALUES ('selected'), ('keep');",
            )
            .unwrap();
        let outcome = f.cleanup(&["selected", "selected"], true);
        assert_eq!(outcome.deleted_count, 1);
        assert!(outcome.failures.is_empty());
        assert!(!removed.exists());
        assert!(!f.exists("selected"));
        assert!(f.exists("keep"));
        assert_eq!(fs::read(kept).unwrap(), before);
        for table in [
            "thread_dynamic_tools",
            "thread_attachments",
            "thread_goals",
            "stage1_outputs",
            "thread_spawn_edges",
        ] {
            let count: i64 = f
                .db(0)
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 1, "{table}");
        }
        let assignment: Option<String> = f
            .db(0)
            .query_row(
                "SELECT assigned_thread_id FROM agent_job_items",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(assignment.is_none());
        for table in ["automation_runs", "inbox_items"] {
            let id: String = f
                .db(1)
                .query_row(&format!("SELECT thread_id FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(id, "keep");
        }
        let names: BTreeSet<_> = fs::read_dir(f.home.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(
            names,
            ["sessions", "archived_sessions", "sqlite", "state_5.sqlite"]
                .map(Into::into)
                .into_iter()
                .collect()
        );
        assert_eq!(
            fs::read_dir(f.home.path().join("archived_sessions"))
                .unwrap()
                .count(),
            0
        );
    }

    #[test]
    fn all_archived_rechecks_state_and_preserves_active_sessions() {
        let f = Fixture::new();
        let active = f.seed("restored", false);
        f.seed("archived", true);
        let result = f.cleanup(&["restored", "archived"], true);
        assert_eq!(result.deleted_count, 1);
        assert_eq!(result.failures[0].session_id, "restored");
        assert!(active.exists());
        assert!(f.exists("restored"));
        let result = f.cleanup(&["restored"], false);
        assert_eq!(result.deleted_count, 1);
        assert!(!active.exists());
    }

    #[test]
    fn duplicate_databases_are_cleaned_and_missing_rollouts_do_not_block() {
        let mut f = Fixture::new();
        let path = f.seed("duplicate", true);
        fs::remove_file(path).unwrap();
        let extra = f.home.path().join("sqlite/old.db");
        f.db(0)
            .execute("VACUUM INTO ?1", [extra.to_string_lossy()])
            .unwrap();
        f.paths.push(extra);
        let result = f.cleanup(&["duplicate"], true);
        assert_eq!(result.deleted_count, 1);
        assert!(!f.exists("duplicate"));
        let count: i64 = f
            .db(2)
            .query_row("SELECT COUNT(*) FROM threads", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn sql_failure_in_one_database_preserves_all_rows_and_files() {
        let f = Fixture::new();
        let path = f.seed("blocked", true);
        f.db(1).execute_batch("INSERT INTO automation_runs VALUES ('blocked', 'archived');
            CREATE TRIGGER refuse_delete BEFORE DELETE ON automation_runs BEGIN SELECT RAISE(ABORT, 'fixture refusal'); END;").unwrap();
        let result = f.cleanup(&["blocked"], true);
        assert_eq!(result.deleted_count, 0);
        assert_eq!(result.failures.len(), 1);
        assert!(path.exists());
        assert!(f.exists("blocked"));
        let count: i64 = f
            .db(1)
            .query_row("SELECT COUNT(*) FROM automation_runs", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn outside_and_shared_paths_fail_without_deleting_records() {
        let f = Fixture::new();
        f.seed("unsafe", true);
        let outside = f.home.path().join("important.jsonl");
        fs::write(&outside, "keep").unwrap();
        f.db(0)
            .execute(
                "UPDATE threads SET rollout_path = ?1 WHERE id = 'unsafe'",
                [outside.to_string_lossy()],
            )
            .unwrap();
        assert_eq!(f.cleanup(&["unsafe"], false).failures.len(), 1);
        assert_eq!(fs::read_to_string(outside).unwrap(), "keep");
        assert!(f.exists("unsafe"));
        let shared = f.seed("shared", true);
        f.db(0)
            .execute(
                "UPDATE threads SET rollout_path = ?1 WHERE id = 'unsafe'",
                [shared.to_string_lossy()],
            )
            .unwrap();
        assert_eq!(f.cleanup(&["shared"], false).failures.len(), 1);
        assert!(shared.exists());
        assert!(f.exists("shared"));
    }

    #[test]
    fn empty_and_unknown_ids_cannot_become_delete_all() {
        let f = Fixture::new();
        let path = f.seed("keep", true);
        for ids in [vec![], vec!["".to_string()]] {
            assert!(
                permanently_delete(
                    f.home.path(),
                    &f.paths,
                    &CleanupRequest {
                        session_ids: ids,
                        archived_only: false
                    }
                )
                .is_err()
            );
        }
        assert_eq!(f.cleanup(&["unknown"], true).failures.len(), 1);
        assert!(path.exists());
        assert!(f.exists("keep"));
    }

    #[test]
    fn automation_only_session_and_inbox_are_removed() {
        let f = Fixture::new();
        f.db(1).execute_batch("INSERT INTO automation_runs VALUES ('auto', 'archived'); INSERT INTO inbox_items VALUES ('auto');").unwrap();
        assert_eq!(f.cleanup(&["auto"], true).deleted_count, 1);
        for table in ["automation_runs", "inbox_items"] {
            assert_eq!(
                f.db(1)
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlink_and_unlink_failure_keep_records_for_retry() {
        let f = Fixture::new();
        let path = f.seed("link", true);
        fs::remove_file(&path).unwrap();
        let keep = f.seed("keep", false);
        std::os::unix::fs::symlink(&keep, &path).unwrap();
        assert_eq!(f.cleanup(&["link"], false).failures.len(), 1);
        assert!(keep.exists());
        assert!(f.exists("link"));
        fs::remove_file(path).unwrap();

        use std::os::unix::fs::PermissionsExt;
        let blocked = f.seed("blocked", true);
        let dir = blocked.parent().unwrap();
        fs::set_permissions(dir, fs::Permissions::from_mode(0o500)).unwrap();
        let result = f.cleanup(&["blocked"], true);
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(result.deleted_count, 0);
        assert!(f.exists("blocked"));
        assert!(blocked.exists());
        assert_eq!(f.cleanup(&["blocked"], true).deleted_count, 1);
    }
}
