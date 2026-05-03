use gpui::{
    prelude::*, App, Application, Context, EventEmitter, FocusHandle, Focusable, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Render, Window, WindowOptions,
};

pub mod field;
mod note;
pub mod style;
pub mod view;

use crate::ui::field::FieldId;
use crate::ui::style as s;
use note::Note;

pub fn run() {
    Application::new().run(|cx: &mut App| {
        let window_handle = cx
            .open_window(WindowOptions::default(), |window, cx| {
                window.set_window_title("Ocotelolco Notes");
                let focus_handle = cx.focus_handle();

                cx.new(|cx| {
                    cx.subscribe_self(Model::handled_note_event).detach();

                    Model {
                        focus_handle,
                        notes: Vec::new(),
                        active_field: None,
                        next_note_id: 1,
                        pointer_interaction: None,
                        new_button_pressed: false,
                        pressed_close_note_index: None,
                    }
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

pub(super) struct Model {
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
        self.notes
            .push(Note::new(note_id, self.notes.len() + 1, offset));
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
                note.width = (note.width + dx).max(note::MIN_SIZE);
                note.height = (note.height + dy).max(note::MIN_SIZE);
            }
        }
        self.pointer_interaction = Some(pointer_interaction);
        cx.notify();
    }

    fn released_mouse(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.pointer_interaction = None;
        cx.notify();
    }

    fn pressed_name_key(&mut self, key_press: &note::KeyPress, cx: &mut Context<Self>) {
        let Some(active_field) = self.active_field() else {
            self.active_field = None;
            return;
        };

        match key_press {
            note::KeyPress::Backspace => {
                if let Some(note) = self.notes.get_mut(active_field.note_index) {
                    note.pressed_name_backspace();
                }
                cx.notify();
            }
            note::KeyPress::Enter => {
                let body_field_id = if let Some(note) = self.notes.get_mut(active_field.note_index)
                {
                    note.clicked_save_name();
                    note.body_field_id()
                } else {
                    self.active_field = None;
                    cx.notify();
                    return;
                };
                self.active_field = Some(body_field_id);
                cx.notify();
            }
            note::KeyPress::Text(key_char) => {
                if let Some(note) = self.notes.get_mut(active_field.note_index) {
                    note.pressed_name_key(key_char);
                }
                cx.notify();
            }
        }
    }

    fn handled_note_event(&mut self, event: &note::Event, cx: &mut Context<Self>) {
        match event {
            note::Event::PressedHeader { note_index, x, y } => {
                let ni = *note_index;
                if let Some(note) = self.notes.get(ni) {
                    self.active_field = Some(note.body_field_id());
                }
                self.pointer_interaction =
                    Some(PointerInteraction::Drag(PointerInteractionState {
                        note_index: ni,
                        last_x: *x,
                        last_y: *y,
                    }));
                cx.notify();
            }
            note::Event::PressedResizeHandle { note_index, x, y } => {
                let ni = *note_index;
                if let Some(note) = self.notes.get(ni) {
                    self.active_field = Some(note.body_field_id());
                }
                self.pointer_interaction =
                    Some(PointerInteraction::Resize(PointerInteractionState {
                        note_index: ni,
                        last_x: *x,
                        last_y: *y,
                    }));
                cx.notify();
            }
            note::Event::PressedBodyEditor { note_index } => {
                if let Some(note) = self.notes.get(*note_index) {
                    self.active_field = Some(note.body_field_id());
                }
                cx.notify();
            }
            note::Event::PressedNameEditor { note_index } => {
                if let Some(note) = self.notes.get(*note_index) {
                    self.active_field = Some(note.name_field_id());
                }
                cx.notify();
            }
            note::Event::ClickedRename { note_index } => {
                let ni = *note_index;
                if ni >= self.notes.len() {
                    return;
                }

                if let Some(note) = self.notes.get_mut(ni) {
                    note.clicked_rename();
                    self.active_field = Some(note.name_field_id());
                }
                cx.notify();
            }
            note::Event::ClickedSaveName { note_index } => {
                let ni = *note_index;
                if let Some(note) = self.notes.get_mut(ni) {
                    note.clicked_save_name();
                    if self.active_field == Some(note.name_field_id()) {
                        self.active_field = Some(note.body_field_id());
                    }
                }
                cx.notify();
            }
            note::Event::PressedCloseButton { note_index } => {
                self.pressed_close_note_index = Some(*note_index);
                cx.notify();
            }
            note::Event::ClickedCloseButton { note_index } => {
                let ni = *note_index;

                if let Some(closed_note) = self.notes.get(ni) {
                    self.pressed_close_note_index = None;
                    if self.active_field == Some(closed_note.name_field_id())
                        || self.active_field == Some(closed_note.body_field_id())
                    {
                        self.active_field = None;
                    }
                    self.notes.remove(ni);
                    self.pointer_interaction = None;
                    cx.notify();
                }
            }
            note::Event::ReleasedCloseButtonOutside => {
                self.pressed_close_note_index = None;
                cx.notify();
            }
            note::Event::PressedKey(key_press) => {
                let Some(active_field) = self.active_field() else {
                    self.active_field = None;
                    return;
                };

                if active_field.kind == FieldKind::Name {
                    self.pressed_name_key(key_press, cx);
                    return;
                }

                let Some(note) = self.notes.get_mut(active_field.note_index) else {
                    self.active_field = None;
                    return;
                };

                match key_press {
                    note::KeyPress::Backspace => {
                        note.content.pop();
                        cx.notify();
                    }
                    note::KeyPress::Enter => {
                        note.content.push('\n');
                        cx.notify();
                    }
                    note::KeyPress::Text(key_char) => {
                        note.content.push_str(key_char);
                        cx.notify();
                    }
                }
            }
        }
    }
}

impl EventEmitter<note::Event> for Model {}

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
                note::render(
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
