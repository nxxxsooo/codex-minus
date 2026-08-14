//! Active-only provider adaptation.
//!
//! The pinned core's `provider_sync` rewrites the whole history by construction — its rollout
//! walk hard-codes `archived_sessions` beside `sessions`, and its sqlite update is a WHERE-less
//! `UPDATE threads SET model_provider = …` — so the manager never calls it. This module is the
//! narrow counterpart: it receives the active mismatched sessions and touches exactly one rollout
//! file and one sqlite row per session, addressed by the `rollout_path`/`db_path` the inventory
//! carries. Archived history is unreachable by construction: there is no directory walk and no
//! table-wide statement here at all.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const BACKUP_NAMESPACE: &str = "codex-minus-session-adapt";
const BACKUP_MANAGED_BY: &str = "codex-minus session adapt";
const BACKUP_KEEP_COUNT: usize = 5;

#[derive(Debug, Default)]
pub struct SessionAdaptationOutcome {
    pub adapted: usize,
    pub skipped_locked: usize,
    pub failed: usize,
    pub encrypted_sessions: usize,
    pub backup_dir: Option<PathBuf>,
}

#[derive(Debug)]
struct PlannedChange {
    session_id: String,
    rollout_path: PathBuf,
    db_path: PathBuf,
    original_text: Option<String>,
    next_text: Option<String>,
    original_mtime: Option<SystemTime>,
}

pub fn adapt_active_sessions_to_provider(
    home: &Path,
    target_provider: &str,
    sessions: &[codex_plus_data::LocalSession],
) -> anyhow::Result<SessionAdaptationOutcome> {
    let target = target_provider.trim();
    if target.is_empty() {
        anyhow::bail!("target provider is empty");
    }
    let mut outcome = SessionAdaptationOutcome::default();
    let mut planned = Vec::new();
    for session in sessions {
        if session.archived || session.model_provider == target || session.id.trim().is_empty() {
            continue;
        }
        let rollout_path = PathBuf::from(&session.rollout_path);
        let mut change = PlannedChange {
            session_id: session.id.clone(),
            rollout_path: rollout_path.clone(),
            db_path: PathBuf::from(&session.db_path),
            original_text: None,
            next_text: None,
            original_mtime: None,
        };
        if !session.rollout_path.trim().is_empty() && rollout_path.exists() {
            let text = match fs::read_to_string(&rollout_path) {
                Ok(text) => text,
                Err(error) if is_locked_io_error(&error) => {
                    // A rollout held open by a running Codex is skipped whole — rewriting only
                    // its sqlite row would leave the session split across two providers.
                    outcome.skipped_locked += 1;
                    continue;
                }
                Err(error) => return Err(error.into()),
            };
            if text.contains("encrypted_content") {
                outcome.encrypted_sessions += 1;
            }
            let rewrite = rewrite_rollout_provider(&text, target)?;
            if rewrite.rewrite_needed {
                change.original_mtime = fs::metadata(&rollout_path).and_then(|m| m.modified()).ok();
                change.original_text = Some(text);
                change.next_text = Some(rewrite.next_text);
            }
        }
        planned.push(change);
    }
    if planned.is_empty() {
        return Ok(outcome);
    }
    let backup_dir = create_backup(home, target, &planned)?;
    for change in &planned {
        if let (Some(original), Some(next)) = (&change.original_text, &change.next_text) {
            match fs::write(&change.rollout_path, next) {
                Ok(()) => restore_file_mtime(&change.rollout_path, change.original_mtime),
                Err(error) if is_locked_io_error(&error) => {
                    outcome.skipped_locked += 1;
                    continue;
                }
                Err(_) => {
                    outcome.failed += 1;
                    continue;
                }
            }
            if update_thread_provider(&change.db_path, &change.session_id, target).is_err() {
                // The rollout already moved; put it back so the session stays whole.
                let _ = fs::write(&change.rollout_path, original);
                restore_file_mtime(&change.rollout_path, change.original_mtime);
                outcome.failed += 1;
                continue;
            }
        } else if update_thread_provider(&change.db_path, &change.session_id, target).is_err() {
            outcome.failed += 1;
            continue;
        }
        outcome.adapted += 1;
    }
    outcome.backup_dir = Some(backup_dir);
    prune_backups(home)?;
    Ok(outcome)
}

