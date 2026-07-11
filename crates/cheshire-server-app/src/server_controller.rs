use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;

use anyhow::{anyhow, bail, Context, Result};
use cheshire_server_core::Config;
use cheshire_server_runtime::Server;
use tokio::runtime::Builder;
use tokio::sync::oneshot;

use crate::AppWindow;

#[derive(Clone)]
pub struct ServerController {
    state: Arc<Mutex<State>>,
    assets_dir: Arc<PathBuf>,
}

#[derive(Default)]
struct State {
    running: bool,
    shutdown: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl ServerController {
    pub fn new(assets_dir: PathBuf) -> Self {
        Self {
            state: Arc::default(),
            assets_dir: Arc::new(assets_dir),
        }
    }

    pub fn start(&self, config: Config, ui: slint::Weak<AppWindow>) -> Result<()> {
        let previous_thread = {
            let mut state = self.lock();
            if state.running {
                bail!("the server is already running");
            }
            state.thread.take()
        };
        if let Some(thread) = previous_thread {
            join_runtime_thread(thread).context("reap previous server runtime thread")?;
        }

        let (shutdown, shutdown_requested) = oneshot::channel();
        {
            let mut state = self.lock();
            if state.running {
                bail!("the server is already running");
            }
            state.running = true;
            state.shutdown = Some(shutdown);
        }

        let controller = self.clone();
        let assets_dir = self.assets_dir.clone();
        let thread = std::thread::Builder::new()
            .name("cheshire-server-runtime".to_string())
            .spawn(move || {
                tracing::info!("starting server from desktop application");
                let result = Builder::new_multi_thread()
                    .enable_all()
                    .thread_name("cheshire-server-worker")
                    .build()
                    .context("create Tokio runtime")
                    .and_then(|runtime| {
                        runtime.block_on(
                            Server::new(config)
                                .with_assets_dir(assets_dir.as_ref().clone())
                                .run_until_shutdown(async move {
                                    let _ = shutdown_requested.await;
                                }),
                        )
                    });

                {
                    let mut state = controller.lock();
                    state.running = false;
                    state.shutdown.take();
                }

                let (status, failed) = match result {
                    Ok(()) => {
                        tracing::info!("server stopped");
                        (
                            "Server stopped. Configuration remains loaded.".to_string(),
                            false,
                        )
                    }
                    Err(error) => {
                        tracing::error!(error = %error, "server stopped with an error");
                        (format!("Server failed: {error:#}"), true)
                    }
                };

                let _ = ui.upgrade_in_event_loop(move |ui| {
                    ui.set_server_running(false);
                    ui.set_status_text(status.into());
                    if failed {
                        ui.set_show_logs(true);
                    }
                });
            });

        match thread {
            Ok(thread) => {
                self.lock().thread = Some(thread);
                Ok(())
            }
            Err(error) => {
                let mut state = self.lock();
                state.running = false;
                state.shutdown.take();
                Err(error).context("start server runtime thread")
            }
        }
    }

    pub fn stop(&self) -> bool {
        let shutdown = {
            let mut state = self.lock();
            if !state.running {
                return false;
            }
            state.shutdown.take()
        };

        shutdown.is_some_and(|shutdown| shutdown.send(()).is_ok())
    }

    pub fn shutdown_and_wait(&self) -> Result<()> {
        self.stop();
        let thread = self.lock().thread.take();
        let result = thread.map_or(Ok(()), join_runtime_thread);

        let mut state = self.lock();
        state.running = false;
        state.shutdown.take();
        result.context("join server runtime thread")
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn join_runtime_thread(thread: JoinHandle<()>) -> Result<()> {
    thread
        .join()
        .map_err(|_| anyhow!("server runtime thread panicked"))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;

    #[test]
    fn shutdown_waits_for_tracked_runtime_thread() {
        let controller = ServerController::new("assets".into());
        let stopped = Arc::new(AtomicBool::new(false));
        let thread_stopped = stopped.clone();
        let (shutdown, shutdown_requested) = oneshot::channel();
        let thread = std::thread::spawn(move || {
            let _ = shutdown_requested.blocking_recv();
            thread_stopped.store(true, Ordering::SeqCst);
        });

        {
            let mut state = controller.lock();
            state.running = true;
            state.shutdown = Some(shutdown);
            state.thread = Some(thread);
        }

        controller.shutdown_and_wait().unwrap();

        assert!(stopped.load(Ordering::SeqCst));
        let state = controller.lock();
        assert!(!state.running);
        assert!(state.shutdown.is_none());
        assert!(state.thread.is_none());
    }
}
