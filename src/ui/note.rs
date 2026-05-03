use gpui::{
    prelude::*, Context, EventEmitter, FocusHandle, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseUpEvent,
};

use crate::ui::{field::FieldId, style as s, view};

pub const DEFAULT_SIZE: f32 = 256.0;
pub const MIN_SIZE: f32 = 128.0;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteId(pub u64);

pub struct Note {
    pub id: NoteId,
    pub name: String,
    pub renaming: RenamingState,
    pub content: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub enum RenamingState {
    NotRenaming,
    Renaming { name_field: String },
}

#[derive(Clone, PartialEq, Eq)]
pub enum ButtonId {
    SaveName,
    X,
}

pub enum Event {
    PressedHeader { x: f32, y: f32 },
    PressedResizeHandle { x: f32, y: f32 },
    PressedBodyEditor,
    PressedNameEditor,
    ClickedRename,
    ClickedSaveName,
    ClickedCloseButton,
    PressedButton { button_id: ButtonId },
    ReleasedButton,
    PressedKey(KeyPress),
}

pub struct IdEvent {
    pub note_id: NoteId,
    pub event: Event,
}

pub enum KeyPress {
    Backspace,
    OptionBackspace,
    CommandBackspace,
    Enter,
    Text(String),
}

impl Note {
    pub fn new(id: NoteId, ordinal: usize, offset: f32) -> Self {
        Self {
            id,
            name: format!("note {ordinal}"),
            renaming: RenamingState::NotRenaming,
            content: String::new(),
            x: 32.0 + offset,
            y: 32.0 + offset,
            width: DEFAULT_SIZE,
            height: DEFAULT_SIZE,
        }
    }

    pub fn name_field_id(&self) -> FieldId {
        FieldId(format!("note-{}/name", self.id.0))
    }

    pub fn body_field_id(&self) -> FieldId {
        FieldId(format!("note-{}/body", self.id.0))
    }

    pub fn clicked_rename(&mut self) {
        self.renaming = RenamingState::Renaming {
            name_field: self.name.clone(),
        };
    }

    pub fn clicked_save_name(&mut self) {
        if let RenamingState::Renaming { name_field } = &self.renaming {
            self.name = name_field.clone();
        }
        self.renaming = RenamingState::NotRenaming;
    }

    pub fn pressed_name_backspace(&mut self) {
        if let RenamingState::Renaming { name_field } = &mut self.renaming {
            name_field.pop();
        }
    }

    pub fn pressed_name_option_backspace(&mut self) {
        if let RenamingState::Renaming { name_field } = &mut self.renaming {
            delete_previous_word(name_field);
        }
    }

    pub fn pressed_name_command_backspace(&mut self) {
        if let RenamingState::Renaming { name_field } = &mut self.renaming {
            delete_current_line(name_field);
        }
    }

    pub fn pressed_body_backspace(&mut self) {
        self.content.pop();
    }

    pub fn pressed_body_option_backspace(&mut self) {
        delete_previous_word(&mut self.content);
    }

    pub fn pressed_body_command_backspace(&mut self) {
        delete_current_line(&mut self.content);
    }

