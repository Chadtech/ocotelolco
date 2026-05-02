use gpui::{
    prelude::*, App, Application, Context, FocusHandle, Focusable, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Render, Window, WindowOptions,
};

pub mod style;
pub mod view;

use crate::ui::style as s;

pub fn run() {
    Application::new().run(|cx: &mut App| {
        let window_handle = cx
            .open_window(WindowOptions::default(), |window, cx| {
                window.set_window_title("Ocotelolco Notes");
                let focus_handle = cx.focus_handle();

                cx.new(|_| Model {
                    focus_handle,
                    notes: Vec::new(),
                    active_field: None,
                    next_note_id: 1,
                    pointer_interaction: None,
                    new_button_pressed: false,
                    pressed_close_note_index: None,
                })
            })
            .expect("failed to open notes window");

        window_handle
            .update(cx, |notes, window, cx| {
                window.focus(&notes.focus_handle);
                cx.activate(true);
            })
            .expect("failed to focus notes window");
    });
}

struct Note {
    id: u64,
    name: String,
    is_renaming: bool,
    content: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

struct Model {
    focus_handle: FocusHandle,
    notes: Vec<Note>,
    active_field: Option<FieldId>,
    next_note_id: u64,
    pointer_interaction: Option<PointerInteraction>,
    new_button_pressed: bool,
    pressed_close_note_index: Option<usize>,
}

enum PointerInteraction {
    Drag(PointerInteractionState),
    Resize(PointerInteractionState),
}

struct PointerInteractionState {
    note_index: usize,
    last_x: f32,
    last_y: f32,
}

#[derive(Clone, PartialEq, Eq)]
struct FieldId(String);

#[derive(Clone, Copy)]
struct ActiveField {
    note_index: usize,
    kind: FieldKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FieldKind {
    Body,
    Name,
}

const DEFAULT_NOTE_SIZE: f32 = 256.0;
const MIN_NOTE_SIZE: f32 = 128.0;

impl Note {
    fn name_field_id(&self) -> FieldId {
        FieldId(format!("note-{}/name", self.id))
    }

    fn body_field_id(&self) -> FieldId {
        FieldId(format!("note-{}/body", self.id))
    }
}

impl Model {
    fn pressed_new_note_button(
        &mut self,
        _: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.new_button_pressed = true;
        cx.notify();
    }

    fn clicked_new_note_button(
        &mut self,
        _: &MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.new_button_pressed = false;
        let offset = self.notes.len() as f32 * 24.0;
        let note_id = self.next_note_id;
        self.next_note_id += 1;
        let body_field_id = FieldId(format!("note-{note_id}/body"));
        self.notes.push(Note {
            id: note_id,
            name: format!("note {}", self.notes.len() + 1),
            is_renaming: false,
            content: String::new(),
            x: 32.0 + offset,
            y: 32.0 + offset,
            width: DEFAULT_NOTE_SIZE,
            height: DEFAULT_NOTE_SIZE,
        });
        self.active_field = Some(body_field_id);
        window.focus(&self.focus_handle);
        cx.notify();
    }

    fn released_new_note_button_outside(
        &mut self,
        _: &MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.new_button_pressed = false;
        cx.notify();
    }

    fn pressed_close_note_button(
        &mut self,
        note_index: usize,
        _: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pressed_close_note_index = Some(note_index);
        cx.stop_propagation();
        cx.notify();
    }

    fn clicked_close_note_button(
        &mut self,
        note_index: usize,
        _: &MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if note_index >= self.notes.len() {
            return;
        }

        self.pressed_close_note_index = None;
        let closed_note = &self.notes[note_index];
        if self.active_field == Some(closed_note.name_field_id())
            || self.active_field == Some(closed_note.body_field_id())
        {
            self.active_field = None;
        }
        self.notes.remove(note_index);
        self.pointer_interaction = None;
        cx.stop_propagation();
        cx.notify();
    }

    fn released_close_note_button_outside(
        &mut self,
        _: &MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pressed_close_note_index = None;
        cx.notify();
    }

    fn active_field(&self) -> Option<ActiveField> {
        let active_field_id = self.active_field.as_ref()?;
        self.notes
            .iter()
            .enumerate()
            .find_map(|(note_index, note)| {
                if &note.body_field_id() == active_field_id {
                    Some(ActiveField {
                        note_index,
                        kind: FieldKind::Body,
                    })
                } else if &note.name_field_id() == active_field_id {
                    Some(ActiveField {
                        note_index,
                        kind: FieldKind::Name,
                    })
                } else {
                    None
                }
            })
    }

