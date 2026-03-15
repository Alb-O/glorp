use {
	std::{io::IsTerminal, sync::Once, time::Duration},
	tracing_subscriber::EnvFilter,
};

const DEFAULT_TRACE_FILTER: &str = "liney=trace";
static INIT_TRACING: Once = Once::new();

pub(crate) fn init_tracing() {
	INIT_TRACING.call_once(|| {
		let Some(filter) = tracing_filter() else {
			return;
		};

		let _ = tracing_subscriber::fmt()
			.with_env_filter(filter)
			.with_target(true)
			.with_thread_names(false)
			.with_ansi(std::io::stderr().is_terminal())
			.compact()
			.try_init();
	});
}

pub(crate) fn duration_ms(duration: Duration) -> f64 {
	duration.as_secs_f64() * 1000.0
}

fn tracing_filter() -> Option<EnvFilter> {
	let liney_trace = std::env::var("LINEY_TRACE").ok();
	let rust_log = std::env::var("RUST_LOG").ok();

	if liney_trace.is_none() && rust_log.is_none() {
		return None;
	}

	if let Some(value) = liney_trace.as_deref() {
		let directive = match value.trim() {
			"" | "1" | "true" | "TRUE" => DEFAULT_TRACE_FILTER,
			"0" | "false" | "FALSE" => return None,
			other => other,
		};

		if let Ok(filter) = EnvFilter::try_new(directive) {
			return Some(filter);
		}
	}

	EnvFilter::try_from_default_env()
		.ok()
		.or_else(|| EnvFilter::try_new(DEFAULT_TRACE_FILTER).ok())
}
