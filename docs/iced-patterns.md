# Iced Patterns

Reusable Iced patterns collected from local code and installed `iced-0.14` sources, focused on less-obvious APIs and reusable composition patterns.

## 1. Application Builder With Theme, Subscription, And Embedded Fonts

Source: `demo-app/src/main.rs`

Builder-style `iced::application(...)` keeps startup wiring small while still attaching theme, subscription, and settings.

```rust
fn run_app() -> iced::Result {
    let settings = iced::Settings {
        default_font: iced::Font::with_name("Noto Sans CJK SC"),
        fonts: vec![
            include_bytes!("../../fonts/JetBrainsMono-Regular.ttf")
                .as_slice()
                .into(),
            include_bytes!("../../fonts/NotoSansCJKsc-Regular.otf")
                .as_slice()
                .into(),
        ],
        ..Default::default()
    };

    iced::application(app::DemoApp::new, app::DemoApp::update, ui::view)
        .subscription(app::DemoApp::subscription)
        .theme(app::DemoApp::theme)
        .settings(settings)
        .run()
}
```

Keep:

- builder-style application bootstrapping
- fonts loaded into `Settings` at startup
- `theme(...)` and `subscription(...)` attached without implementing an application trait

For wasm, only the entrypoint changes:

```rust
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    run_app().map_err(|e| JsValue::from_str(&e.to_string()))
}
```

## 2. Combining Frame Ticks With Global Event Listening

Source: `demo-app/src/app.rs`

Use `Subscription::batch` when a UI needs both frame ticks and global events.

```rust
pub fn subscription(_state: &Self) -> Subscription<Message> {
    Subscription::batch([
        window::frames().map(|_| Message::Tick),
        event::listen().map(Message::WindowEvent),
    ])
}
```

Notes:

- `window::frames()` is a straightforward animation tick
- `event::listen()` gives one place to observe top-level Iced events
- `Subscription::batch` keeps unrelated event streams unified under one message enum

## 3. Async File Operations With `Task::perform`

Source: `demo-app/src/app.rs`

Use `Task::perform` for user-triggered async work.

```rust
fn handle_file_open(&mut self) -> Task<Message> {
    self.log("INFO", &format!("Opening file for {:?} editor...", self.active_tab_id));
    Task::perform(file_ops::open_file_dialog(), Message::FileOpened)
}

fn handle_file_save(&mut self) -> Task<Message> {
    let tab_snapshot = self
        .tabs
        .iter()
        .find(|t| t.id == self.active_tab_id)
        .map(|tab| (tab.file_path.clone(), tab.editor.content()));

    let Some((file_path, content)) = tab_snapshot else {
        self.log("ERROR", "No active tab to save");
        return Task::none();
    };

    if let Some(path) = file_path {
        Task::perform(file_ops::save_file(path, content), Message::FileSaved)
    } else {
        self.update(Message::SaveFileAs)
    }
}
```

Pattern:

- capture just enough state before launching the task
- keep the async boundary narrow
- map results directly into the parent message enum

## 4. Mapping Child Messages Back Into A Parent Message

Source: `demo-app/src/app.rs`

Map child messages back into the parent enum with `Task::map`.

```rust
let task = editor.reset(&content);
task.map(move |e| Message::EditorEvent(target_tab_id, e))
```

The same pattern also works for follow-up editor actions:

```rust
return tab
    .editor
    .set_cursor(line, col)
    .map(move |e| Message::EditorEvent(editor_id, e));
```

## 5. Running Multiple UI Tasks Together With `Task::batch`

Two common `Task::batch` uses:

Widget operations:

```rust
Task::batch([
    focus(self.search_state.search_input_id.clone()),
    select_all(self.search_state.search_input_id.clone()),
])
```

State-transition tasks:

```rust
let t1 = editor
    .reset(&content)
    .map(move |e| Message::EditorEvent(target_tab_id, e));

let t2 = editor
    .set_cursor(line, col)
    .map(move |e| Message::EditorEvent(target_tab_id, e));

Task::batch([t1, t2])
```

## 6. Intercepting A Child Widget Event And Rerouting It

Source: `demo-app/src/app.rs`

Intercept child events before forwarding them, and reroute selected ones into overlay messages.

