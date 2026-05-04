use gpui::{
    prelude::*, Context, EventEmitter, FocusHandle, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseUpEvent,
};
use serde::{Deserialize, Serialize};
use std::{io, path::PathBuf};

use crate::ui::{field::FieldId, style as s, view};

pub const DEFAULT_SIZE: f32 = 256.0;
pub const MIN_SIZE: f32 = 128.0;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteId(pub u64);

pub struct Model {
    pub id: NoteId,
    pub name: String,
    pub renaming: RenamingState,
    save_state: SaveState,
    edit_generation: u64,
    pub content: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Deserialize, Serialize)]
pub struct Storage {
    pub name: String,
    pub content: String,
}

enum InitFlags {
    New {
        ordinal: usize,
    },
    #[allow(dead_code)]
    FromStorage(Storage),
}

enum SaveState {
    Idle,
    Saving,
    Saved,
    Failed,
}

pub enum RenamingState {
    NotRenaming,
    Renaming { name_field: String },
}

#[derive(Clone, PartialEq, Eq)]
pub enum ButtonId {
    Save,
    X,
}

#[derive(Clone)]
pub enum Event {
    PressedHeader { x: f32, y: f32 },
    PressedResizeHandle { x: f32, y: f32 },
    PressedBodyEditor,
    PressedNameEditor,
    ClickedRename,
    ClickedSaveName,
    ClickedSaveButton,
    ClickedCloseButton,
    PressedButton { button_id: ButtonId },
    ReleasedButton,
    PressedKey(KeyPress),
}

#[derive(Clone)]
pub struct IdEvent {
    pub note_id: NoteId,
    pub event: Event,
}

pub struct SaveRequest {
    pub note_id: NoteId,
    pub generation: u64,
    storage: Storage,
}

#[derive(Clone)]
pub enum KeyPress {
    Backspace,
    OptionBackspace,
    CommandBackspace,
    Enter,
    Text(String),
}

impl Model {
    pub fn new(id: NoteId, ordinal: usize, offset: f32) -> Self {
        Self::initialize(id, offset, InitFlags::New { ordinal })
    }

    #[allow(dead_code)]
    pub fn from_storage(id: NoteId, storage: Storage, offset: f32) -> Self {
        Self::initialize(id, offset, InitFlags::FromStorage(storage))
    }

    fn initialize(id: NoteId, offset: f32, init_flags: InitFlags) -> Self {
        let name = match &init_flags {
            InitFlags::New { ordinal } => format!("note {ordinal}"),
            InitFlags::FromStorage(storage) => storage.name.clone(),
        };
        let content = match init_flags {
            InitFlags::New { .. } => String::new(),
            InitFlags::FromStorage(storage) => storage.content,
        };

        Self {
            id,
            name,
            renaming: RenamingState::NotRenaming,
            save_state: SaveState::Idle,
            edit_generation: 0,
            content,
            x: 32.0 + offset,
            y: 32.0 + offset,
            width: DEFAULT_SIZE,
            height: DEFAULT_SIZE,
        }
    }

    pub fn to_storage(&self) -> Storage {
        Storage {
            name: self.name.clone(),
            content: self.content.clone(),
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
            self.started_editing();
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
        self.started_editing();
    }

    pub fn pressed_body_option_backspace(&mut self) {
        delete_previous_word(&mut self.content);
        self.started_editing();
    }

    pub fn pressed_body_command_backspace(&mut self) {
        delete_current_line(&mut self.content);
        self.started_editing();
    }

    pub fn pressed_name_key(&mut self, key_char: &str) {
        if let RenamingState::Renaming { name_field } = &mut self.renaming {
            name_field.push_str(key_char);
        }
    }

    pub fn save_note(&mut self) -> SaveRequest {
        self.save_state = SaveState::Saving;
        SaveRequest {
            note_id: self.id,
            generation: self.edit_generation,
            storage: self.to_storage(),
        }
    }

    pub fn finished_saving(&mut self, generation: u64, result: Result<(), String>) {
        if generation != self.edit_generation {
            return;
        }

        self.save_state = match result {
            Ok(()) => SaveState::Saved,
            Err(_) => SaveState::Failed,
        };
    }

    pub fn started_editing(&mut self) {
        self.edit_generation += 1;
        self.save_state = SaveState::Idle;
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

pub fn save_note_file(save_request: SaveRequest) -> io::Result<PathBuf> {
    let notes_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("notes");
    std::fs::create_dir_all(&notes_dir)?;

    let file_slug = note_name_slug(&save_request.storage.name).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "note name must contain at least one letter or number",
        )
    })?;
    let path = notes_dir.join(format!("{file_slug}.json"));
    let contents = serde_json::to_string_pretty(&save_request.storage).map_err(io::Error::other)?;
    std::fs::write(&path, contents)?;

    Ok(path)
}

fn note_name_slug(note_name: &str) -> Option<String> {
    let mut slug = String::new();
    let mut previous_was_separator = false;

    for character in note_name.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            previous_was_separator = false;
        } else if !slug.is_empty() && !previous_was_separator {
            slug.push('-');
            previous_was_separator = true;
        }
    }

    if previous_was_separator {
        slug.pop();
    }

    if slug.is_empty() {
        None
    } else {
        Some(slug)
    }
}

pub fn render<T>(
    note: &Model,
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
    let save_button_pressed = pressed_button == Some(&ButtonId::Save);

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
            .child(save_row(emitter, note, save_button_pressed, cx)),
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

fn save_row<T>(
    emitter: IdEmitter,
    note: &Model,
    pressed: bool,
    cx: &mut Context<T>,
) -> impl IntoElement
where
    T: EventEmitter<IdEvent>,
{
    let status = match note.save_state {
        SaveState::Idle => "",
        SaveState::Saving => "saving...",
        SaveState::Saved => "saved",
        SaveState::Failed => "save failed",
    };

    gpui::div()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .p(s::S3)
        .pt(s::S0)
        .child(
            gpui::div()
                .flex_1()
                .min_h(s::S6)
                .text_color(s::YELLOW3)
                .child(status),
        )
        .child(
            view::button::from_text("save", pressed)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |_, _: &MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        emitter.emit(
                            cx,
                            Event::PressedButton {
                                button_id: ButtonId::Save,
                            },
                        );
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(move |_, _: &MouseUpEvent, _, cx| {
                        cx.stop_propagation();
                        emitter.emit(cx, Event::ClickedSaveButton);
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
    note: &Model,
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
