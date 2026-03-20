//! Thin GUI/runtime attachment surface.
//!
//! This crate contains the rendering client and launcher logic, but its public
//! library surface is intentionally narrow: [`launcher`] is the reusable entry
//! point for starting or joining a runtime-backed GUI session.
//!
//! The GUI should translate widget-local behavior into public runtime calls or
//! the private GUI session protocol from `glorp_runtime`. It should not become a
//! second owner of editor semantics, config semantics, or persistence rules.
//!
//! Most GUI modules are app-internal composition details. The stable surface is
//! the attach/start path in [`launcher`], which can host an owned runtime,
//! attach to an existing one, and present one client shape across both modes.
//!
//! The GUI then rebuilds richer local editor and scene state from the runtime's
//! narrower document/config contract, which is why the launcher/client boundary
//! matters more than most internal app modules.

pub mod launcher;

pub use self::launcher::{GuiLaunchOptions, GuiRuntimeClient, GuiRuntimeSession};
