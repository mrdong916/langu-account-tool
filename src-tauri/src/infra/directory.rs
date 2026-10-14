use crate::{domain::workspace::Account, error::AppResult};
use rusqlite::{params, Connection};
use std::path::Path;

pub struct Cache {
    pub users: Vec<Account>,
    pub last_sync: Option<i64>,
    pub last_full_sync: Option<i64>,
}

fn open(path: &Path) -> AppResult<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let db = Connection::open(path)?;
    db.busy_timeout(std::time::Duration::from_secs(5))?;
    db.execute_batch("CREATE TABLE IF NOT EXISTS users (scope TEXT NOT NULL, id TEXT NOT NULL, account TEXT NOT NULL, PRIMARY KEY(scope,id));
        CREATE TABLE IF NOT EXISTS sync (scope TEXT PRIMARY KEY, last_sync INTEGER NOT NULL, last_full_sync INTEGER);")?;
    Ok(db)
}

pub fn load(path: &Path, scope: &str) -> AppResult<Cache> {
    let db = open(path)?;
    let mut statement = db.prepare("SELECT account FROM users WHERE scope=? ORDER BY id")?;
    let mut users = Vec::new();
    for row in statement.query_map([scope], |row| row.get::<_, String>(0))? {
        let mut account: Account = serde_json::from_str(&row?)?;
        if account.status != "1" {
            continue;
        }
        crate::domain::directory::index(&mut account);
        users.push(account);
    }
    use rusqlite::OptionalExtension;
    let times = db
        .query_row(
            "SELECT last_sync,last_full_sync FROM sync WHERE scope=?",
            [scope],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?)),
        )
        .optional()?;
    Ok(Cache {
        users,
        last_sync: times.map(|t| t.0),
        last_full_sync: times.and_then(|t| t.1),
    })
}

pub fn save_snapshot(
    path: &Path,
    scope: &str,
    users: &[Account],
    full: bool,
    until: i64,
) -> AppResult<()> {
    let mut db = open(path)?;
    let tx = db.transaction()?;
    tx.execute("DELETE FROM users WHERE scope=?", [scope])?;
    {
        let mut statement = tx.prepare("INSERT INTO users(scope,id,account) VALUES(?,?,?)")?;
        for user in users.iter().filter(|user| user.status == "1") {
            statement.execute(params![scope, user.id, serde_json::to_string(user)?])?;
        }
    }
    tx.execute("INSERT INTO sync(scope,last_sync,last_full_sync) VALUES(?,?,?) ON CONFLICT(scope) DO UPDATE SET last_sync=excluded.last_sync, last_full_sync=COALESCE(excluded.last_full_sync,sync.last_full_sync)", params![scope, until, full.then_some(until)])?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_replacement_preserves_full_sync_time_and_owner_isolation() {
        let path =
            std::env::temp_dir().join(format!("langu-directory-{}.sqlite", uuid::Uuid::new_v4()));
        let a = Account {
            id: "1".into(),
            username: "a".into(),
            status: "1".into(),
            ..Default::default()
        };
        let b = Account {
            id: "2".into(),
            username: "b".into(),
            status: "1".into(),
            ..Default::default()
        };
        save_snapshot(&path, "owner-a", &[a.clone(), b.clone()], true, 10).unwrap();
        save_snapshot(&path, "owner-b", &[a.clone()], true, 11).unwrap();
        let mut updated = a.clone();
        updated.department = "财务部".into();
        save_snapshot(&path, "owner-a", &[updated, b.clone()], false, 12).unwrap();
        let cache = load(&path, "owner-a").unwrap();
        assert_eq!(cache.users.len(), 2);
        assert_eq!(cache.users[0].department, "财务部");
        assert_eq!(cache.last_full_sync, Some(10));
        save_snapshot(&path, "owner-a", &[b], true, 13).unwrap();
        assert_eq!(load(&path, "owner-a").unwrap().users.len(), 1);
        assert_eq!(load(&path, "owner-b").unwrap().users[0].username, "a");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn inactive_accounts_are_neither_saved_nor_restored_from_legacy_cache() {
        let path =
            std::env::temp_dir().join(format!("langu-status-{}.sqlite", uuid::Uuid::new_v4()));
        let users: Vec<_> = ["1", "2", "3", ""]
            .into_iter()
            .enumerate()
            .map(|(id, status)| Account {
                id: id.to_string(),
                username: format!("user-{id}"),
                status: status.into(),
                ..Default::default()
            })
            .collect();
        save_snapshot(&path, "owner", &users, true, 10).unwrap();
        let db = open(&path).unwrap();
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        db.execute(
            "INSERT INTO users VALUES(?,?,?)",
            params!["owner", "legacy", serde_json::to_string(&users[1]).unwrap()],
        )
        .unwrap();
        assert_eq!(load(&path, "owner").unwrap().users.len(), 1);
        save_snapshot(&path, "owner", &[], false, 11).unwrap();
        let cache = load(&path, "owner").unwrap();
        assert!(cache.users.is_empty());
        assert_eq!(cache.last_full_sync, Some(10));
        drop(db);
        std::fs::remove_file(path).unwrap();
    }
}
