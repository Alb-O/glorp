use {
	crate::{
		perf::{PerfDashboard, PerfGraphSeries},
		types::Message,
		ui::{panel_scrollable, panel_style},
	},
	iced::{
		Color, Element, Font, Length, Pixels, Point, Rectangle, Size, Theme,
		widget::{canvas, column, container, row, text},
	},
};

pub(crate) fn view_perf_tab(dashboard: &PerfDashboard) -> Element<'static, Message> {
	panel_scrollable(
		container(
			column![
				text("Overview").size(18),
				view_perf_panel(dashboard.overview.text()),
				text("Live graphs").size(18),
				view_graph_grid(&dashboard.graphs),
				text("Frame pacing").size(18),
				view_perf_panel(dashboard.frame_pacing.text()),
				text("Hot paths").size(18),
				view_perf_panel(join_lines(
					dashboard.hot_paths.iter().map(crate::perf::PerfMetricSummary::text)
				)),
				text("Recent activity").size(18),
				view_perf_panel(join_lines(
					dashboard
						.recent_activity
						.iter()
						.map(crate::perf::PerfRecentActivity::text)
				)),
			]
			.spacing(12),
		)
		.width(Length::Fill),
	)
	.width(Length::Fill)
	.into()
}

fn view_perf_panel(content: String) -> Element<'static, Message> {
	container(text(content).font(Font::MONOSPACE).size(14).width(Length::Fill))
		.padding(12)
		.width(Length::Fill)
		.style(panel_style)
		.into()
}

fn view_graph_grid(graphs: &[PerfGraphSeries]) -> Element<'static, Message> {
	column(graphs.iter().cloned().map(view_graph_panel))
		.spacing(12)
		.width(Length::Fill)
		.into()
}

fn view_graph_panel(graph: PerfGraphSeries) -> Element<'static, Message> {
	container(
		column![
			row![
				text(graph.title).size(16),
				text(format!(
					"last {:>9}   avg {:>9}   p95 {:>9}   ceil {:>9}",
					format_duration_label(graph.latest_ms),
					format_duration_label(graph.avg_ms),
					format_duration_label(graph.p95_ms),
					format_duration_label(graph.ceiling_ms),
				))
				.font(Font::MONOSPACE)
				.size(13)
				.width(Length::Fill)
			]
			.spacing(12)
			.width(Length::Fill),
			canvas(TimeSeriesGraph { graph }).width(Length::Fill).height(96)
		]
		.spacing(10),
	)
	.padding(12)
	.width(Length::Fill)
	.style(panel_style)
	.into()
}

#[derive(Debug, Clone)]
struct TimeSeriesGraph {
	graph: PerfGraphSeries,
}

impl canvas::Program<Message> for TimeSeriesGraph {
	type State = ();

	fn draw(
		&self, _state: &Self::State, renderer: &iced::Renderer, theme: &Theme, bounds: Rectangle,
		_cursor: iced::mouse::Cursor,
	) -> Vec<canvas::Geometry> {
		let palette = theme.palette();
		let mut frame = canvas::Frame::new(renderer, bounds.size());
		let size = bounds.size();
		let chart_bounds = chart_bounds(size);

		frame.fill_rectangle(Point::ORIGIN, size, palette.background.base.color);
		draw_grid(&mut frame, chart_bounds, palette);
		draw_thresholds(&mut frame, chart_bounds, &self.graph);
		draw_series(&mut frame, chart_bounds, &self.graph, palette);
		draw_axis_labels(&mut frame, chart_bounds, &self.graph, palette);

		vec![frame.into_geometry()]
	}
}

fn chart_bounds(size: Size) -> Rectangle {
	Rectangle {
		x: 0.0,
		y: 10.0,
		width: size.width,
		height: (size.height - 22.0).max(24.0),
	}
}

fn draw_grid(frame: &mut canvas::Frame, bounds: Rectangle, palette: &iced::theme::Palette) {
	for (fraction, alpha) in [(0.0, 0.18), (0.5, 0.18), (1.0, 0.5)] {
		let y = bounds.y + bounds.height * fraction;
		let path = canvas::Path::line(Point::new(bounds.x, y), Point::new(bounds.x + bounds.width, y));
		frame.stroke(
			&path,
			canvas::Stroke::default().with_width(1.0).with_color(Color::from_rgba(
				palette.background.strong.color.r,
				palette.background.strong.color.g,
				palette.background.strong.color.b,
				alpha,
			)),
		);
	}
}

fn draw_thresholds(frame: &mut canvas::Frame, bounds: Rectangle, graph: &PerfGraphSeries) {
	for (threshold, color) in [
		(graph.warning_ms, Color::from_rgba(1.0, 0.8, 0.2, 0.45)),
		(graph.severe_ms, Color::from_rgba(1.0, 0.35, 0.35, 0.55)),
	] {
		let Some(threshold) = threshold else {
			continue;
		};

		let y = sample_y(bounds, threshold, graph.ceiling_ms);
		let path = canvas::Path::line(Point::new(bounds.x, y), Point::new(bounds.x + bounds.width, y));
		frame.stroke(&path, canvas::Stroke::default().with_width(1.0).with_color(color));
	}

	if let Some(warning_ms) = graph.warning_ms {
		let warning_y = sample_y(bounds, warning_ms, graph.ceiling_ms);
		let danger_zone = canvas::Path::rectangle(
			Point::new(bounds.x, warning_y),
			Size::new(bounds.width, bounds.height - (warning_y - bounds.y)),
		);
		frame.fill(&danger_zone, Color::from_rgba(1.0, 0.2, 0.2, 0.06));
	}
}

