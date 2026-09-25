//! SQLite storage for accounts and opaque world-state snapshots.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension, Result};

pub struct Store {
    connection: Mutex<Connection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub account_id: String,
    pub display_name: String,
    pub ticket: String,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let connection = Connection::open(path)?;
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS accounts (
                 account_id TEXT PRIMARY KEY NOT NULL,
                 display_name TEXT NOT NULL,
                 ticket TEXT NOT NULL,
                 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
             );
             CREATE TABLE IF NOT EXISTS world_state (
                 account_id TEXT NOT NULL,
                 world_id TEXT NOT NULL,
                 state BLOB NOT NULL,
                 updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 PRIMARY KEY (account_id, world_id),
                 FOREIGN KEY (account_id) REFERENCES accounts(account_id) ON DELETE CASCADE
             );",
        )?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn upsert_account(&self, account: &Account) -> Result<()> {
        self.connection.lock().unwrap().execute(
            "INSERT INTO accounts (account_id, display_name, ticket)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(account_id) DO UPDATE SET
                 display_name = excluded.display_name,
                 ticket = excluded.ticket,
                 updated_at = CURRENT_TIMESTAMP",
            params![account.account_id, account.display_name, account.ticket],
        )?;
        Ok(())
    }

    pub fn upsert_world_state(&self, account_id: &str, world_id: &str, state: &[u8]) -> Result<()> {
        self.connection.lock().unwrap().execute(
            "INSERT INTO world_state (account_id, world_id, state)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(account_id, world_id) DO UPDATE SET
                 state = excluded.state,
                 updated_at = CURRENT_TIMESTAMP",
            params![account_id, world_id, state],
        )?;
        Ok(())
    }

    pub fn world_state(&self, account_id: &str, world_id: &str) -> Result<Option<Vec<u8>>> {
        self.connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT state FROM world_state WHERE account_id = ?1 AND world_id = ?2",
                params![account_id, world_id],
                |row| row.get(0),
            )
            .optional()
    }
}
