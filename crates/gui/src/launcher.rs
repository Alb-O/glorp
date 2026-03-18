use {
	cosmic_text::Buffer,
	glorp_api::{EditorConfig, GlorpCall, GlorpCallDescriptor, GlorpCallResult, GlorpCaller, GlorpError},
	glorp_editor::{
		EditorPresentation, EditorTextLayerState, SessionSnapshot, build_buffer, make_font_system, scene_config,
	},
	glorp_runtime::{
		GuiCommand, GuiRuntimeFrame, GuiTransportFrame, RuntimeHost, RuntimeOptions, default_runtime_paths,
	},
	glorp_transport::{
		GuiTransportRequest, GuiTransportResponse, IpcClient, IpcServerHandle, default_socket_path,
		gui_transport_request, socket_is_live, start_server_shared,
	},
	std::{
		path::{Path, PathBuf},
		sync::{Arc, Mutex},
		thread,
		time::{Duration, Instant},
	},
};

#[derive(Debug, Clone)]
pub struct GuiLaunchOptions {
	pub repo_root: PathBuf,
	pub socket_path: PathBuf,
}

impl GuiLaunchOptions {
	#[must_use]
	pub fn for_repo_root(repo_root: impl Into<PathBuf>) -> Self {
		let repo_root = repo_root.into();
		Self {
			socket_path: default_socket_path(&repo_root),
			repo_root,
		}
	}
}

pub struct GuiRuntimeSession {
	socket_path: PathBuf,
	server: Option<IpcServerHandle>,
}

pub struct GuiRuntimeClient {
	client: IpcClient,
	socket_path: PathBuf,
	text_layer: TextLayerCache,
}

#[derive(Default)]
struct TextLayerCache {
	key: Option<TextLayerKey>,
	font_system: Option<cosmic_text::FontSystem>,
	buffer: Option<Arc<Buffer>>,
}

#[derive(Debug, Clone, PartialEq)]
struct TextLayerKey {
	document_text: String,
	editor_config: EditorConfig,
	layout_width_bits: u32,
}

impl GuiRuntimeSession {
	pub fn start_owned(options: GuiLaunchOptions) -> Result<(Self, GuiRuntimeClient), GlorpError> {
		if socket_is_live(&options.socket_path) {
			return Err(GlorpError::transport(format!(
				"shared GUI socket already active at {}",
				options.socket_path.display()
			)));
		}

		ensure_socket_parent(&options.socket_path)?;

		let host = Arc::new(Mutex::new(RuntimeHost::new(RuntimeOptions {
			paths: default_runtime_paths(&options.repo_root),
		})?));
		let server = start_server_shared(options.socket_path.as_path(), host)?;
		wait_for_socket(&options.socket_path)?;

		let mut client = GuiRuntimeClient::new(options.socket_path.clone());
		ensure_runtime_capabilities(&mut client, "unexpected capabilities response from GUI runtime")?;

		Ok((
			Self {
				socket_path: options.socket_path,
				server: Some(server),
			},
			client,
		))
	}

	pub fn connect_or_start(options: GuiLaunchOptions) -> Result<(Self, GuiRuntimeClient), GlorpError> {
		if !socket_is_live(&options.socket_path) {
			return Self::start_owned(options);
		}

		let mut client = GuiRuntimeClient::new(options.socket_path.clone());
		ensure_runtime_capabilities(&mut client, "unexpected capabilities response from shared GUI runtime")?;

		Ok((
			Self {
				socket_path: options.socket_path,
				server: None,
			},
			client,
		))
	}

	#[must_use]
	pub fn socket_path(&self) -> &Path {
		&self.socket_path
	}

	#[must_use]
	pub const fn owns_server(&self) -> bool {
		self.server.is_some()
	}

	pub fn shutdown(&mut self) -> Result<(), GlorpError> {
		self.server.take().map_or(Ok(()), IpcServerHandle::shutdown)
	}
}

impl GuiRuntimeClient {
	#[must_use]
	pub fn new(socket_path: impl Into<PathBuf>) -> Self {
		let socket_path = socket_path.into();
		Self {
			client: IpcClient::new(socket_path.as_path()),
			socket_path,
			text_layer: TextLayerCache::default(),
		}
	}