#[derive(Debug, Default)]
struct RolloutRewrite {
    next_text: String,
    rewrite_needed: bool,
}

/// Same on-disk semantics the core defined for provider sync: only `session_meta` lines change,
/// only their payload `model_provider`, and every other byte — including line endings and
/// unparseable lines — passes through untouched.
fn rewrite_rollout_provider(text: &str, target_provider: &str) -> anyhow::Result<RolloutRewrite> {
    let mut rewrite = RolloutRewrite::default();
    for segment in text.split_inclusive('\n') {
        let (line, line_ending) = split_line_ending(segment);
        let mut next_line = line.to_string();
        if !line.trim().is_empty() {
            if let Ok(mut record) = serde_json::from_str::<serde_json::Value>(line) {
                if record.get("type").and_then(serde_json::Value::as_str) == Some("session_meta") {
                    if let Some(payload) = record
                        .get_mut("payload")
                        .and_then(serde_json::Value::as_object_mut)
                    {
                        if payload
                            .get("model_provider")
                            .and_then(serde_json::Value::as_str)
                            != Some(target_provider)
                        {
                            payload.insert(
                                "model_provider".to_string(),
                                serde_json::json!(target_provider),
                            );
                            next_line = serde_json::to_string(&record)?;
                            rewrite.rewrite_needed = true;
                        }
                    }
                }
            }
        }
        rewrite.next_text.push_str(&next_line);
        rewrite.next_text.push_str(line_ending);
    }
    Ok(rewrite)
}

fn split_line_ending(segment: &str) -> (&str, &str) {
    if let Some(line) = segment.strip_suffix("\r\n") {
        (line, "\r\n")
    } else if let Some(line) = segment.strip_suffix('\n') {
        (line, "\n")
    } else {
        (segment, "")
    }
}

fn is_locked_io_error(error: &std::io::Error) -> bool {
    matches!(error.kind(), std::io::ErrorKind::PermissionDenied)
        || matches!(error.raw_os_error(), Some(32 | 33))
}

fn restore_file_mtime(path: &Path, mtime: Option<SystemTime>) {
    let Some(mtime) = mtime else { return };
    let Ok(file) = fs::File::options().write(true).open(path) else {
        return;
    };
    let times = fs::FileTimes::new().set_modified(mtime);
    let _ = file.set_times(times);
}

/// One row, by id, only when it differs. The threads-table guard mirrors the core's: a database
/// without the column is left alone rather than errored on.
fn update_thread_provider(
    db_path: &Path,
    thread_id: &str,
    target_provider: &str,
) -> anyhow::Result<()> {
    if !db_path.exists() {
        return Ok(());
    }
    let db = rusqlite::Connection::open(db_path)?;
    let mut stmt = db.prepare("PRAGMA table_info(\"threads\")")?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(stmt);
    if !columns.iter().any(|column| column == "model_provider") {
        return Ok(());
    }
    db.execute(
        "UPDATE threads SET model_provider = ?1 WHERE id = ?2 AND COALESCE(model_provider, '') <> ?1",
        rusqlite::params![target_provider, thread_id],
    )?;
    Ok(())
}

fn create_backup(
    home: &Path,
    target_provider: &str,
    planned: &[PlannedChange],
) -> anyhow::Result<PathBuf> {
    let backup_root = home.join("backups_state").join(BACKUP_NAMESPACE);
    let mut backup_dir = backup_root.join(timestamp_name());
    let mut suffix = 0;
    while backup_dir.exists() {
        suffix += 1;
        backup_dir = backup_root.join(format!("{}-{suffix}", timestamp_name()));
    }
    let rollout_dir = backup_dir.join("rollouts");
    fs::create_dir_all(&rollout_dir)?;
    let mut rollout_files = Vec::new();
    for change in planned {
        let Some(original) = &change.original_text else {
            continue;
        };
        let name = change
            .rollout_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("{}.jsonl", change.session_id));
        fs::write(rollout_dir.join(&name), original)?;
        rollout_files.push(name);
    }
    let db_dir = backup_dir.join("db");
    let mut db_files = Vec::new();
    let db_paths = planned
        .iter()
        .map(|change| change.db_path.clone())
        .collect::<BTreeSet<_>>();
    for db_path in db_paths {
        for source in codex_plus_core::codex_sqlite::codex_sqlite_sidecar_paths(&db_path) {
            if !source.exists() {
                continue;
            }
            let relative = codex_plus_core::codex_sqlite::relative_to_codex_home(home, &source);
            let target = db_dir.join(&relative);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source, &target)?;
            db_files.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    fs::write(
        backup_dir.join("metadata.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "version": 1,
            "namespace": BACKUP_NAMESPACE,
            "codexHome": home.to_string_lossy(),
            "targetProvider": target_provider,
            "createdAt": chrono::Utc::now().to_rfc3339(),
            "sessionIds": planned.iter().map(|change| change.session_id.clone()).collect::<Vec<_>>(),
            "rolloutFiles": rollout_files,
            "dbFiles": db_files,
            "managedBy": BACKUP_MANAGED_BY
        }))?,
    )?;
    Ok(backup_dir)
}

