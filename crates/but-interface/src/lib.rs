use std::sync::Arc;

use but_settings::AppSettingsWithDiskSync;
use tokio::sync::Mutex;

use crate::broadcaster::Broadcaster;

pub mod broadcaster;
pub mod commands;
pub mod error;

#[derive(Clone)]
pub struct IpcContext {
    app_settings: Arc<AppSettingsWithDiskSync>,
    user_controller: Arc<gitbutler_user::Controller>,
    project_controller: Arc<gitbutler_project::Controller>,
    broadcaster: Arc<Mutex<Broadcaster>>,
}
