use {
	std::{io::IsTerminal, sync::Once, time::Duration},
	tracing_subscriber::EnvFilter,
};

const DEFAULT_TRACE_FILTER: &str = "glorp=trace";
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
	let glorp_trace = std::env::var("GLORP_TRACE").ok();
	let rust_log = std::env::var("RUST_LOG").ok();
	let directive = match glorp_trace.as_deref().map(str::trim) {
		None if rust_log.is_none() => return None,
		Some(value) if value == "0" || value.eq_ignore_ascii_case("false") => return None,
		Some(value) if value.is_empty() || value == "1" || value.eq_ignore_ascii_case("true") => {
			Some(DEFAULT_TRACE_FILTER)
		}
		Some(value) => Some(value),
		None => None,
	};

	directive
		.and_then(|directive| EnvFilter::try_new(directive).ok())
		.or_else(|| EnvFilter::try_from_default_env().ok())
		.or_else(|| EnvFilter::try_new(DEFAULT_TRACE_FILTER).ok())
}