fn draw_series(frame: &mut canvas::Frame, bounds: Rectangle, graph: &PerfGraphSeries, palette: &iced::theme::Palette) {
	if graph.samples_ms.is_empty() {
		frame.fill_text(canvas::Text {
			content: "waiting for samples".to_string(),
			position: Point::new(bounds.x + 10.0, bounds.y + bounds.height * 0.55),
			color: palette.background.base.text,
			size: Pixels(14.0),
			font: Font::MONOSPACE,
			..canvas::Text::default()
		});
		return;
	}

	let points = graph_points(bounds, graph);
	let area = canvas::Path::new(|builder| {
		let first = points[0];
		builder.move_to(Point::new(first.x, bounds.y + bounds.height));

		for point in &points {
			builder.line_to(*point);
		}

		let last = *points.last().unwrap_or(&first);
		builder.line_to(Point::new(last.x, bounds.y + bounds.height));
		builder.close();
	});

	frame.fill(&area, Color::from_rgba(0.25, 0.75, 1.0, 0.12));

	let line = canvas::Path::new(|builder| {
		builder.move_to(points[0]);

		for point in points.iter().skip(1) {
			builder.line_to(*point);
		}
	});

	frame.stroke(
		&line,
		canvas::Stroke::default()
			.with_width(2.0)
			.with_color(Color::from_rgb(0.35, 0.82, 1.0)),
	);

	for (sample, point) in graph.samples_ms.iter().zip(points.iter()) {
		let color = spike_color(*sample, graph.warning_ms, graph.severe_ms);
		let marker = canvas::Path::circle(
			*point,
			if graph.severe_ms.is_some_and(|threshold| *sample >= threshold)
				|| graph.warning_ms.is_some_and(|threshold| *sample >= threshold)
			{
				2.6
			} else {
				1.8
			},
		);
		frame.fill(&marker, color);
	}
}

fn draw_axis_labels(
	frame: &mut canvas::Frame, bounds: Rectangle, graph: &PerfGraphSeries, palette: &iced::theme::Palette,
) {
	frame.fill_text(canvas::Text {
		content: format_duration_label(graph.ceiling_ms),
		position: Point::new(bounds.x + 6.0, bounds.y + 2.0),
		color: faded_text(palette.background.base.text, 0.78),
		size: Pixels(11.0),
		font: Font::MONOSPACE,
		..canvas::Text::default()
	});

	frame.fill_text(canvas::Text {
		content: "0".into(),
		position: Point::new(bounds.x + 6.0, bounds.y + bounds.height - 2.0),
		color: faded_text(palette.background.base.text, 0.78),
		size: Pixels(11.0),
		font: Font::MONOSPACE,
		..canvas::Text::default()
	});
}

fn graph_points(bounds: Rectangle, graph: &PerfGraphSeries) -> Vec<Point> {
	if graph.samples_ms.len() == 1 {
		return vec![Point::new(
			bounds.x + bounds.width - 8.0,
			sample_y(bounds, graph.samples_ms[0], graph.ceiling_ms),
		)];
	}

	let steps = (graph.samples_ms.len() - 1) as f32;
	let step = bounds.width / steps.max(1.0);
	let mut x = bounds.x;

	graph
		.samples_ms
		.iter()
		.map(|sample| {
			let point = Point::new(x, sample_y(bounds, *sample, graph.ceiling_ms));
			x += step;
			point
		})
		.collect()
}

fn sample_y(bounds: Rectangle, sample_ms: f32, ceiling_ms: f32) -> f32 {
	let normalized = 1.0 - (sample_ms / ceiling_ms.max(0.001)).clamp(0.0, 1.0);
	bounds.y + bounds.height * normalized
}

fn spike_color(sample_ms: f32, warning_ms: Option<f32>, severe_ms: Option<f32>) -> Color {
	if severe_ms.is_some_and(|threshold| sample_ms >= threshold) {
		Color::from_rgb(1.0, 0.42, 0.42)
	} else if warning_ms.is_some_and(|threshold| sample_ms >= threshold) {
		Color::from_rgb(1.0, 0.86, 0.35)
	} else {
		Color::from_rgb(0.35, 0.82, 1.0)
	}
}

fn faded_text(color: Color, alpha: f32) -> Color {
	Color {
		a: color.a * alpha,
		..color
	}
}

fn format_duration_label(ms: f32) -> String {
	if ms >= 100.0 {
		format!("{ms:>6.0} ms")
	} else if ms >= 10.0 {
		format!("{ms:>6.1} ms")
	} else {
		format!("{ms:>6.2} ms")
	}
}

fn join_lines(lines: impl IntoIterator<Item = String>) -> String {
	let mut text = String::new();

	for line in lines {
		if !text.is_empty() {
			text.push('\n');
		}
		text.push_str(&line);
	}

	text
}
