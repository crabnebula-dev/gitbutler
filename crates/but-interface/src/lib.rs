use std::sync::Arc;

use but_settings::AppSettingsWithDiskSync;
use tokio::sync::Mutex;

use crate::broadcaster::Broadcaster;

pub mod broadcaster;
pub mod commands;
pub mod error;

#[derive(Clone)]
pub struct IpcContext {
    pub app_settings: Arc<AppSettingsWithDiskSync>,
    pub user_controller: Arc<gitbutler_user::Controller>,
    pub broadcaster: Arc<Mutex<Broadcaster>>,
}
