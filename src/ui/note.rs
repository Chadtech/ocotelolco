use gpui::{
    prelude::*, Context, EventEmitter, FocusHandle, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseUpEvent,
};

use crate::ui::{field::FieldId, style as s, view};

pub const DEFAULT_SIZE: f32 = 256.0;
pub const MIN_SIZE: f32 = 128.0;

pub struct Note {
    pub id: u64,
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

pub enum Event {
    PressedHeader { note_index: usize, x: f32, y: f32 },
    PressedResizeHandle { note_index: usize, x: f32, y: f32 },
    PressedBodyEditor { note_index: usize },
    PressedNameEditor { note_index: usize },
    ClickedRename { note_index: usize },
    ClickedSaveName { note_index: usize },
    PressedCloseButton { note_index: usize },
    ClickedCloseButton { note_index: usize },
    ReleasedCloseButtonOutside,
    PressedKey(KeyPress),
}

pub enum KeyPress {
    Backspace,
    Enter,
    Text(String),
}

impl Note {
    pub fn new(id: u64, ordinal: usize, offset: f32) -> Self {
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
        FieldId(format!("note-{}/name", self.id))
    }

    pub fn body_field_id(&self) -> FieldId {
        FieldId(format!("note-{}/body", self.id))
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

    pub fn pressed_name_key(&mut self, key_char: &str) {
        if let RenamingState::Renaming { name_field } = &mut self.renaming {
            name_field.push_str(key_char);
        }
    }
}

pub fn render<T>(
    note_index: usize,
    note: &Note,
    focus_handle: &FocusHandle,
    close_button_pressed: bool,
    show_cursor: bool,
    show_name_cursor: bool,
    cx: &mut Context<T>,
) -> gpui::Div
where
    T: EventEmitter<Event>,
{
    let mut lines = note.content.split('\n').collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push("");
    }
    let header_focus_handle = focus_handle.clone();
    let body_focus_handle = focus_handle.clone();

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
                            cx.emit(Event::PressedHeader {
                                note_index,
                                x: event.position.x.into(),
                                y: event.position.y.into(),
                            });
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
                                    cx.listener(move |_, _: &MouseDownEvent, window, cx| {
                                        window.focus(&body_focus_handle);
                                        cx.emit(Event::PressedBodyEditor { note_index });
                                    }),
                                )
                                .on_key_down(cx.listener(|_, event, _, cx| {
                                    emitted_key_event(event, cx);
                                }))
                                .children(render_lines(lines, show_cursor)),
                        )
                        .size_full(),
                    ),
            ),
    )
    .child(resize_handle(note_index, focus_handle, cx))
    .absolute()
    .left(gpui::px(note.x))
    .top(gpui::px(note.y))
    .w(gpui::px(note.width))
    .h(gpui::px(note.height))
}

fn rename_row<T>(
    note_index: usize,
    note: &Note,
    focus_handle: &FocusHandle,
    show_name_cursor: bool,
    cx: &mut Context<T>,
) -> impl IntoElement
where
    T: EventEmitter<Event>,
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
                                    cx.emit(Event::PressedNameEditor { note_index });
                                }),
                            )
                            .on_key_down(cx.listener(|_, event, _, cx| {
                                emitted_key_event(event, cx);
                            }))
                            .child(name_field_with_cursor),
                    )
                    .flex_1(),
                )
                .child(view::button::from_text("save name", false).on_mouse_up(
                    MouseButton::Left,
                    cx.listener(move |_, _: &MouseUpEvent, _, cx| {
                        cx.stop_propagation();
                        cx.emit(Event::ClickedSaveName { note_index });
                    }),
                ))
        }
        RenamingState::NotRenaming => gpui::div().flex().items_center().size_full().child(
            view::button::from_text("rename", false).on_mouse_up(
                MouseButton::Left,
                cx.listener(move |_, _: &MouseUpEvent, window, cx| {
                    window.focus(&rename_button_focus_handle);
                    cx.stop_propagation();
                    cx.emit(Event::ClickedRename { note_index });
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

fn close_button<T>(note_index: usize, pressed: bool, cx: &mut Context<T>) -> gpui::Div
where
    T: EventEmitter<Event>,
{
    view::button::x(pressed)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |_, _: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
                cx.emit(Event::PressedCloseButton { note_index });
            }),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(move |_, _: &MouseUpEvent, _, cx| {
                cx.stop_propagation();
                cx.emit(Event::ClickedCloseButton { note_index });
            }),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(|_, _: &MouseUpEvent, _, cx| {
                cx.emit(Event::ReleasedCloseButtonOutside);
            }),
        )
}

fn resize_handle<T>(
    note_index: usize,
    focus_handle: &FocusHandle,
    cx: &mut Context<T>,
) -> impl IntoElement
where
    T: EventEmitter<Event>,
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
                cx.emit(Event::PressedResizeHandle {
                    note_index,
                    x: event.position.x.into(),
                    y: event.position.y.into(),
                });
            }),
        )
}

fn emitted_key_event<T>(event: &KeyDownEvent, cx: &mut Context<T>)
where
    T: EventEmitter<Event>,
{
    if event.keystroke.modifiers.platform || event.keystroke.modifiers.control {
        return;
    }

    let key_press = match event.keystroke.key.as_str() {
        "backspace" => KeyPress::Backspace,
        "enter" => KeyPress::Enter,
        _ => {
            let Some(key_char) = event.keystroke.key_char.as_ref() else {
                return;
            };
            KeyPress::Text(key_char.clone())
        }
    };

    cx.stop_propagation();
    cx.emit(Event::PressedKey(key_press));
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
