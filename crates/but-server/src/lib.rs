use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{
        WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
    routing::{any, get},
};
use but_interface::{
    IpcContext,
    broadcaster::Broadcaster,
    commands::{git, repo, stack, users, virtual_branches},
    error::ToError as _,
};
use but_settings::AppSettingsWithDiskSync;
use futures_util::{SinkExt, StreamExt as _};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::json;
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

mod projects;
use crate::projects::ActiveProjects;

#[derive(Clone)]
pub(crate) struct RequestContext {
    app_settings: Arc<AppSettingsWithDiskSync>,
    user_controller: Arc<gitbutler_user::Controller>,
    active_projects: Arc<Mutex<ActiveProjects>>,
    broadcaster: Arc<Mutex<Broadcaster>>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "subject", rename_all = "camelCase")]
enum Response {
    Success(serde_json::Value),
    Error(serde_json::Value),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Request {
    command: String,
    params: serde_json::Value,
}

pub async fn run() {
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(Any)
        .allow_headers(Any);

    let config_dir = dirs::config_dir()
        .expect("missing config dir")
        .join("gitbutler");

    // TODO: This should probably be a real com.gitbutler.whatever directory
    let app_data_dir = dirs::config_dir()
        .expect("missing config dir")
        .join("gitbutler-server");

    let broadcaster = Arc::new(Mutex::new(Broadcaster::new()));

    let ctx = RequestContext {
        app_settings: Arc::new(
            AppSettingsWithDiskSync::new(config_dir.clone())
                .expect("failed to create app settings"),
        ),
        user_controller: Arc::new(gitbutler_user::Controller::from_path(&app_data_dir)),
        active_projects: Arc::new(Mutex::new(ActiveProjects::new())),
        broadcaster: broadcaster.clone(),
    };

    let ipc_ctx = IpcContext {
        app_settings: Arc::new(
            AppSettingsWithDiskSync::new(config_dir.clone())
                .expect("failed to create app settings"),
        ),
        user_controller: Arc::new(gitbutler_user::Controller::from_path(&app_data_dir)),
        broadcaster: broadcaster.clone(),
    };

    // build our application with a single route
    let app = Router::new()
        .route(
            "/",
            get(|| async { "Hello, World!" }).post({
                let ctx = ctx.clone();
                let ipc_ctx = ipc_ctx.clone();
                move |req| handle_command(req, ctx, ipc_ctx)
            }),
        )
        .route(
            "/ws",
            any({
                let broadcaster = broadcaster.clone();
                async move |req| handle_ws_request(req, broadcaster).await
            }),
        )
        .layer(ServiceBuilder::new().layer(cors));

    // run our app with hyper, listening globally on port 6978
    let listener = tokio::net::TcpListener::bind("0.0.0.0:6978").await.unwrap();
    println!("Running at 0.0.0.0:6978");
    axum::serve(listener, app).await.unwrap();
}

async fn handle_ws_request(
    ws: WebSocketUpgrade,
    broadcaster: Arc<Mutex<Broadcaster>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, broadcaster))
}

async fn handle_websocket(socket: WebSocket, broadcaster: Arc<Mutex<Broadcaster>>) {
    let (send, mut recv) = tokio::sync::mpsc::unbounded_channel();
    let id = uuid::Uuid::new_v4();
    broadcaster.lock().await.register_sender(&id, send);

    let (mut socket_send, mut socket_recv) = socket.split();
    let thread = tokio::spawn(async move {
        while let Some(event) = recv.recv().await {
            socket_send
                .send(Message::Text(serde_json::to_string(&event).unwrap().into()))
                .await
                .unwrap();
        }
    });

    while let Some(Ok(msg)) = socket_recv.next().await {
        #[allow(clippy::single_match)]
        match msg {
            Message::Close(_) => {
                thread.abort();
                break;
            }
            _ => {}
        }
    }

    broadcaster.lock().await.deregister_sender(&id);
}

fn run_cmd<
    D: DeserializeOwned,
    S: Serialize,
    Fun: Fn(&IpcContext, D) -> Result<S, but_interface::error::Error>,
>(
    ipc_ctx: &IpcContext,
    params: serde_json::Value,
    fun: Fun,
) -> Result<serde_json::Value, but_interface::error::Error> {
    let result = fun(ipc_ctx, serde_json::from_value(params).to_error()?)?;
    Ok(json!(result))
}