```rust
if self.lsp_overlay.completion_visible
    && !self.lsp_overlay.completion_suppressed
    && !self.lsp_overlay.completion_items.is_empty()
{
    match event {
        EditorMessage::ArrowKey(direction, false) => {
            use iced_code_editor::ArrowDirection;
            match direction {
                ArrowDirection::Up => {
                    return Task::done(Message::LspOverlay(
                        iced_code_editor::LspOverlayMessage::CompletionNavigateUp,
                    ));
                }
                ArrowDirection::Down => {
                    return Task::done(Message::LspOverlay(
                        iced_code_editor::LspOverlayMessage::CompletionNavigateDown,
                    ));
                }
                ArrowDirection::Left | ArrowDirection::Right => {
                    self.lsp_overlay.clear_completions();
                    if !self.lsp_overlay.hover_visible {
                        self.lsp_overlay_editor = None;
                    }
                }
            }
        }
        EditorMessage::Enter => {
            return Task::done(Message::LspOverlay(
                iced_code_editor::LspOverlayMessage::CompletionConfirm,
            ));
        }
        _ => {}
    }
}
```

Key detail: `Task::done(...)` reroutes control flow without inventing async work.

## 7. Modal Layering With `stack![]` And A Click-Catching Backdrop

Source: `demo-app/src/ui.rs`

Reusable modal pattern: base content, translucent backdrop, centered modal, all layered with `stack!`.

```rust
stack![
    content,
    mouse_area(
        container(Space::new().width(Length::Fill).height(Length::Fill)).style(|_| container::Style {
            background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.5).into()),
            ..Default::default()
        })
    )
    .on_press(Message::ToggleSettings),
    modal
]
```

Notes:

- the backdrop is a real widget, not a special-case renderer trick
- the backdrop also handles dismiss-on-click
- the modal stays in normal Iced composition instead of requiring custom overlay plumbing

## 8. Horizontally Scrollable Tabs With Custom Rails

Source: `demo-app/src/ui.rs`

Use a `scrollable` row for tab strips instead of hand-rolled overflow handling.

```rust
let tabs_list = row(
    app.tabs
        .iter()
        .map(|tab| view_tab_header(tab, tab.id == app.active_tab_id))
        .collect::<Vec<_>>()
)
.spacing(2);

let tab_bar = scrollable(tabs_list)
    .direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new()))
    .height(tab_bar_height)
    .style(|theme: &Theme, _status| {
        let palette = theme.extended_palette();
        scrollable::Style {
            container: container::Style {
                background: Some(palette.background.weak.color.into()),
                ..Default::default()
            },
            vertical_rail: scrollable::Rail {
                background: Some(palette.background.weak.color.into()),
                border: iced::Border {
                    radius: 4.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                scroller: scrollable::Scroller {
                    background: palette.primary.weak.color.into(),
                    border: iced::Border {
                        radius: 4.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                },
            },
            horizontal_rail: scrollable::Rail {
                background: Some(palette.background.weak.color.into()),
                border: iced::Border {
                    radius: 4.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                scroller: scrollable::Scroller {
                    background: palette.primary.weak.color.into(),
                    border: iced::Border {
                        radius: 4.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                },
            },
            gap: None,
            auto_scroll: scrollable::AutoScroll {
                background: Color::TRANSPARENT.into(),
                border: iced::Border::default(),
                shadow: iced::Shadow::default(),
                icon: Color::TRANSPARENT,
            },
        }
    });
```

The important part is explicit, theme-aware rail styling. It makes overflow feel intentional.

## 9. Tab Headers Built Out Of Ordinary Widgets

Source: `demo-app/src/ui.rs`

Tab headers can stay simple: `container + column + button + Space`, with an active indicator on top.

```rust
fn view_tab_header(tab: &EditorTab, is_active: bool) -> Element<'_, Message> {
    let label_text = text(label).size(14).style(move |theme: &Theme| {
        let palette = theme.extended_palette();
        text::Style {
            color: Some(if is_active {
                palette.background.base.text
            } else {
                let mut color = palette.background.base.text;
                color.a = 0.6;
                color
            }),
        }
    });

    let indicator: Element<'_, Message> = if is_active {
        container(Space::new())
            .width(Length::Fill)
            .height(2)
            .style(move |theme: &Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(palette.primary.base.color.into()),
                    ..Default::default()
                }
            })
            .into()
    } else {
        container(Space::new()).width(Length::Fill).height(2).into()
    };

    container(
        column![
            indicator,
            container(
                row![
                    button(label_text)
                        .on_press(Message::SelectTab(tab.id))
                        .style(button::text),
                    button(text("×").size(16))
                        .on_press(Message::CloseTab(tab.id))
                        .padding(0)
                        .width(20)
                        .style(button::text)
                ]
                .spacing(5)
                .align_y(iced::Center)
            )
            .padding([3, 10])
            .height(Length::Fill)
        ]
        .spacing(0)
    )
    .height(38)
    .into()
}
```

