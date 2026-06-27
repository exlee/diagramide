use std::fs::OpenOptions;

use slog::{Drain, Duplicate, Logger, o};
pub fn init_logger() -> Logger {
    let log_path = std::env::temp_dir().join("diagramide.jsonlog");
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .unwrap();
    let file_drain = slog_json::Json::new(file).build().fuse();

    let decorator = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let console_drain = slog_term::FullFormat::new(decorator).build().fuse();
    let terminal_drain = slog_envlogger::LogBuilder::new(console_drain)
        .parse(&std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .build()
        .fuse();

    let dual_drain = Duplicate::new(terminal_drain, file_drain).fuse();

    let async_drain = slog_async::Async::new(dual_drain).build().fuse();

    Logger::root(async_drain, o!("version" => env!("CARGO_PKG_VERSION")))
}