async fn handle_command(
    Json(request): Json<Request>,
    ctx: RequestContext,
    ipc_ctx: IpcContext,
) -> Json<serde_json::Value> {
    let command: &str = &request.command;
    let result = match command {
        // General commands
        "git_remote_branches" => run_cmd(&ipc_ctx, request.params, git::git_remote_branches),
        "git_test_push" => run_cmd(&ipc_ctx, request.params, git::git_test_push),
        "git_test_fetch" => run_cmd(&ipc_ctx, request.params, git::git_test_fetch),
        "git_index_size" => run_cmd(&ipc_ctx, request.params, git::git_index_size),
        "git_head" => run_cmd(&ipc_ctx, request.params, git::git_head),
        "delete_all_data" => run_cmd(&ipc_ctx, request.params, git::delete_all_data),
        "git_set_global_config" => run_cmd(&ipc_ctx, request.params, git::git_set_global_config),
        "git_remove_global_config" => {
            run_cmd(&ipc_ctx, request.params, git::git_remove_global_config)
        }
        "git_get_global_config" => run_cmd(&ipc_ctx, request.params, git::git_get_global_config),
        // // App settings
        // "get_app_settings" => settings::get_app_settings(&ctx),
        // "update_onboarding_complete" => settings::update_onboarding_complete(&ctx, request.params),
        // "update_telemetry" => settings::update_telemetry(&ctx, request.params),
        // "update_telemetry_distinct_id" => {
        //     settings::update_telemetry_distinct_id(&ctx, request.params)
        // }
        // "update_feature_flags" => settings::update_feature_flags(&ctx, request.params),
        // // Secret management
        // "secret_get_global" => secret::secret_get_global(&ctx, request.params),
        // "secret_set_global" => secret::secret_set_global(&ctx, request.params),
        // User management
        "get_user" => run_cmd(&ipc_ctx, request.params, users::get_user),
        "set_user" => run_cmd(&ipc_ctx, request.params, users::set_user),
        "delete_user" => run_cmd(&ipc_ctx, request.params, users::delete_user),
        // // Project management
        // "update_project" => projects::update_project(&ctx, request.params),
        // "add_project" => projects::add_project(&ctx, request.params),
        // "get_project" => projects::get_project(&ctx, request.params),
        // "list_projects" => projects::list_projects(&ctx).await,
        // "delete_project" => projects::delete_project(&ctx, request.params),
        // "set_project_active" => projects::set_project_active(&ctx, request.params).await,
        // Virtual branches commands
        "normalize_branch_name" => run_cmd(&ipc_ctx, request.params, virtual_branches::normalize_branch_name),
        "create_virtual_branch" => run_cmd(&ipc_ctx, request.params, virtual_branches::create_virtual_branch),
        "delete_local_branch" => run_cmd(&ipc_ctx, request.params, virtual_branches::delete_local_branch),
        "create_virtual_branch_from_branch" => {
            run_cmd(&ipc_ctx, request.params, virtual_branches::create_virtual_branch_from_branch)
        }
        "integrate_upstream_commits" => {
            run_cmd(&ipc_ctx, request.params, virtual_branches::integrate_upstream_commits)
        }
        "get_base_branch_data" => run_cmd(&ipc_ctx, request.params, virtual_branches::get_base_branch_data),
        "set_base_branch" => run_cmd(&ipc_ctx, request.params, virtual_branches::set_base_branch),
        "push_base_branch" => run_cmd(&ipc_ctx, request.params, virtual_branches::push_base_branch),
        "update_stack_order" => run_cmd(&ipc_ctx, request.params, virtual_branches::update_stack_order),
        "unapply_stack" => run_cmd(&ipc_ctx, request.params, virtual_branches::unapply_stack),
        "can_apply_remote_branch" => {
            run_cmd(&ipc_ctx, request.params, virtual_branches::can_apply_remote_branch)
        }
        "list_commit_files" => run_cmd(&ipc_ctx, request.params, virtual_branches::list_commit_files),
        "amend_virtual_branch" => run_cmd(&ipc_ctx, request.params, virtual_branches::amend_virtual_branch),
        "move_commit_file" => run_cmd(&ipc_ctx, request.params, virtual_branches::move_commit_file),
        "undo_commit" => run_cmd(&ipc_ctx, request.params, virtual_branches::undo_commit),
        "insert_blank_commit" => run_cmd(&ipc_ctx, request.params, virtual_branches::insert_blank_commit),
        "reorder_stack" => run_cmd(&ipc_ctx, request.params, virtual_branches::reorder_stack),
        "find_git_branches" => run_cmd(&ipc_ctx, request.params, virtual_branches::find_git_branches),
        "list_branches" => run_cmd(&ipc_ctx, request.params, virtual_branches::list_branches),
        "get_branch_listing_details" => {
            run_cmd(&ipc_ctx, request.params, virtual_branches::get_branch_listing_details)
        }
        "squash_commits" => run_cmd(&ipc_ctx, request.params, virtual_branches::squash_commits),
        "fetch_from_remotes" => run_cmd(&ipc_ctx, request.params, virtual_branches::fetch_from_remotes),
        "move_commit" => run_cmd(&ipc_ctx, request.params, virtual_branches::move_commit),
        "update_commit_message" => run_cmd(&ipc_ctx, request.params, virtual_branches::update_commit_message),
        "find_commit" => run_cmd(&ipc_ctx, request.params, virtual_branches::find_commit),
        "upstream_integration_statuses" => {
            run_cmd(&ipc_ctx, request.params, virtual_branches::upstream_integration_statuses)
        }
        "integrate_upstream" => run_cmd(&ipc_ctx, request.params, virtual_branches::integrate_upstream),
        "resolve_upstream_integration" => {
            run_cmd(&ipc_ctx, request.params, virtual_branches::resolve_upstream_integration)
        }
        // // Operating modes commands
        // "operating_mode" => modes::operating_mode(&ctx, request.params),
        // "enter_edit_mode" => modes::enter_edit_mode(&ctx, request.params),
        // "abort_edit_and_return_to_workspace" => {
        //     modes::abort_edit_and_return_to_workspace(&ctx, request.params)
        // }
        // "save_edit_and_return_to_workspace" => {
        //     modes::save_edit_and_return_to_workspace(&ctx, request.params)
        // }
        // "edit_initial_index_state" => modes::edit_initial_index_state(&ctx, request.params),
        // "edit_changes_from_initial" => modes::edit_changes_from_initial(&ctx, request.params),
        // // Stack commands
        // "create_branch" => stack::create_branch(&ctx, request.params),
        // "remove_branch" => stack::remove_branch(&ctx, request.params),
        // "update_branch_name" => stack::update_branch_name(&ctx, request.params),
        // "update_branch_description" => stack::update_branch_description(&ctx, request.params),
        // "update_branch_pr_number" => stack::update_branch_pr_number(&ctx, request.params),
        // "push_stack" => stack::push_stack(&ctx, request.params),
        // "push_stack_to_review" => stack::push_stack_to_review(&ctx, request.params),
        // // Workspace commands
        // "stacks" => workspace::stacks(&ctx, request.params),
        // #[cfg(unix)]
        // "show_graph_svg" => workspace::show_graph_svg(&ctx, request.params),
        // "stack_details" => workspace::stack_details(&ctx, request.params),
        // "branch_details" => workspace::branch_details(&ctx, request.params),
        // "create_commit_from_worktree_changes" => {
        //     workspace::create_commit_from_worktree_changes(&ctx, request.params)
        // }
        // "amend_commit_from_worktree_changes" => {
        //     workspace::amend_commit_from_worktree_changes(&ctx, request.params)
        // }
        // "discard_worktree_changes" => workspace::discard_worktree_changes(&ctx, request.params),
        // "move_changes_between_commits" => {
        //     workspace::move_changes_between_commits(&ctx, request.params)
        // }
        // "split_branch" => workspace::split_branch(&ctx, request.params),
        // "split_branch_into_dependent_branch" => {
        //     workspace::split_branch_into_dependent_branch(&ctx, request.params)
        // }
        // "uncommit_changes" => workspace::uncommit_changes(&ctx, request.params),
        // "stash_into_branch" => workspace::stash_into_branch(&ctx, request.params),
        // "canned_branch_name" => workspace::canned_branch_name(&ctx, request.params),
        // "target_commits" => workspace::target_commits(&ctx, request.params),
        // // Diff commands
        // "tree_change_diffs" => diff::tree_change_diffs(&ctx, request.params),
        // "commit_details" => diff::commit_details(&ctx, request.params),
        // "changes_in_branch" => diff::changes_in_branch(&ctx, request.params),
        // "changes_in_worktree" => diff::changes_in_worktree(&ctx, request.params),
        // "assign_hunk" => diff::assign_hunk(&ctx, request.params),
        // // Archive/Zip commands
        // "get_logs_archive_path" => zip::get_logs_archive_path(&ctx, request.params),
        // "get_project_archive_path" => zip::get_project_archive_path(&ctx, request.params),
        // Repository commands
        "git_get_local_config" => run_cmd(&ipc_ctx, request.params, repo::git_get_local_config),
        "git_set_local_config" => run_cmd(&ipc_ctx, request.params, repo::git_set_local_config),
        "check_signing_settings" => run_cmd(&ipc_ctx, request.params, repo::check_signing_settings),
        "git_clone_repository" => run_cmd(&ipc_ctx, request.params, repo::git_clone_repository),
        "get_uncommited_files" => run_cmd(&ipc_ctx, request.params, repo::get_uncommitted_files),
        "get_commit_file" => run_cmd(&ipc_ctx, request.params, repo::get_commit_file),
        "get_workspace_file" => run_cmd(&ipc_ctx, request.params, repo::get_workspace_file),
        "pre_commit_hook" => run_cmd(&ipc_ctx, request.params, repo::pre_commit_hook),
        "pre_commit_hook_diffspecs" => {
            run_cmd(&ipc_ctx, request.params, repo::pre_commit_hook_diffspecs)
        }
        "post_commit_hook" => run_cmd(&ipc_ctx, request.params, repo::post_commit_hook),
        "message_hook" => run_cmd(&ipc_ctx, request.params, repo::message_hook),
        // Stack management commands
        "create_branch" => run_cmd(&ipc_ctx, request.params, stack::create_branch),
        "remove_branch" => run_cmd(&ipc_ctx, request.params, stack::remove_branch),
        "update_branch_name" => run_cmd(&ipc_ctx, request.params, stack::update_branch_name),
        "update_branch_description" => run_cmd(&ipc_ctx, request.params, stack::update_branch_description),
        "update_branch_pr_number" => run_cmd(&ipc_ctx, request.params, stack::update_branch_pr_number),
        "push_stack" => run_cmd(&ipc_ctx, request.params, stack::push_stack),
        "push_stack_to_review" => run_cmd(&ipc_ctx, request.params, stack::push_stack_to_review),
        // // Undo/Snapshot commands
        // "list_snapshots" => undo::list_snapshots(&ctx, request.params),
        // "restore_snapshot" => undo::restore_snapshot(&ctx, request.params),
        // "snapshot_diff" => undo::snapshot_diff(&ctx, request.params),
        // "oplog_diff_worktrees" => undo::oplog_diff_worktrees(&ctx, request.params),
        // // Config management commands
        // "get_gb_config" => config::get_gb_config(&ctx, request.params),
        // "set_gb_config" => config::set_gb_config(&ctx, request.params),
        // // Remotes management commands
        // "list_remotes" => remotes::list_remotes(&ctx, request.params),
        // "add_remote" => remotes::add_remote(&ctx, request.params),
        // // Rules/Workspace rules commands
        // "create_workspace_rule" => rules::create_workspace_rule(&ctx, request.params),
        // "delete_workspace_rule" => rules::delete_workspace_rule(&ctx, request.params),
        // "update_workspace_rule" => rules::update_workspace_rule(&ctx, request.params),
        // "list_workspace_rules" => rules::list_workspace_rules(&ctx, request.params),
        // // Action/Workflow commands
        // "list_actions" => action::list_actions(&ctx, request.params),
        // "handle_changes" => action::handle_changes(&ctx, request.params),
        // "list_workflows" => action::list_workflows(&ctx, request.params),
        // // GitHub OAuth commands (async)
        // "init_device_oauth" => github::init_device_oauth(&ctx, request.params).await,
        // "check_auth_status" => github::check_auth_status(&ctx, request.params).await,
        // // Forge commands
        // "get_available_review_templates" => {
        //     forge::get_available_review_templates(&ctx, request.params)
        // }
        // "get_review_template_contents" => forge::get_review_template_contents(&ctx, request.params),
        // // Menu commands (limited - no menu_item_set_enabled as it's Tauri-specific)
        // "get_editor_link_scheme" => menu::get_editor_link_scheme(&ctx, request.params),
        // // CLI commands
        // "install_cli" => cli::install_cli(&ctx, request.params),
        // "cli_path" => cli::cli_path(&ctx, request.params),
        // // Askpass commands (async)
        // "submit_prompt_response" => askpass::submit_prompt_response(&ctx, request.params).await,
        // // Open/System commands (limited - no open_project_in_window as it's Tauri-specific)
        // "open_url" => open::open_url(&ctx, request.params),

        // TODO: Tauri-specific commands that cannot be ported to HTTP API:
        //
        // AI-Integrated Action Commands (require Tauri AppHandle for real-time UI updates):
        // - "auto_commit" => action::auto_commit() // Needs AppHandle for real-time AI progress updates
        // - "auto_branch_changes" => action::auto_branch_changes() // Needs AppHandle for real-time AI progress updates
        // - "absorb" => action::absorb() // Needs AppHandle for real-time AI progress updates
        // - "freestyle" => action::freestyle() // Needs AppHandle for real-time AI progress updates
        //
        // UI Management Commands (require Tauri window/menu system):
        // - "menu_item_set_enabled" => menu::menu_item_set_enabled() // Requires Tauri menu management
        // - "open_project_in_window" => projects::open_project_in_window() // Requires Tauri window creation
        _ => Err(anyhow::anyhow!("Command {} not found!", command).into()),
    };

    match result {
        Ok(value) => Json(json!(Response::Success(value))),
        Err(e) => Json(json!(Response::Error(json!(e)))),
    }
}