## 10. Custom `canvas::Program` Event Routing

`canvas::Program` can be a real interactive widget, not just a passive drawing surface.

```rust
impl canvas::Program<Message> for GlyphCanvas {
    type State = CanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let cursor_target =
            cursor.position_in(bounds).and_then(|position| self.hit_test(position));

        match event {
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.hovered_target != cursor_target {
                    state.hovered_target = cursor_target;
                    return Some(canvas::Action::publish(Message::CanvasHovered(cursor_target)));
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if cursor.is_over(bounds) {
                    state.hovered_target = cursor_target;
                    return Some(
                        canvas::Action::publish(Message::CanvasSelected(cursor_target))
                            .and_capture(),
                    );
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(_)) => {
                if cursor.is_over(bounds) {
                    return Some(canvas::Action::capture());
                }
            }
            _ => {
                if !cursor.is_over(bounds) && state.hovered_target.is_some() {
                    state.hovered_target = None;
                    return Some(canvas::Action::publish(Message::CanvasHovered(None)));
                }
            }
        }

        None
    }
}
```

Keep:

- `Action::publish(...)` for widget-to-parent messages
- `.and_capture()` when the widget should own the event
- explicit hover reset when the cursor leaves bounds
- a widget-local `State` type for hover bookkeeping

## 11. Drawing Mixed Primitive Geometry And Text In A Canvas

The same canvas also mixes primitive drawing with `canvas::Text`.

```rust
fn draw(
    &self,
    _state: &Self::State,
    renderer: &iced::Renderer,
    _theme: &Theme,
    bounds: Rectangle,
    _cursor: iced::mouse::Cursor,
) -> Vec<canvas::Geometry> {
    let mut frame = canvas::Frame::new(renderer, bounds.size());
    let origin = scene_origin();

    frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::from_rgb8(20, 24, 32));

    frame.stroke_rectangle(
        origin,
        Size::new(self.scene.max_width.max(1.0), self.scene.measured_height.max(1.0)),
        canvas::Stroke::default()
            .with_width(1.0)
            .with_color(Color::from_rgba(0.8, 0.8, 0.9, 0.65)),
    );

    frame.fill_text(canvas::Text {
        content: self.scene.text.clone(),
        position: origin,
        max_width: self.scene.max_width,
        color: Color::from_rgba(0.4, 0.8, 1.0, 0.9),
        size: Pixels(self.scene.font_size),
        line_height: LineHeight::Absolute(Pixels(self.scene.line_height)),
        font: self.scene.font,
        align_x: Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: self.scene.shaping.to_iced(),
    });

    vec![frame.into_geometry()]
}
```

`canvas` is useful for text-heavy debug overlays and bespoke editors too.

## 12. Generic Overlay Rendering With Message Mapping

Make overlay renderers generic over the parent message type and accept a mapper function.

```rust
pub fn view_lsp_overlay<'a, M: Clone + 'a>(
    state: &'a LspOverlayState,
    editor: &'a CodeEditor,
    theme: &'a Theme,
    font_size: f32,
    line_height: f32,
    f: impl Fn(LspOverlayMessage) -> M + 'a,
) -> Element<'a, M> {
    let msg_hover_entered = f(LspOverlayMessage::HoverEntered);
    let msg_hover_exited = f(LspOverlayMessage::HoverExited);
    let msg_completion_closed = f(LspOverlayMessage::CompletionClosed);
    let msg_completion_selected: Vec<M> = (0..state.completion_items.len())
        .map(|i| f(LspOverlayMessage::CompletionSelected(i)))
        .collect();

    let mut has_overlay = false;

    let hover_layer = build_hover_layer(
        state,
        editor,
        theme,
        (font_size, line_height),
        msg_hover_entered,
        msg_hover_exited,
        &mut has_overlay,
    );

    let completion_layer = build_completion_layer(
        state,
        editor,
        line_height,
        msg_completion_closed,
        msg_completion_selected,
        &mut has_overlay,
    );

    if !has_overlay {
        return container(Space::new().width(Length::Shrink).height(Length::Shrink)).into();
    }

    let base = container(Space::new().width(Length::Fill).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill);

    stack![base, completion_layer, hover_layer].into()
}
```

