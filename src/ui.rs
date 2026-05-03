use gpui::{
    prelude::*, App, Application, Context, EventEmitter, FocusHandle, Focusable, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Render, Window, WindowOptions,
};
use std::collections::HashMap;

pub mod field;
mod note;
pub mod style;
pub mod view;

use crate::ui::field::FieldId;
use crate::ui::style as s;
use note::{Note, NoteId};

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
                        notes: HashMap::new(),
                        note_order: Vec::new(),
                        active_field: None,
                        next_note_id: NoteId(1),
                        pointer_interaction: None,
                        pressed_button: None,
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
    notes: HashMap<NoteId, Note>,
    note_order: Vec<NoteId>,
    active_field: Option<FieldId>,
    next_note_id: NoteId,
    pointer_interaction: Option<PointerInteraction>,
    pressed_button: Option<ButtonId>,
}

#[derive(Clone, PartialEq, Eq)]
enum ButtonId {
    NewNote,
    NoteButtonId {
        note_id: NoteId,
        button_id: note::ButtonId,
    },
}

enum PointerInteraction {
    Drag(PointerInteractionState),
    Resize(PointerInteractionState),
}

struct PointerInteractionState {
    note_id: NoteId,
    last_x: f32,
    last_y: f32,
}

#[derive(Clone, Copy)]
struct ActiveField {
    note_id: NoteId,
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
        self.pressed_button = Some(ButtonId::NewNote);
        cx.notify();
    }

    fn clicked_new_note_button(
        &mut self,
        _: &MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pressed_button = None;
        let offset = self.note_order.len() as f32 * 24.0;
        let note_id = self.next_note_id;
        self.next_note_id = NoteId(self.next_note_id.0 + 1);
        let body_field_id = FieldId(format!("note-{}/body", note_id.0));
        self.notes.insert(
            note_id,
            Note::new(note_id, self.note_order.len() + 1, offset),
        );
        self.note_order.push(note_id);
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
        self.pressed_button = None;
        cx.notify();
    }

    fn active_field(&self) -> Option<ActiveField> {
        let active_field_id = self.active_field.as_ref()?;
        self.notes.iter().find_map(|(note_id, note)| {
            if &note.body_field_id() == active_field_id {
                Some(ActiveField {
                    note_id: *note_id,
                    kind: FieldKind::Body,
                })
            } else if &note.name_field_id() == active_field_id {
                Some(ActiveField {
                    note_id: *note_id,
                    kind: FieldKind::Name,
                })
            } else {
                None
            }
        })
    }

    fn pointer_note_mut(&mut self, note_id: NoteId, cx: &mut Context<Self>) -> Option<&mut Note> {
        if !self.notes.contains_key(&note_id) {
            self.pointer_interaction = None;
            cx.notify();
            return None;
        }

        self.notes.get_mut(&note_id)
    }

    fn bring_note_to_front(&mut self, note_id: NoteId) -> Option<NoteId> {
        if !self.notes.contains_key(&note_id) {
            return None;
        }

        if let Some(order_index) = self.note_order.iter().position(|id| *id == note_id) {
            self.note_order.remove(order_index);
        }
        self.note_order.push(note_id);
        Some(note_id)
    }

    fn activate_note_body(&mut self, note_id: NoteId) -> Option<NoteId> {
        let front_note_id = self.bring_note_to_front(note_id)?;
        self.active_field = Some(self.notes.get(&front_note_id)?.body_field_id());
        Some(front_note_id)
    }

    fn activate_note_name(&mut self, note_id: NoteId) -> Option<NoteId> {
        let front_note_id = self.bring_note_to_front(note_id)?;
        self.active_field = Some(self.notes.get(&front_note_id)?.name_field_id());
        Some(front_note_id)
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

                let Some(note) = self.pointer_note_mut(state.note_id, cx) else {
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

                let Some(note) = self.pointer_note_mut(state.note_id, cx) else {
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
                if let Some(note) = self.notes.get_mut(&active_field.note_id) {
                    note.pressed_name_backspace();
                }
                cx.notify();
            }
            note::KeyPress::OptionBackspace => {
                if let Some(note) = self.notes.get_mut(&active_field.note_id) {
                    note.pressed_name_option_backspace();
                }
                cx.notify();
            }
            note::KeyPress::CommandBackspace => {
                if let Some(note) = self.notes.get_mut(&active_field.note_id) {
                    note.pressed_name_command_backspace();
                }
                cx.notify();
            }
            note::KeyPress::Enter => {
                let body_field_id = if let Some(note) = self.notes.get_mut(&active_field.note_id) {
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
                if let Some(note) = self.notes.get_mut(&active_field.note_id) {
                    note.pressed_name_key(key_char);
                }
                cx.notify();
            }
        }
    }

    fn handled_note_event(&mut self, note_event: &note::IdEvent, cx: &mut Context<Self>) {
        let note_id = note_event.note_id;

        match &note_event.event {
            note::Event::PressedHeader { x, y } => {
                let Some(front_note_id) = self.activate_note_body(note_id) else {
                    return;
                };
                self.pointer_interaction =
                    Some(PointerInteraction::Drag(PointerInteractionState {
                        note_id: front_note_id,
                        last_x: *x,
                        last_y: *y,
                    }));
                cx.notify();
            }
            note::Event::PressedResizeHandle { x, y } => {
                let Some(front_note_id) = self.activate_note_body(note_id) else {
                    return;
                };
                self.pointer_interaction =
                    Some(PointerInteraction::Resize(PointerInteractionState {
                        note_id: front_note_id,
                        last_x: *x,
                        last_y: *y,
                    }));
                cx.notify();
            }
            note::Event::PressedBodyEditor => {
                self.activate_note_body(note_id);
                cx.notify();
            }
            note::Event::PressedNameEditor => {
                self.activate_note_name(note_id);
                cx.notify();
            }
            note::Event::ClickedRename => {
                let Some(front_note_id) = self.activate_note_name(note_id) else {
                    return;
                };

                if let Some(note) = self.notes.get_mut(&front_note_id) {
                    note.clicked_rename();
                    self.active_field = Some(note.name_field_id());
                }
                cx.notify();
            }
            note::Event::ClickedSaveName => {
                let Some(front_note_id) = self.bring_note_to_front(note_id) else {
                    return;
                };
                if let Some(note) = self.notes.get_mut(&front_note_id) {
                    note.clicked_save_name();
                    if self.active_field == Some(note.name_field_id()) {
                        self.active_field = Some(note.body_field_id());
                    }
                }
                cx.notify();
            }
            note::Event::ClickedCloseButton => {
                let Some(front_note_id) = self.bring_note_to_front(note_id) else {
                    return;
                };

                if let Some(closed_note) = self.notes.get(&front_note_id) {
                    self.pressed_button = None;
                    if self.active_field == Some(closed_note.name_field_id())
                        || self.active_field == Some(closed_note.body_field_id())
                    {
                        self.active_field = None;
                    }
                    self.notes.remove(&front_note_id);
                    self.note_order
                        .retain(|ordered_note_id| *ordered_note_id != front_note_id);
                    self.pointer_interaction = None;
                    cx.notify();
                }
            }
            note::Event::PressedButton { button_id } => {
                let Some(front_note_id) = self.bring_note_to_front(note_id) else {
                    return;
                };
                self.pressed_button = Some(ButtonId::NoteButtonId {
                    note_id: front_note_id,
                    button_id: button_id.clone(),
                });
                cx.notify();
            }
            note::Event::ReleasedButton => {
                self.pressed_button = None;
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

                let Some(note) = self.notes.get_mut(&active_field.note_id) else {
                    self.active_field = None;
                    return;
                };

                match key_press {
                    note::KeyPress::Backspace => {
                        note.pressed_body_backspace();
                        cx.notify();
                    }
                    note::KeyPress::OptionBackspace => {
                        note.pressed_body_option_backspace();
                        cx.notify();
                    }
                    note::KeyPress::CommandBackspace => {
                        note.pressed_body_command_backspace();
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

impl EventEmitter<note::IdEvent> for Model {}

impl Focusable for Model {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Model {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_focused = self.focus_handle.is_focused(window);
        let note_windows = self
            .note_order
            .iter()
            .filter_map(|note_id| {
                let note = self.notes.get(note_id)?;
                let pressed_note_button = match self.pressed_button.as_ref() {
                    Some(ButtonId::NoteButtonId {
                        note_id: pressed_note_id,
                        button_id,
                    }) if pressed_note_id == note_id => Some(button_id),
                    _ => None,
                };
                let show_body_cursor =
                    self.active_field == Some(note.body_field_id()) && is_focused;
                let show_name_cursor =
                    self.active_field == Some(note.name_field_id()) && is_focused;
                Some(note::render(
                    note,
                    &self.focus_handle,
                    pressed_note_button,
                    show_body_cursor,
                    show_name_cursor,
                    cx,
                ))
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
            .child(toolbar(self.pressed_button == Some(ButtonId::NewNote), cx))
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
