use std::sync::{Arc, Mutex};

use crate::Config;


pub struct AppState {
    pub config: Arc<Mutex<Option<Config>>>,
}