Keep:

- the overlay module stays reusable
- the caller keeps ownership of the real message enum
- message mapping happens at the boundary instead of leaking outer-layer concerns inward

## 13. Overlay Hover Tooltips That Stay Interactive

Wrap hover tooltips in `mouse_area` so they stay alive while the pointer moves inside them.

```rust
let hover_box: Element<'_, M> = mouse_area(hover_box)
    .on_enter(msg_entered.clone())
    .on_move(move |_| msg_entered.clone())
    .on_exit(msg_exited)
    .into();
```

Useful for tooltips, inspectors, scrubbers, and other transient panes.

The layout uses ordinary spacer widgets:

```rust
container(
    column![
        Space::new().height(Length::Fixed(offset_y)),
        row![Space::new().width(Length::Fixed(offset_x)), hover_box]
    ]
    .spacing(0)
    .width(Length::Fill)
    .height(Length::Fill),
)
.width(Length::Fill)
.height(Length::Fill)
```

## 14. Click-Outside-To-Dismiss Using A Transparent Full-Screen Button

Dismiss the completion menu with a transparent full-screen button below the content layer.

```rust
let click_outside = button(Space::new().width(Length::Fill).height(Length::Fill))
    .width(Length::Fill)
    .height(Length::Fill)
    .on_press(msg_closed)
    .style(|_theme: &Theme, _status| button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        ..Default::default()
    });

stack![click_outside, completion_content].into()
```

Notes:

- it preserves normal message flow
- it avoids ad hoc hit-testing logic
- it makes overlay dismissal obvious and local to the overlay implementation

## 15. Widget Operations: `focus`, `select_all`, And `scroll_to`

Use widget operations for imperative focus and scrolling.

Focus and select-all:

```rust
Task::batch([
    focus(self.search_state.search_input_id.clone()),
    select_all(self.search_state.search_input_id.clone()),
])
```

Scroll to a known offset:

```rust
return scroll_to(
    Id::new("completion_scrollable"),
    scrollable::AbsoluteOffset { x: 0.0, y: scroll_y },
);
```

## 16. Clipboard Read As A Task Chain

If no text is available yet, read the clipboard and chain into a second paste message.

```rust
fn handle_paste_msg(&mut self, text: &str) -> Task<Message> {
    self.end_grouping_if_active();

    if text.is_empty() {
        iced::clipboard::read()
            .and_then(|clipboard_text| Task::done(Message::Paste(clipboard_text)))
    } else {
        self.paste_text(text);
        self.finish_edit_operation();
        self.scroll_to_cursor()
    }
}
```

Use this when a command may either:

- use provided data immediately, or
- fetch missing data from an Iced-side capability first

## 17. Turning A Synchronous Event Drain Into Batched Messages

After draining an `mpsc` receiver synchronously, turn the collected messages into a task batch.

```rust
if messages.is_empty() {
    Task::none()
} else {
    Task::batch(messages.into_iter().map(|msg| Task::perform(async move { msg }, |m| m)))
}
```

Useful when a synchronous subsystem needs to hand multiple messages back into normal Iced update flow.

## 18. Zero-Size Placeholder Elements

Source: several files

Use `Space::new()` with shrink sizing as a typed empty element.

```rust
container(Space::new().width(Length::Shrink).height(Length::Shrink)).into()
```

## 19. Theme-Aware Styling Via Palette Closures

Common styling pattern:

```rust
.style(move |theme: &Theme| {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(palette.background.weak.color.into()),
        border: iced::Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
})
```

Benefits:

- local to the widget being built
- derived from the active Iced theme
- easy to refactor into reusable helper functions later

## 20. A Minimal Overlay Gate At The Root Boundary

Before rendering the overlay, gate on actual overlay ownership.