    fn pointer_note_mut(&mut self, note_index: usize, cx: &mut Context<Self>) -> Option<&mut Note> {
        if note_index >= self.notes.len() {
            self.pointer_interaction = None;
            cx.notify();
            return None;
        }

        self.notes.get_mut(note_index)
    }

    fn moved_mouse(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(mut pointer_interaction) = self.pointer_interaction.take() else {
            return;
        };
        if !event.dragging() {
            cx.notify();
            return;
        }

        let x = f32::from(event.position.x);
        let y = f32::from(event.position.y);
        match &mut pointer_interaction {
            PointerInteraction::Drag(state) => {
                let dx = x - state.last_x;
                let dy = y - state.last_y;
                state.last_x = x;
                state.last_y = y;

                let Some(note) = self.pointer_note_mut(state.note_index, cx) else {
                    return;
                };
                note.x += dx;
                note.y += dy;
            }
            PointerInteraction::Resize(state) => {
                let dx = x - state.last_x;
                let dy = y - state.last_y;
                state.last_x = x;
                state.last_y = y;

                let Some(note) = self.pointer_note_mut(state.note_index, cx) else {
                    return;
                };
                note.width = (note.width + dx).max(MIN_NOTE_SIZE);
                note.height = (note.height + dy).max(MIN_NOTE_SIZE);
            }
        }
        self.pointer_interaction = Some(pointer_interaction);
        cx.notify();
    }

    fn pressed_note_header(
        &mut self,
        note_index: usize,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(note) = self.notes.get(note_index) {
            self.active_field = Some(note.body_field_id());
        }
        self.pointer_interaction = Some(PointerInteraction::Drag(PointerInteractionState {
            note_index,
            last_x: event.position.x.into(),
            last_y: event.position.y.into(),
        }));
        window.focus(&self.focus_handle);
        cx.stop_propagation();
        cx.notify();
    }

    fn pressed_resize_handle(
        &mut self,
        note_index: usize,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(note) = self.notes.get(note_index) {
            self.active_field = Some(note.body_field_id());
        }
        self.pointer_interaction = Some(PointerInteraction::Resize(PointerInteractionState {
            note_index,
            last_x: event.position.x.into(),
            last_y: event.position.y.into(),
        }));
        window.focus(&self.focus_handle);
        cx.stop_propagation();
        cx.notify();
    }

    fn released_mouse(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.pointer_interaction = None;
        cx.notify();
    }

    fn pressed_note_body_editor(
        &mut self,
        note_index: usize,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(note) = self.notes.get(note_index) {
            self.active_field = Some(note.body_field_id());
        }
        window.focus(&self.focus_handle);
        cx.notify();
    }

    fn clicked_rename_note(
        &mut self,
        note_index: usize,
        _: &MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if note_index >= self.notes.len() {
            return;
        }

        if let Some(note) = self.notes.get_mut(note_index) {
            note.is_renaming = true;
            self.active_field = Some(note.name_field_id());
        }
        window.focus(&self.focus_handle);
        cx.stop_propagation();
        cx.notify();
    }

    fn pressed_note_name_editor(
        &mut self,
        note_index: usize,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(note) = self.notes.get(note_index) {
            self.active_field = Some(note.name_field_id());
        }
        window.focus(&self.focus_handle);
        cx.stop_propagation();
        cx.notify();
    }