	pub fn execute_gui(&mut self, command: GuiCommand) -> Result<(), GlorpError> {
		let GuiTransportResponse::ExecuteGui(result) =
			gui_transport_request(&self.socket_path, GuiTransportRequest::ExecuteGui(command))?
		else {
			return Err(GlorpError::transport("unexpected private gui execute response"));
		};
		result
	}

	pub fn gui_frame(&mut self) -> Result<GuiRuntimeFrame, GlorpError> {
		let GuiTransportResponse::GuiFrame(result) =
			gui_transport_request(&self.socket_path, GuiTransportRequest::GuiFrame)?
		else {
			return Err(GlorpError::transport("unexpected private gui frame response"));
		};
		Ok(self.hydrate_frame((*result)?))
	}

	fn hydrate_frame(&mut self, frame: GuiTransportFrame) -> GuiRuntimeFrame {
		let buffer = Arc::clone(self.text_layer.buffer(&frame));
		let editor = frame.snapshot.editor;
		let snapshot = SessionSnapshot {
			editor: EditorPresentation::new(
				editor.revision,
				editor.viewport_metrics,
				EditorTextLayerState {
					buffer: Arc::downgrade(&buffer),
					measured_height: editor.viewport_metrics.measured_height,
				},
				editor.editor,
				editor.editor_bytes,
				editor.undo_depth,
				editor.redo_depth,
			),
			scene: frame.snapshot.scene,
		};

		GuiRuntimeFrame {
			config: frame.config,
			ui: frame.ui,
			revisions: frame.revisions,
			snapshot,
			document_text: frame.document_text,
		}
	}
}

impl GlorpCaller for GuiRuntimeClient {
	fn call(&mut self, call: GlorpCall) -> Result<GlorpCallResult, GlorpError> {
		self.client.call(call)
	}
}

impl TextLayerCache {
	fn buffer(&mut self, frame: &GuiTransportFrame) -> &Arc<Buffer> {
		let key = TextLayerKey {
			document_text: frame.document_text.clone(),
			editor_config: frame.config.editor.clone(),
			layout_width_bits: frame.ui.layout_width.to_bits(),
		};

		if self.key.as_ref() != Some(&key) {
			let font_system = self.font_system.get_or_insert_with(make_font_system);
			let buffer = build_buffer(
				font_system,
				key.document_text.as_str(),
				scene_config(
					key.editor_config.font,
					key.editor_config.shaping,
					key.editor_config.wrapping,
					key.editor_config.font_size,
					key.editor_config.line_height,
					frame.ui.layout_width,
				),
			);
			self.buffer = Some(Arc::new(buffer));
			self.key = Some(key);
		}

		self.buffer
			.as_ref()
			.expect("text layer cache should hold a buffer after hydration")
	}
}

fn ensure_socket_parent(socket_path: &Path) -> Result<(), GlorpError> {
	socket_path.parent().map_or(Ok(()), |parent| {
		std::fs::create_dir_all(parent).map_err(|error| {
			GlorpError::transport(format!("failed to create socket parent {}: {error}", parent.display()))
		})
	})
}

fn ensure_runtime_capabilities(client: &mut impl GlorpCaller, error: &'static str) -> Result<(), GlorpError> {
	glorp_api::calls::Capabilities::call(client, ())
		.map(|_| ())
		.map_err(|_| GlorpError::transport(error))
}

fn wait_for_socket(socket_path: &Path) -> Result<(), GlorpError> {
	let deadline = Instant::now() + Duration::from_secs(5);
	while Instant::now() < deadline {
		if socket_is_live(socket_path) {
			return Ok(());
		}
		thread::sleep(Duration::from_millis(10));
	}

	Err(GlorpError::transport(format!(
		"timed out waiting for GUI runtime at {}",
		socket_path.display()
	)))
}

impl Drop for GuiRuntimeSession {
	fn drop(&mut self) {
		let _ = self.shutdown();
	}
}