```rust
pub fn view_lsp_overlay(app: &DemoApp, editor_id: EditorId) -> Element<'_, Message> {
    if app.lsp_overlay_editor != Some(editor_id) {
        return container(Space::new().width(Length::Shrink).height(Length::Shrink)).into();
    }

    let Some(tab) = app.tabs.iter().find(|t| t.id == editor_id) else {
        return container(Space::new().width(Length::Shrink).height(Length::Shrink)).into();
    };

    iced_code_editor::view_lsp_overlay(
        &app.lsp_overlay,
        &tab.editor,
        &app.current_theme,
        app.current_font_size,
        app.current_line_height,
        Message::LspOverlay,
    )
}
```

Keep:

- overlay ownership stays in top-level state
- the overlay renderer stays generic
- the root view decides when a child view is allowed to render its transient UI

## 21. Size-Aware Layout With `responsive(...)`

Source: Cargo cache `iced_widget-0.14.2/src/responsive.rs`, `helpers.rs`

`responsive(...)` builds from the maximum available layout size. It fits breakpoint-style layouts without storing window width in state.

```rust
use iced::widget::{column, responsive, row, text};
use iced::{Element, Length, Size};

fn view(state: &State) -> Element<'_, Message> {
    responsive(|size: Size| {
        if size.width > 900.0 {
            row![
                left_sidebar(state),
                main_content(state),
            ]
            .spacing(16)
            .into()
        } else {
            column![
                main_content(state),
                left_sidebar(state),
            ]
            .spacing(12)
            .into()
        }
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
```

Notes:

- it reacts to layout constraints, not just the top-level window size
- it avoids pushing transient size data through your update loop
- it is a natural fit for desktop/mobile split layouts

## 22. Dependency-Scoped View Caching With `lazy(...)`

Source: Cargo cache `iced_widget-0.14.2/src/lazy.rs`, `src/lazy/helpers.rs`

`lazy` is feature-gated and only rebuilds its subtree when the hashed dependency changes.

```rust
#[cfg(feature = "lazy")]
use iced::widget::{column, lazy, text};

#[cfg(feature = "lazy")]
fn expensive_panel(state: &State) -> Element<'_, Message> {
    lazy(
        (state.active_tab, state.theme_name.clone()),
        |(active_tab, theme_name)| {
            column![
                text(format!("Tab: {:?}", active_tab)),
                build_expensive_preview(*active_tab, theme_name),
            ]
            .into()
        },
    )
    .into()
}
```

Use cases:

- expensive derived panels
- inspectors whose output depends on a small dependency tuple
- subtrees that churn too often in a large view

Constraint:

- the dependency must implement `Hash`
- this is about rebuild continuity, not state ownership

## 23. Stable Child Continuity With `keyed_column(...)`

Source: Cargo cache `iced_widget-0.14.2/src/helpers.rs`

`keyed_column(...)` preserves continuity across reordered children.

```rust
use iced::widget::{button, keyed_column, text};

fn tabs_list(state: &State) -> Element<'_, Message> {
    keyed_column(state.tabs.iter().map(|tab| {
        (
            tab.id,
            button(text(&tab.title))
                .on_press(Message::SelectTab(tab.id))
                .into(),
        )
    }))
    .spacing(4)
    .into()
}
```

Use it when:

- a list is reordered frequently
- child state should survive reordering
- continuity matters more than raw append/remove behavior

## 24. Scroll Anchoring Is Built In

Source: Cargo cache `iced_widget-0.14.2/src/scrollable.rs`

`scrollable` includes anchoring helpers:

```rust
use iced::widget::scrollable;

let log_view = scrollable(log_lines)
    .anchor_bottom()
    .on_scroll(Message::LogScrolled);

let timeline = scrollable(events_row)
    .direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new()))
    .anchor_right();
```

Useful for:

- logs and consoles that should stick to the bottom
- horizontally growing tracks that should stay pinned to the latest content

Related imperative helpers:

```rust
use iced::widget::operation::{scroll_by, snap_to_end};

let jump_to_latest = snap_to_end::<Message>("log_scroll");
let nudge = scroll_by::<Message>(
    "log_scroll",
    iced::widget::operation::AbsoluteOffset { x: 0.0, y: 120.0 },
);
```

## 25. Focus Traversal Is A First-Class Operation

Source: Cargo cache `iced_runtime-0.14.0/src/widget/operation.rs`

The runtime exposes focus traversal directly, so many forms do not need manual “next field” tracking.

