use std::io::{self, Write};
use std::sync::mpsc::{self, Sender};

use anyhow::{anyhow, Context, Result};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::EnvFilter;

use crate::AppWindow;

const MAX_LOG_BYTES: usize = 100_000;

#[derive(Clone)]
struct ChannelMakeWriter {
    sender: Sender<String>,
}

struct ChannelWriter {
    sender: Sender<String>,
    buffer: Vec<u8>,
}

impl<'a> MakeWriter<'a> for ChannelMakeWriter {
    type Writer = ChannelWriter;

    fn make_writer(&'a self) -> Self::Writer {
        ChannelWriter {
            sender: self.sender.clone(),
            buffer: Vec::new(),
        }
    }
}

impl Write for ChannelWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for ChannelWriter {
    fn drop(&mut self) {
        if self.buffer.is_empty() {
            return;
        }

        let message = String::from_utf8_lossy(&self.buffer).into_owned();
        let _ = self.sender.send(message);
    }
}

pub fn init(ui: slint::Weak<AppWindow>) -> Result<()> {
    let (sender, receiver) = mpsc::channel::<String>();
    std::thread::Builder::new()
        .name("cheshire-log-relay".to_string())
        .spawn(move || {
            while let Ok(message) = receiver.recv() {
                let _ = ui.upgrade_in_event_loop(move |ui| append(&ui, &message));
            }
        })
        .context("start UI log relay")?;

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,hudsucker=off"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_writer(ChannelMakeWriter { sender })
        .try_init()
        .map_err(|error| anyhow!("initialize application logging: {error}"))?;

    Ok(())
}

fn append(ui: &AppWindow, message: &str) {
    let mut log = ui.get_log_text().to_string();
    log.push_str(message);

    if log.len() > MAX_LOG_BYTES {
        let mut start = log.len() - MAX_LOG_BYTES;
        while !log.is_char_boundary(start) {
            start += 1;
        }
        log = log[start..].to_string();
    }

    ui.set_log_text(log.into());
}