/// Prunes only directories carrying this feature's own marker: the core's provider-sync backups
/// share `backups_state/` and must never be this feature's to delete.
fn prune_backups(home: &Path) -> anyhow::Result<()> {
    let root = home.join("backups_state").join(BACKUP_NAMESPACE);
    if !root.exists() {
        return Ok(());
    }
    let mut managed = Vec::new();
    for entry in fs::read_dir(&root)? {
        let path = entry?.path();
        if !path.is_dir() {
            continue;
        }
        let Ok(text) = fs::read_to_string(path.join("metadata.json")) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        if value.get("managedBy").and_then(serde_json::Value::as_str) == Some(BACKUP_MANAGED_BY) {
            managed.push(path);
        }
    }
    managed.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    for path in managed.into_iter().skip(BACKUP_KEEP_COUNT) {
        let _ = fs::remove_dir_all(path);
    }
    Ok(())
}

fn timestamp_name() -> String {
    chrono::Local::now().format("%Y%m%d%H%M%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(
        id: &str,
        archived: bool,
        provider: &str,
        rollout_path: &Path,
        db_path: &Path,
    ) -> codex_plus_data::LocalSession {
        codex_plus_data::LocalSession {
            id: id.to_string(),
            title: String::new(),
            cwd: String::new(),
            model_provider: provider.to_string(),
            archived,
            updated_at_ms: Some(1_700_000_000_000),
            rollout_path: rollout_path.to_string_lossy().to_string(),
            db_path: db_path.to_string_lossy().to_string(),
        }
    }

    fn rollout_text(id: &str, provider: &str, extra: &str) -> String {
        format!(
            "{}\n{{\"type\":\"message\",\"payload\":{{\"text\":\"hello{extra}\"}}}}\n",
            serde_json::json!({
                "type": "session_meta",
                "payload": {"id": id, "cwd": "/tmp/project", "model_provider": provider}
            })
        )
    }

    fn seed_db(path: &Path, rows: &[(&str, &str)]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let db = rusqlite::Connection::open(path).unwrap();
        db.execute(
            "CREATE TABLE threads (id TEXT PRIMARY KEY, model_provider TEXT)",
            [],
        )
        .unwrap();
        for (id, provider) in rows {
            db.execute(
                "INSERT INTO threads (id, model_provider) VALUES (?1, ?2)",
                rusqlite::params![id, provider],
            )
            .unwrap();
        }
    }

    fn provider_of(db_path: &Path, id: &str) -> String {
        let db = rusqlite::Connection::open(db_path).unwrap();
        db.query_row(
            "SELECT model_provider FROM threads WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .unwrap()
    }

    #[test]
    fn adapts_active_mismatches_and_leaves_archived_history_byte_identical() {
        let home = tempfile::tempdir().unwrap();
        let sessions_dir = home.path().join("sessions");
        let archived_dir = home.path().join("archived_sessions");
        fs::create_dir_all(&sessions_dir).unwrap();
        fs::create_dir_all(&archived_dir).unwrap();
        let db_path = home.path().join("sqlite/codex-dev.db");
        seed_db(
            &db_path,
            &[("a", "custom"), ("b", "custom"), ("c", "custom")],
        );
        let rollout_a = sessions_dir.join("rollout-a.jsonl");
        let rollout_b = sessions_dir.join("rollout-b.jsonl");
        let rollout_c = archived_dir.join("rollout-c.jsonl");
        fs::write(&rollout_a, rollout_text("a", "custom", "")).unwrap();
        fs::write(
            &rollout_b,
            rollout_text("b", "custom", " encrypted_content"),
        )
        .unwrap();
        let archived_original = rollout_text("c", "custom", "");
        fs::write(&rollout_c, &archived_original).unwrap();
        let sessions = vec![
            session("a", false, "custom", &rollout_a, &db_path),
            session("b", false, "custom", &rollout_b, &db_path),
            session("c", true, "custom", &rollout_c, &db_path),
        ];

        let outcome = adapt_active_sessions_to_provider(home.path(), "OpenAI", &sessions).unwrap();

        assert_eq!(outcome.adapted, 2);
        assert_eq!(outcome.failed, 0);
        assert_eq!(outcome.skipped_locked, 0);
        assert_eq!(outcome.encrypted_sessions, 1);
        for (rollout, id) in [(&rollout_a, "a"), (&rollout_b, "b")] {
            let text = fs::read_to_string(rollout).unwrap();
            assert!(text.contains("\"model_provider\":\"OpenAI\""), "{text}");
            assert_eq!(provider_of(&db_path, id), "OpenAI");
        }
        // The archived session is untouched in both stores even though it mismatches too.
        assert_eq!(fs::read_to_string(&rollout_c).unwrap(), archived_original);
        assert_eq!(provider_of(&db_path, "c"), "custom");
        // The backup carries the exact originals of what was rewritten — and nothing archived.
        let backup_dir = outcome.backup_dir.unwrap();
        let backed_up = fs::read_dir(backup_dir.join("rollouts"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            backed_up,
            BTreeSet::from(["rollout-a.jsonl".to_string(), "rollout-b.jsonl".to_string()])
        );
        assert_eq!(
            fs::read_to_string(backup_dir.join("rollouts/rollout-a.jsonl")).unwrap(),
            rollout_text("a", "custom", "")
        );
    }

    #[test]
    fn a_session_without_a_rollout_file_still_repairs_its_row() {
        let home = tempfile::tempdir().unwrap();
        let db_path = home.path().join("sqlite/codex-dev.db");
        seed_db(&db_path, &[("a", "custom")]);
        let missing = home.path().join("sessions/rollout-missing.jsonl");
        let sessions = vec![session("a", false, "custom", &missing, &db_path)];

        let outcome = adapt_active_sessions_to_provider(home.path(), "OpenAI", &sessions).unwrap();

        assert_eq!(outcome.adapted, 1);
        assert_eq!(provider_of(&db_path, "a"), "OpenAI");
    }

    #[test]
    fn a_matching_or_empty_target_never_writes() {
        let home = tempfile::tempdir().unwrap();
        let db_path = home.path().join("sqlite/codex-dev.db");
        seed_db(&db_path, &[("a", "OpenAI")]);
        let rollout = home.path().join("sessions/rollout-a.jsonl");
        fs::create_dir_all(rollout.parent().unwrap()).unwrap();
        fs::write(&rollout, rollout_text("a", "OpenAI", "")).unwrap();
        let sessions = vec![session("a", false, "OpenAI", &rollout, &db_path)];

        let outcome = adapt_active_sessions_to_provider(home.path(), "OpenAI", &sessions).unwrap();
        assert_eq!(outcome.adapted, 0);
        assert!(outcome.backup_dir.is_none());
        assert!(adapt_active_sessions_to_provider(home.path(), "  ", &sessions).is_err());
    }

    #[test]
    fn pruning_keeps_recent_runs_and_only_its_own_marker() {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join("backups_state").join(BACKUP_NAMESPACE);
        for index in 0..7 {
            let dir = root.join(format!("2026010100000{index}"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("metadata.json"),
                serde_json::json!({"managedBy": BACKUP_MANAGED_BY}).to_string(),
            )
            .unwrap();
        }
        let foreign = root.join("29990101000000");
        fs::create_dir_all(&foreign).unwrap();
        fs::write(
            foreign.join("metadata.json"),
            serde_json::json!({"managedBy": "Codex++ provider sync"}).to_string(),
        )
        .unwrap();

        prune_backups(home.path()).unwrap();

        let survivors = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect::<BTreeSet<_>>();
        assert_eq!(survivors.len(), BACKUP_KEEP_COUNT + 1);
        assert!(survivors.contains("29990101000000"));
        assert!(!survivors.contains("20260101000000"));
        assert!(!survivors.contains("20260101000001"));
    }
}
