use {
	super::{DocumentLayout, LayoutCluster, LayoutRun, SceneConfig, line_byte_offsets},
	cosmic_text::{Buffer, FontSystem, fontdb},
	std::{collections::BTreeSet, sync::Arc},
};

#[cfg(test)]
use super::build_buffer;

#[cfg(test)]
pub(crate) struct DocumentLayoutTestSpec {
	pub(crate) text: Arc<str>,
	pub(crate) wrapping: crate::types::WrapChoice,
	pub(crate) max_width: f32,
	pub(crate) measured_width: f32,
	pub(crate) measured_height: f32,
	pub(crate) glyph_count: usize,
	pub(crate) font_count: usize,
	pub(crate) runs: Vec<LayoutRun>,
	pub(crate) clusters: Vec<LayoutCluster>,
}

impl DocumentLayout {
	pub(crate) fn build(
		text: &str, buffer: &Buffer, config: SceneConfig, font_names: &[(fontdb::ID, Arc<str>)],
	) -> Self {
		let text = Arc::<str>::from(text);
		let line_byte_offsets = Arc::<[usize]>::from(line_byte_offsets(text.as_ref()));
		let mut runs = Vec::new();
		let mut clusters = Vec::new();
		let mut measured_width: f32 = 0.0;
		let mut measured_height: f32 = 0.0;
		let mut glyph_count = 0usize;

		for run in buffer.layout_runs() {
			measured_width = measured_width.max(run.line_w);
			measured_height = measured_height.max(run.line_top + run.line_height);
			let line_byte_offset = line_byte_offsets[run.line_i];
			let cluster_start = clusters.len();
			clusters.extend(build_clusters(
				runs.len(),
				line_byte_offset,
				run.line_top,
				run.line_height,
				run.glyphs,
				font_names,
			));
			let cluster_end = clusters.len();
			glyph_count += run.glyphs.len();

			runs.push(LayoutRun {
				baseline: run.line_y,
				cluster_range: cluster_start..cluster_end,
				glyph_count: run.glyphs.len(),
				line_height: run.line_height,
				line_index: run.line_i,
				line_top: run.line_top,
				line_width: run.line_w,
				rtl: run.rtl,
			});
		}

		let byte_order = build_byte_order(&clusters);

		Self {
			text,
			wrapping: config.wrapping,
			max_width: config.max_width,
			measured_width,
			measured_height,
			glyph_count,
			cluster_count: clusters.len(),
			font_count: font_names.len(),
			warnings: build_warnings(runs.is_empty()),
			runs: runs.into(),
			clusters: clusters.into(),
			line_byte_offsets,
			byte_order: byte_order.into(),
		}
	}
}

pub(crate) fn resolve_font_names_from_buffer(
	font_system: &FontSystem, buffer: &Buffer,
) -> Arc<[(fontdb::ID, Arc<str>)]> {
	buffer
		.layout_runs()
		.flat_map(|run| run.glyphs.iter().map(|glyph| glyph.font_id))
		.collect::<BTreeSet<_>>()
		.into_iter()
		.map(|font_id| {
			let name = font_system
				.db()
				.face(font_id)
				.map_or("unknown-font", |face| face.post_script_name.as_str());
			(font_id, Arc::<str>::from(name))
		})
		.collect::<Arc<[_]>>()
}

fn build_clusters(
	run_index: usize, line_byte_offset: usize, line_top: f32, line_height: f32, glyphs: &[cosmic_text::LayoutGlyph],
	font_names: &[(fontdb::ID, Arc<str>)],
) -> Vec<LayoutCluster> {
	let mut clusters = Vec::with_capacity(glyphs.len());
	let mut current: Option<LayoutCluster> = None;
	let mut cluster_font_id = None;

	for glyph in glyphs {
		let byte_range = (line_byte_offset + glyph.start)..(line_byte_offset + glyph.end);
		let glyph_y = line_top + glyph.y;
		let glyph_height = glyph.line_height_opt.unwrap_or(line_height);

		match current.as_mut() {
			Some(cluster) if cluster.byte_range == byte_range => {
				cluster.width = (glyph.x + glyph.w - cluster.x).max(cluster.width);
				cluster.height = cluster.height.max(glyph_height);
				cluster.glyph_count += 1;
				cluster.y = cluster.y.min(glyph_y);
				// A cluster can span multiple glyphs; once faces disagree, the
				// inspect surface intentionally degrades to a simple "mixed" label.
				if cluster_font_id.is_some_and(|font_id| font_id != glyph.font_id) {
					cluster_font_id = None;
					cluster.font_summary = Arc::<str>::from("mixed");
				}
			}
			_ => {
				if let Some(cluster) = current.replace(LayoutCluster {
					byte_range,
					glyph_count: 1,
					run_index,
					width: glyph.w.max(1.0),
					x: glyph.x,
					y: glyph_y,
					height: glyph_height.max(1.0),
					font_summary: lookup_font_name(font_names, glyph.font_id),
				}) {
					clusters.push(cluster);
				}
				cluster_font_id = Some(glyph.font_id);
			}
		}
	}

	if let Some(cluster) = current {
		clusters.push(cluster);
	}

	clusters
}

fn lookup_font_name(font_names: &[(fontdb::ID, Arc<str>)], font_id: fontdb::ID) -> Arc<str> {
	font_names
		.binary_search_by_key(&font_id, |(id, _)| *id)
		.ok()
		.and_then(|index| font_names.get(index))
		.map_or_else(|| Arc::<str>::from("unknown-font"), |(_, name)| Arc::clone(name))
}

fn build_byte_order(clusters: &[LayoutCluster]) -> Vec<usize> {
	let mut byte_order: Vec<_> = (0..clusters.len()).collect();
	// Navigation and cursor lookup are byte-based even when visual cluster
	// order differs, so keep a separate byte-sorted index beside run order.
	byte_order.sort_by(|a, b| {
		clusters[*a]
			.byte_range
			.start
			.cmp(&clusters[*b].byte_range.start)
			.then_with(|| clusters[*a].byte_range.end.cmp(&clusters[*b].byte_range.end))
			.then_with(|| clusters[*a].run_index.cmp(&clusters[*b].run_index))
	});
	byte_order
}

fn build_warnings(no_runs: bool) -> Arc<[String]> {
	if no_runs {
		Arc::from(["No layout runs were produced. Check the font choice and text content.".to_string()])
	} else {
		Arc::from([])
	}
}

#[cfg(test)]
impl DocumentLayout {
	pub(crate) fn build_for_test(font_system: &mut FontSystem, text: &str, config: SceneConfig) -> Self {
		let buffer = build_buffer(font_system, text, config);
		let font_names = resolve_font_names_from_buffer(font_system, &buffer);
		Self::build(text, &buffer, config, font_names.as_ref())
	}

	pub(crate) fn new_for_test(spec: DocumentLayoutTestSpec) -> Self {
		let DocumentLayoutTestSpec {
			text,
			wrapping,
			max_width,
			measured_width,
			measured_height,
			glyph_count,
			font_count,
			runs,
			clusters,
		} = spec;
		let byte_order = build_byte_order(&clusters);

		Self {
			text: text.clone(),
			wrapping,
			max_width,
			measured_width,
			measured_height,
			glyph_count,
			cluster_count: clusters.len(),
			font_count,
			runs: runs.into(),
			clusters: clusters.into(),
			line_byte_offsets: Arc::from(line_byte_offsets(text.as_ref())),
			byte_order: byte_order.into(),
			warnings: Vec::new().into(),
		}
	}
}