```rust
use iced::Task;
use iced::widget::operation::{focus_next, focus_previous};

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::TabForward => focus_next(),
        Message::TabBackward => focus_previous(),
        _ => Task::none(),
    }
}
```

Useful for:

- keyboard-heavy tools
- property panels
- dialogs with custom key handling

## 26. Custom Streaming Subscriptions With `Subscription::run` And `run_with`

Source: Cargo cache `iced_futures-0.14.0/src/subscription.rs`, `iced-0.14.0/src/lib.rs`

`iced` exposes `Subscription::run` and `run_with` for custom passive data sources.

```rust
use iced::Subscription;
use futures::stream;

#[derive(Debug, Clone)]
enum Message {
    WorkerEvent(String),
}

fn worker() -> impl futures::Stream<Item = Message> {
    stream::iter(vec![
        Message::WorkerEvent("boot".into()),
        Message::WorkerEvent("ready".into()),
    ])
}

fn subscription(state: &State) -> Subscription<Message> {
    Subscription::run(worker)
}
```

When stream identity depends on data:

```rust
fn subscription(state: &State) -> Subscription<Message> {
    Subscription::run_with(state.session_id, |session_id| {
        build_session_stream(*session_id)
    })
}
```

Subscriptions are declarative and identity-based; `run_with` restarts a stream when its driving data changes.

## 27. Absolute Positioning With `pin(...)`

Source: Cargo cache `iced_widget-0.14.2/src/pin.rs`, `helpers.rs`

`pin(...)` positions normal widgets at fixed coordinates inside its bounds.

```rust
use iced::widget::{container, pin, text};
use iced::{Element, Length};

fn overlay_badges(state: &State) -> Element<'_, Message> {
    container(
        pin(
            container(text("42"))
                .padding([2, 6])
                .style(container::rounded_box),
        )
        .x(12)
        .y(8),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
```

Useful for:

- HUD labels
- fixed editor annotations
- overlay badges inside a bounded region

## 28. Floating A Widget Above Neighbors With `float(...)`

Source: Cargo cache `iced_widget-0.14.2/src/float.rs`, `helpers.rs`

`float(...)` differs from `pin(...)`: it keeps normal layout size but can scale or translate rendered content above neighbors.

```rust
use iced::widget::{button, float, text};
use iced::{Rectangle, Vector};

let hovered_card = float(
    button(text("Preview"))
)
.scale(1.05)
.translate(|bounds: Rectangle, viewport: Rectangle| {
    let overflow_right = (bounds.x + bounds.width) - viewport.width;

    if overflow_right > 0.0 {
        Vector::new(-overflow_right - 8.0, 0.0)
    } else {
        Vector::ZERO
    }
});
```

Useful for:

- hover cards
- zoom-on-hover grids
- menus or transient emphasis effects that still want widget semantics

## 29. `pane_grid` Is Still The Right Primitive For IDE Layouts

Source: Cargo cache `iced_widget-0.14.2/src/pane_grid.rs`

The installed Iced docs still show a simple `pane_grid` API that fits editor and workbench shells well.

```rust
use iced::widget::{pane_grid, text};

fn view(state: &State) -> Element<'_, Message> {
    pane_grid(&state.panes, |pane, pane_state, is_maximized| {
        pane_grid::Content::new(match pane_state {
            Pane::Editor => text("Editor"),
            Pane::Console => text("Console"),
        })
    })
    .on_drag(Message::PaneDragged)
    .on_resize(10, Message::PaneResized)
    .into()
}
```

If the UI grows into a real multi-pane environment, this is still the first primitive to reach for.

## Closing Note

If the implementation changes wholesale, port these first:

```rust
// 1. builder-style application wiring
iced::application(App::new, App::update, App::view)
    .subscription(App::subscription)
    .theme(App::theme)
    .run()
```

```rust
// 2. layered UI with stack![]
stack![base, backdrop, modal]
```

```rust
// 3. child-message mapping
child_task.map(Message::Child)
```

```rust
// 4. multiple side effects in one update branch
Task::batch([focus(id.clone()), select_all(id.clone())])
```

```rust
// 5. custom interactive canvas widgets
canvas::Action::publish(msg).and_capture()
```

```rust
// 6. generic overlay renderers with a message mapper
fn view_overlay<M>(..., map: impl Fn(OverlayMessage) -> M) -> Element<'_, M>
```
