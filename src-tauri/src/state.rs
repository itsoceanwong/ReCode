use std::sync::{Arc, Mutex};

use crate::db::Db;
use crate::providers::cursor::CursorProvider;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub otlp_port: Arc<Mutex<u16>>,
    pub cursor: Arc<Mutex<CursorProvider>>,
}

impl AppState {
    pub fn new(db: Db, otlp_port: u16, cursor_enabled: bool) -> Self {
        Self {
            db: Arc::new(db),
            otlp_port: Arc::new(Mutex::new(otlp_port)),
            cursor: Arc::new(Mutex::new(CursorProvider::new(cursor_enabled))),
        }
    }
}
