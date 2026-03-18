use {
	glorp_nu_plugin::GlorpPlugin,
	nu_plugin::{JsonSerializer, serve_plugin},
};

fn main() {
	serve_plugin(&GlorpPlugin, JsonSerializer)
}
