use std::fs::OpenOptions;

use slog::{Drain, Duplicate, Logger, o};

/// Default log directives when `RUST_LOG` is unset.
///
/// The Gluon VM logs every call, compile and GC pass at INFO, which drowns the
/// application output. Keep INFO for everything else and raise Gluon to WARN.
pub const DEFAULT_LOG_FILTER: &str = "info,gluon_vm=warn,gluon=warn";

/// Log directives from `RUST_LOG`, falling back to [`DEFAULT_LOG_FILTER`].
pub fn log_filter_directives() -> String {
    std::env::var("RUST_LOG").unwrap_or_else(|_| DEFAULT_LOG_FILTER.to_string())
}

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
        .parse(&log_filter_directives())
        .build()
        .fuse();

    let dual_drain = Duplicate::new(terminal_drain, file_drain).fuse();

    let async_drain = slog_async::Async::new(dual_drain).build().fuse();

    Logger::root(async_drain, o!("version" => env!("CARGO_PKG_VERSION")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::Level;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{EnvFilter, Registry};

    fn enabled_with_default(target: &str, level: Level) -> bool {
        let subscriber = Registry::default().with(EnvFilter::new(DEFAULT_LOG_FILTER));
        tracing::subscriber::with_default(subscriber, || {
            let meta = tracing::Metadata::new(
                "probe",
                target,
                level,
                None,
                None,
                None,
                tracing::field::FieldSet::new(&[], tracing::callsite::Identifier(&PROBE)),
                tracing::metadata::Kind::EVENT,
            );
            tracing::dispatcher::get_default(|d| d.enabled(&meta))
        })
    }

    struct Probe;
    impl tracing::Callsite for Probe {
        fn set_interest(&self, _: tracing::subscriber::Interest) {}
        fn metadata(&self) -> &tracing::Metadata<'_> {
            unreachable!()
        }
    }
    static PROBE: Probe = Probe;

    #[test]
    fn default_filter_keeps_app_info() {
        assert!(enabled_with_default("diagramide::editor", Level::INFO));
    }

    #[test]
    fn default_filter_drops_gluon_vm_info() {
        assert!(!enabled_with_default("gluon_vm::gc", Level::INFO));
        assert!(!enabled_with_default("gluon_vm::thread", Level::INFO));
    }

    #[test]
    fn default_filter_keeps_gluon_vm_warnings() {
        assert!(enabled_with_default("gluon_vm::gc", Level::WARN));
    }
}