    fn clicked_save_note_name(
        &mut self,
        note_index: usize,
        _: &MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(note) = self.notes.get_mut(note_index) {
            note.is_renaming = false;
            if self.active_field == Some(note.name_field_id()) {
                self.active_field = Some(note.body_field_id());
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn pressed_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.platform || event.keystroke.modifiers.control {
            return;
        }

        let Some(active_field) = self.active_field() else {
            self.active_field = None;
            return;
        };

        if active_field.kind == FieldKind::Name {
            self.pressed_name_key(event, cx);
            return;
        }

        let Some(note) = self.notes.get_mut(active_field.note_index) else {
            self.active_field = None;
            return;
        };

        match event.keystroke.key.as_str() {
            "backspace" => {
                note.content.pop();
                cx.stop_propagation();
                cx.notify();
            }
            "enter" => {
                note.content.push('\n');
                cx.stop_propagation();
                cx.notify();
            }
            _ => {
                if let Some(key_char) = event.keystroke.key_char.as_ref() {
                    note.content.push_str(key_char);
                    cx.stop_propagation();
                    cx.notify();
                }
            }
        }
    }

    fn pressed_name_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(active_field) = self.active_field() else {
            self.active_field = None;
            return;
        };

        match event.keystroke.key.as_str() {
            "backspace" => {
                if let Some(note) = self.notes.get_mut(active_field.note_index) {
                    note.name.pop();
                }
                cx.stop_propagation();
                cx.notify();
            }
            "enter" => {
                let body_field_id = if let Some(note) = self.notes.get_mut(active_field.note_index)
                {
                    note.is_renaming = false;
                    note.body_field_id()
                } else {
                    self.active_field = None;
                    cx.stop_propagation();
                    cx.notify();
                    return;
                };
                self.active_field = Some(body_field_id);
                cx.stop_propagation();
                cx.notify();
            }
            _ => {
                if let Some(key_char) = event.keystroke.key_char.as_ref() {
                    if let Some(note) = self.notes.get_mut(active_field.note_index) {
                        note.name.push_str(key_char);
                    }
                    cx.stop_propagation();
                    cx.notify();
                }
            }
        }
    }
}

impl Focusable for Model {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Model {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_focused = self.focus_handle.is_focused(window);
        let note_windows = self
            .notes
            .iter()
            .enumerate()
            .map(|(index, note)| {
                let show_body_cursor =
                    self.active_field == Some(note.body_field_id()) && is_focused;
                let show_name_cursor =
                    self.active_field == Some(note.name_field_id()) && is_focused;
                render_note_window(
                    index,
                    note,
                    &self.focus_handle,
                    self.pressed_close_note_index == Some(index),
                    show_body_cursor,
                    show_name_cursor,
                    cx,
                )
            })
            .collect::<Vec<_>>();

        gpui::div()
            .flex()
            .flex_col()
            .size_full()
            .font_family(s::FONT)
            .bg(s::GREEN3)
            .text_color(s::GRAY6)
            .on_mouse_move(cx.listener(Self::moved_mouse))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::released_mouse))
            .child(
                gpui::div()
                    .relative()
                    .flex_1()
                    .flex()
                    .size_full()
                    .child(
                        gpui::img(std::path::Path::new(concat!(
                            env!("CARGO_MANIFEST_DIR"),
                            "/ocotelolco_bg.png"
                        )))
                        .absolute()
                        .size_full()
                        .object_fit(gpui::ObjectFit::Cover),
                    )
                    .children(note_windows),
            )
            .child(toolbar(self.new_button_pressed, cx))
    }
}

fn render_note_window(
    note_index: usize,
    note: &Note,
    focus_handle: &FocusHandle,
    close_button_pressed: bool,
    show_cursor: bool,
    show_name_cursor: bool,
    cx: &mut Context<Model>,
) -> gpui::Div {
    let mut lines = note.content.split('\n').collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push("");
    }

    s::raised(
        gpui::div()
            .flex()
            .flex_col()
            .size_full()
            .bg(s::GRAY2)
            .p(s::S3)
            .child(
                gpui::div()
                    .px(s::S4)
                    .flex()
                    .items_center()
                    .justify_between()
                    .bg(s::GRAY5)
                    .text_color(s::GREEN1)
                    .p(s::S3)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |model, event, window, cx| {
                            model.pressed_note_header(note_index, event, window, cx);
                        }),
                    )
                    .child(note.name.clone())
                    .child(close_button(note_index, close_button_pressed, cx)),
            )
            .child(rename_row(
                note_index,
                note,
                focus_handle,
                show_name_cursor,
                cx,
            ))
            .child(
                gpui::div()
                    .p(s::S3)
                    .pt(s::S0)
                    .size_full()
                    .bg(s::GRAY2)
                    .child(
                        s::sunken(
                            gpui::div()
                                .flex()
                                .flex_col()
                                .size_full()
                                .p(s::S3)
                                .bg(s::GREEN2)
                                .track_focus(focus_handle)
                                .key_context("NoteEditor")
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |model, event, window, cx| {
                                        model.pressed_note_body_editor(
                                            note_index, event, window, cx,
                                        );
                                    }),
                                )
                                .on_key_down(cx.listener(Model::pressed_key))
                                .children(render_note_lines(lines, show_cursor)),
                        )
                        .size_full(),
                    ),
            ),
    )
    .child(resize_handle(note_index, cx))
    .absolute()
    .left(gpui::px(note.x))
    .top(gpui::px(note.y))
    .w(gpui::px(note.width))
    .h(gpui::px(note.height))
}

