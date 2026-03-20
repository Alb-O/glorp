//! Pure editing, presentation, and layout semantics for `glorp`.
//!
//! This crate sits below the public API and the shared runtime. It owns text
//! mutation, selection, history, layout, and scene materialization, but not
//! persistence, scripting, IPC, or widget state.
//!
//! Main seams:
//!
//! - [`editor`] answers "what did the edit do to text and selection?"
//! - [`scene`] answers "how does this text materialize geometrically?"
//! - [`presentation`] is the read model shared by rendering and inspection.
//!
//! One project-specific detail matters here: the shared runtime does not host
//! the full [`editor::EditorEngine`]. It owns the lighter document/history core,
//! while the GUI keeps a local `EditorEngine` and scene snapshot in sync with
//! runtime revisions and edit deltas. That makes this crate both the source of
//! truth for edit/layout behavior and the projection engine the GUI uses to
//! rebuild rich local state.

pub mod editor;
pub mod overlay;
pub mod presentation;
pub mod scene;
pub mod telemetry;
pub mod types;

pub use self::{editor::*, overlay::*, presentation::*, scene::*, types::*};