    pub fn pressed_name_key(&mut self, key_char: &str) {
        if let RenamingState::Renaming { name_field } = &mut self.renaming {
            name_field.push_str(key_char);
        }
    }
}

fn delete_previous_word(text: &mut String) {
    if text.is_empty() {
        return;
    }

    let mut after_trailing_whitespace = text.len();
    for (index, character) in text.char_indices().rev() {
        if character.is_whitespace() {
            after_trailing_whitespace = index;
        } else {
            break;
        }
    }
    text.truncate(after_trailing_whitespace);

    let mut before_word = 0;
    for (index, character) in text.char_indices().rev() {
        if character.is_whitespace() {
            before_word = index + character.len_utf8();
            break;
        }
    }
    text.truncate(before_word);
}

fn delete_current_line(text: &mut String) {
    if let Some(text_before_empty_line) = text.strip_suffix('\n') {
        let previous_line_start = text_before_empty_line
            .rfind('\n')
            .map_or(0, |index| index + '\n'.len_utf8());
        text.truncate(previous_line_start);
        return;
    }

    let line_start = text.rfind('\n').map_or(0, |index| index + '\n'.len_utf8());
    text.truncate(line_start);
}

pub fn render<T>(
    note: &Note,
    focus_handle: &FocusHandle,
    pressed_button: Option<&ButtonId>,
    show_cursor: bool,
    show_name_cursor: bool,
    cx: &mut Context<T>,
) -> gpui::Div
where
    T: EventEmitter<IdEvent>,
{
    let emitter = IdEmitter { note_id: note.id };
    let mut lines = note.content.split('\n').collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push("");
    }
    let header_focus_handle = focus_handle.clone();
    let body_focus_handle = focus_handle.clone();
    let close_button_pressed = pressed_button == Some(&ButtonId::X);
    let save_button_pressed = pressed_button == Some(&ButtonId::SaveName);

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
                        cx.listener(move |_, event: &MouseDownEvent, window, cx| {
                            window.focus(&header_focus_handle);
                            cx.stop_propagation();
                            emitter.emit(
                                cx,
                                Event::PressedHeader {
                                    x: event.position.x.into(),
                                    y: event.position.y.into(),
                                },
                            );
                        }),
                    )
                    .child(note.name.clone())
                    .child(close_button(emitter, close_button_pressed, cx)),
            )
            .child(rename_row(
                emitter,
                note,
                focus_handle,
                show_name_cursor,
                cx,
            ))
            .child(
                gpui::div().p(s::S3).pt(s::S0).flex_1().bg(s::GRAY2).child(
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
                                cx.listener(move |_, _: &MouseDownEvent, window, cx| {
                                    window.focus(&body_focus_handle);
                                    emitter.emit(cx, Event::PressedBodyEditor);
                                }),
                            )
                            .on_key_down(cx.listener(move |_, event, _, cx| {
                                emitted_key_event(emitter, event, cx);
                            }))
                            .children(render_lines(lines, show_cursor)),
                    )
                    .size_full(),
                ),
            )
            .child(save_row(emitter, save_button_pressed, cx)),
    )
    .child(resize_handle(emitter, focus_handle, cx))
    .absolute()
    .left(gpui::px(note.x))
    .top(gpui::px(note.y))
    .w(gpui::px(note.width))
    .h(gpui::px(note.height))
}

#[derive(Clone, Copy)]
struct IdEmitter {
    note_id: NoteId,
}

impl IdEmitter {
    fn emit<T>(self, cx: &mut Context<T>, event: Event)
    where
        T: EventEmitter<IdEvent>,
    {
        cx.emit(IdEvent {
            note_id: self.note_id,
            event,
        });
    }
}

fn save_row<T>(emitter: IdEmitter, pressed: bool, cx: &mut Context<T>) -> impl IntoElement
where
    T: EventEmitter<IdEvent>,
{
    gpui::div().flex().justify_end().p(s::S3).pt(s::S0).child(
        view::button::from_text("save", pressed)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |_, _: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    emitter.emit(
                        cx,
                        Event::PressedButton {
                            button_id: ButtonId::SaveName,
                        },
                    );
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |_, _: &MouseUpEvent, _, cx| {
                    cx.stop_propagation();
                    emitter.emit(cx, Event::ReleasedButton);
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(move |_, _: &MouseUpEvent, _, cx| {
                    emitter.emit(cx, Event::ReleasedButton);
                }),
            ),
    )
}

