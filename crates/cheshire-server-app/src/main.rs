mod config_editor;
mod log_sink;
mod server_controller;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cheshire_server_core::Config;
use slint::ComponentHandle;

use server_controller::ServerController;

slint::include_modules!();

const CONFIG_PATH: &str = "config.toml";

fn main() -> Result<()> {
    let ui = AppWindow::new().context("create application window")?;
    log_sink::init(ui.as_weak())?;

    load_initial_config(&ui);

    let assets_dir = resolve_assets_dir();
    tracing::info!(assets = %assets_dir.display(), "resolved application assets");
    let controller = ServerController::new(assets_dir);
    register_save_callback(&ui);
    register_start_callback(&ui, controller.clone());
    register_stop_callback(&ui, controller.clone());

    tracing::info!(
        config = CONFIG_PATH,
        "desktop application ready; server is stopped"
    );
    let event_loop_result = ui.run().context("run application event loop");
    let shutdown_result = controller
        .shutdown_and_wait()
        .context("shut down desktop server runtime");
    event_loop_result?;
    shutdown_result
}

fn load_initial_config(ui: &AppWindow) {
    match Config::load_or_create(CONFIG_PATH) {
        Ok(config) => {
            if let Err(error) = config_editor::populate(ui, &config) {
                report_error(ui, "Could not display configuration", &error);
            } else {
                ui.set_status_text(
                    "Ready — config.toml loaded. The server waits for Start, as civilized software should."
                        .into(),
                );
            }
        }
        Err(error) => {
            let fallback = Config::default();
            if let Err(populate_error) = config_editor::populate(ui, &fallback) {
                tracing::error!(error = %populate_error, "failed to display fallback configuration");
            }
            report_error(ui, "Could not load config.toml; showing defaults", &error);
        }
    }
}

fn register_save_callback(ui: &AppWindow) {
    let ui_weak = ui.as_weak();
    ui.on_save_config(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };

        match collect_and_save(&ui) {
            Ok(_) => {
                ui.set_status_text(
                    "Configuration saved to config.toml. No sockets were harmed.".into(),
                );
                tracing::info!(config = CONFIG_PATH, "configuration saved");
            }
            Err(error) => report_error(&ui, "Configuration was not saved", &error),
        }
    });
}

fn register_start_callback(ui: &AppWindow, controller: ServerController) {
    let ui_weak = ui.as_weak();
    ui.on_start_server(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };

        let config = match collect_and_save(&ui) {
            Ok(config) => config,
            Err(error) => {
                report_error(
                    &ui,
                    "Server did not start because configuration is invalid",
                    &error,
                );
                return;
            }
        };

        match controller.start(config, ui.as_weak()) {
            Ok(()) => {
                ui.set_server_running(true);
                ui.set_status_text("Starting server… configuration was saved first.".into());
            }
            Err(error) => report_error(&ui, "Server did not start", &error),
        }
    });
}

fn register_stop_callback(ui: &AppWindow, controller: ServerController) {
    let ui_weak = ui.as_weak();
    ui.on_stop_server(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };

        if controller.stop() {
            ui.set_status_text("Stopping server… persuading every task to leave politely.".into());
            tracing::info!("server shutdown requested from desktop application");
        } else {
            ui.set_status_text("The server is already stopping or stopped.".into());
        }
    });
}

fn collect_and_save(ui: &AppWindow) -> Result<Config> {
    let config = config_editor::collect(ui)?;
    config.save(Path::new(CONFIG_PATH))?;
    Ok(config)
}

fn report_error(ui: &AppWindow, summary: &str, error: &anyhow::Error) {
    let message = format!("{summary}: {error:#}");
    ui.set_status_text(message.clone().into());
    ui.set_show_logs(true);
    tracing::error!(error = %error, "{summary}");
}

fn resolve_assets_dir() -> PathBuf {
    let executable = std::env::current_exe().ok();

    #[cfg(target_os = "macos")]
    if let Some(bundle_assets) = executable
        .as_deref()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(|contents| contents.join("Resources/assets"))
        .filter(|path| path.is_dir())
    {
        return bundle_assets;
    }

    if let Some(adjacent_assets) = executable
        .as_deref()
        .and_then(Path::parent)
        .map(|directory| directory.join("assets"))
        .filter(|path| path.is_dir())
    {
        return adjacent_assets;
    }

    PathBuf::from("assets")
}
