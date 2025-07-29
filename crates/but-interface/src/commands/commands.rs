use but_settings::AppSettingsWithDiskSync;
use gitbutler_command_context::CommandContext;
use gitbutler_project::ProjectId;
use gitbutler_reference::RemoteRefname;
use gitbutler_repo::RepositoryExt as _;
use serde::Deserialize;
use tracing::instrument;

use crate::{IpcContext, error::Error};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRemoteBranchesParams {
    project_id: ProjectId,
}

pub fn git_remote_branches(
    ipc_ctx: &IpcContext,
    params: GitRemoteBranchesParams,
) -> Result<Vec<RemoteRefname>, Error> {
    let project = ipc_ctx.project_controller.get(params.project_id)?;
    let ctx = CommandContext::open(&project, ipc_ctx.app_settings.get()?.clone())?;
    Ok(ctx.repo().remote_branches()?)
}

pub fn git_test_push(
    app: State<'_, App>,
    settings: State<'_, AppSettingsWithDiskSync>,
    project_id: ProjectId,
    remote_name: &str,
    branch_name: &str,
) -> Result<(), Error> {
    Ok(app.git_test_push(
        project_id,
        remote_name,
        branch_name,
        // Run askpass, but don't pass any action
        Some(None),
        settings.get()?.clone(),
    )?)
}

#[tauri::command(async)]
#[instrument(skip(app, settings), err(Debug))]
pub fn git_test_fetch(
    app: State<'_, App>,
    settings: State<'_, AppSettingsWithDiskSync>,
    project_id: ProjectId,
    remote_name: &str,
    action: Option<String>,
) -> Result<(), Error> {
    Ok(app.git_test_fetch(
        project_id,
        remote_name,
        Some(action.unwrap_or_else(|| "test".to_string())),
        settings.get()?.clone(),
    )?)
}

#[tauri::command(async)]
#[instrument(skip(app, settings), err(Debug))]
pub fn git_index_size(
    app: State<'_, App>,
    settings: State<'_, AppSettingsWithDiskSync>,
    project_id: ProjectId,
) -> Result<usize, Error> {
    Ok(app
        .git_index_size(project_id, settings.get()?.clone())
        .expect("git index size"))
}

#[tauri::command(async)]
#[instrument(skip(app, settings), err(Debug))]
pub fn git_head(
    app: State<'_, App>,
    settings: State<'_, AppSettingsWithDiskSync>,
    project_id: ProjectId,
) -> Result<String, Error> {
    Ok(app.git_head(project_id, settings.get()?.clone())?)
}

#[tauri::command(async)]
#[instrument(skip(app), err(Debug))]
pub fn delete_all_data(app: State<'_, App>) -> Result<(), Error> {
    app.delete_all_data()?;
    Ok(())
}

#[tauri::command(async)]
#[instrument(err(Debug))]
pub fn git_set_global_config(key: &str, value: &str) -> Result<String, Error> {
    Ok(App::git_set_global_config(key, value)?)
}

#[tauri::command(async)]
#[instrument(err(Debug))]
pub fn git_remove_global_config(key: &str) -> Result<(), Error> {
    Ok(App::git_remove_global_config(key)?)
}

#[tauri::command(async)]
#[instrument(err(Debug), level = "trace")]
pub fn git_get_global_config(key: &str) -> Result<Option<String>, Error> {
    Ok(App::git_get_global_config(key)?)
}