fn rename_row(
    note_index: usize,
    note: &Note,
    focus_handle: &FocusHandle,
    show_name_cursor: bool,
    cx: &mut Context<Model>,
) -> impl IntoElement {
    let name = if show_name_cursor {
        format!("{}|", note.name)
    } else {
        note.name.clone()
    };

    let rename_control = if note.is_renaming {
        gpui::div()
            .flex()
            .items_center()
            .gap_2()
            .size_full()
            .child(
                s::sunken(
                    gpui::div()
                        .flex_1()
                        .min_h(s::S6)
                        .p(s::S3)
                        .bg(s::GREEN1)
                        .text_color(s::GRAY6)
                        .track_focus(focus_handle)
                        .key_context("NoteNameEditor")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |model, event, window, cx| {
                                model.pressed_note_name_editor(note_index, event, window, cx);
                            }),
                        )
                        .on_key_down(cx.listener(Model::pressed_key))
                        .child(name),
                )
                .flex_1(),
            )
            .child(view::button::from_text("save name", false).on_mouse_up(
                MouseButton::Left,
                cx.listener(move |model, event, window, cx| {
                    model.clicked_save_note_name(note_index, event, window, cx);
                }),
            ))
    } else {
        gpui::div().flex().items_center().size_full().child(
            view::button::from_text("rename", false).on_mouse_up(
                MouseButton::Left,
                cx.listener(move |model, event, window, cx| {
                    model.clicked_rename_note(note_index, event, window, cx);
                }),
            ),
        )
    };

    gpui::div()
        .flex()
        .items_center()
        .bg(s::GRAY2)
        .p(s::S3)
        .child(rename_control)
}

fn toolbar(new_button_pressed: bool, cx: &mut Context<Model>) -> impl IntoElement {
    gpui::div()
        .flex()
        .items_center()
        .border_t_2()
        .border_color(s::GRAY3)
        .bg(s::GRAY2)
        .p(s::S3)
        .gap_3()
        .child(
            view::button::from_text("new note", new_button_pressed)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(Model::pressed_new_note_button),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(Model::clicked_new_note_button),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(Model::released_new_note_button_outside),
                ),
        )
}

fn close_button(note_index: usize, pressed: bool, cx: &mut Context<Model>) -> gpui::Div {
    view::button::x(pressed)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |model, event, window, cx| {
                model.pressed_close_note_button(note_index, event, window, cx);
            }),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(move |model, event, window, cx| {
                model.clicked_close_note_button(note_index, event, window, cx);
            }),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(Model::released_close_note_button_outside),
        )
}

fn resize_handle(note_index: usize, cx: &mut Context<Model>) -> impl IntoElement {
    gpui::div()
        .absolute()
        .right_0()
        .bottom_0()
        .size(s::S5)
        .child(
            gpui::canvas(
                |_, _, _| {},
                |bounds, _, window, _| {
                    let mut builder = gpui::PathBuilder::stroke(s::S2);
                    builder.move_to(gpui::point(bounds.right() - s::S4, bounds.bottom() - s::S2));
                    builder.line_to(gpui::point(bounds.right() - s::S2, bounds.bottom() - s::S4));
                    builder.move_to(gpui::point(bounds.right() - s::S3, bounds.bottom() - s::S2));
                    builder.line_to(gpui::point(bounds.right() - s::S2, bounds.bottom() - s::S3));
                    if let Ok(path) = builder.build() {
                        window.paint_path(path, s::GRAY1);
                    }
                },
            )
            .size_full(),
        )
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |model, event, window, cx| {
                model.pressed_resize_handle(note_index, event, window, cx);
            }),
        )
}

fn render_note_lines(lines: Vec<&str>, is_focused: bool) -> Vec<impl IntoElement> {
    let last_index = lines.len().saturating_sub(1);

    lines
        .into_iter()
        .enumerate()
        .map(move |(index, line)| {
            let text = if is_focused && index == last_index {
                format!("{line}|")
            } else if line.is_empty() {
                " ".to_string()
            } else {
                line.to_string()
            };

            gpui::div().min_h(s::S6).text_color(s::GRAY6).child(text)
        })
        .collect()
}