fn rename_row<T>(
    emitter: IdEmitter,
    note: &Note,
    focus_handle: &FocusHandle,
    show_name_cursor: bool,
    cx: &mut Context<T>,
) -> impl IntoElement
where
    T: EventEmitter<IdEvent>,
{
    let name_focus_handle = focus_handle.clone();
    let rename_button_focus_handle = focus_handle.clone();
    let rename_control = match &note.renaming {
        RenamingState::Renaming { name_field } => {
            let name_field_with_cursor = if show_name_cursor {
                format!("{}|", name_field)
            } else {
                name_field.clone()
            };

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
                                cx.listener(move |_, _: &MouseDownEvent, window, cx| {
                                    window.focus(&name_focus_handle);
                                    cx.stop_propagation();
                                    emitter.emit(cx, Event::PressedNameEditor);
                                }),
                            )
                            .on_key_down(cx.listener(move |_, event, _, cx| {
                                emitted_key_event(emitter, event, cx);
                            }))
                            .child(name_field_with_cursor),
                    )
                    .flex_1(),
                )
                .child(view::button::from_text("save name", false).on_mouse_up(
                    MouseButton::Left,
                    cx.listener(move |_, _: &MouseUpEvent, _, cx| {
                        cx.stop_propagation();
                        emitter.emit(cx, Event::ClickedSaveName);
                    }),
                ))
        }
        RenamingState::NotRenaming => gpui::div().flex().items_center().size_full().child(
            view::button::from_text("rename", false).on_mouse_up(
                MouseButton::Left,
                cx.listener(move |_, _: &MouseUpEvent, window, cx| {
                    window.focus(&rename_button_focus_handle);
                    cx.stop_propagation();
                    emitter.emit(cx, Event::ClickedRename);
                }),
            ),
        ),
    };

    gpui::div()
        .flex()
        .items_center()
        .bg(s::GRAY2)
        .p(s::S3)
        .child(rename_control)
}

fn close_button<T>(emitter: IdEmitter, pressed: bool, cx: &mut Context<T>) -> gpui::Div
where
    T: EventEmitter<IdEvent>,
{
    view::button::x(pressed)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |_, _: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
                emitter.emit(
                    cx,
                    Event::PressedButton {
                        button_id: ButtonId::X,
                    },
                );
            }),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(move |_, _: &MouseUpEvent, _, cx| {
                cx.stop_propagation();
                emitter.emit(cx, Event::ClickedCloseButton);
            }),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(move |_, _: &MouseUpEvent, _, cx| {
                emitter.emit(cx, Event::ReleasedButton);
            }),
        )
}

fn resize_handle<T>(
    emitter: IdEmitter,
    focus_handle: &FocusHandle,
    cx: &mut Context<T>,
) -> impl IntoElement
where
    T: EventEmitter<IdEvent>,
{
    let focus_handle = focus_handle.clone();
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
            cx.listener(move |_, event: &MouseDownEvent, window, cx| {
                window.focus(&focus_handle);
                cx.stop_propagation();
                emitter.emit(
                    cx,
                    Event::PressedResizeHandle {
                        x: event.position.x.into(),
                        y: event.position.y.into(),
                    },
                );
            }),
        )
}

fn emitted_key_event<T>(emitter: IdEmitter, event: &KeyDownEvent, cx: &mut Context<T>)
where
    T: EventEmitter<IdEvent>,
{
    let key_press = match event.keystroke.key.as_str() {
        "backspace" if event.keystroke.modifiers.platform => KeyPress::CommandBackspace,
        "backspace" if event.keystroke.modifiers.alt => KeyPress::OptionBackspace,
        "backspace" => KeyPress::Backspace,
        "enter" => KeyPress::Enter,
        _ => {
            if event.keystroke.modifiers.platform || event.keystroke.modifiers.control {
                return;
            }

            let Some(key_char) = event.keystroke.key_char.as_ref() else {
                return;
            };
            KeyPress::Text(key_char.clone())
        }
    };

    cx.stop_propagation();
    emitter.emit(cx, Event::PressedKey(key_press));
}
fn render_lines(lines: Vec<&str>, is_focused: bool) -> Vec<impl IntoElement> {
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
