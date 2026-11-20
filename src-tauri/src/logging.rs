use std::sync::Mutex;

use tauri::{AppHandle, Manager};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Keep a week of daily files. `max_log_files` counts files, not calendar days:
/// a day the app never opens has no file, and pruning happens on start or roll.
const LOG_RETENTION_FILES: usize = 7;

/// Keeps the non-blocking writer's worker alive for the app's lifetime.
pub struct LogGuard {
    _guard: Mutex<WorkerGuard>,
}

impl LogGuard {
    pub fn new(guard: WorkerGuard) -> Self {
        Self {
            _guard: Mutex::new(guard),
        }
    }
}

/// Log to stderr and to one file per UTC day in the platform log directory.
/// Older daily files are pruned by the appender itself.
pub fn init(app: &AppHandle) -> Result<WorkerGuard, Box<dyn std::error::Error>> {
    let log_dir = app.path().app_log_dir()?;
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("peekr.log")
        .max_log_files(LOG_RETENTION_FILES)
        .build(&log_dir)?;
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    // Our own crate logs at debug in development and info in release. Clamp the
    // HTTP/TLS stack and the keychain so a developer's `RUST_LOG=trace` does not
    // turn request plumbing into the log.
    let default_filter = if cfg!(debug_assertions) {
        "info,peekr_lib=debug"
    } else {
        "info,peekr_lib=info"
    };
    let base = std::env::var("RUST_LOG").unwrap_or_else(|_| default_filter.to_string());
    let directives = format!("{base},tauri=info,reqwest=info,hyper=info,rustls=info,keyring=info");
    let filter = EnvFilter::try_new(directives).unwrap_or_else(|_| EnvFilter::new("info"));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false)
        .with_target(true);

    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(true);

    if let Err(error) = tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .with(file_layer)
        .try_init()
    {
        // A subscriber is already installed (tests or a second init); keep going.
        eprintln!("logging already initialized: {error}");
    }

    install_panic_hook();
    tracing::info!(log_dir = %log_dir.display(), "logging initialized");

    Ok(guard)
}

/// Report panics through tracing before the default hook prints them. Without
/// this, a panic in a GUI release build only reaches stderr, which is discarded.
fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(panic = %info, "unhandled panic");
        previous(info);
    }));
}
