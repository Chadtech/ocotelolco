use gpui::{
    prelude::*, App, Application, Context, EventEmitter, FocusHandle, Focusable, MouseButton,
    MouseDownEvent, MouseMoveEvent, Render, WindowOptions,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
};

pub mod field;
mod note;
mod spreadsheet;
pub mod style;
pub mod view;

use crate::ui::field::FieldId;
use crate::ui::style as s;
use note::NoteId;
use spreadsheet::SpreadsheetId;

const DEFAULT_WINDOW_SIZE: f32 = 256.0;
const MIN_WINDOW_SIZE: f32 = 128.0;

pub fn run() {
    Application::new().run(|cx: &mut App| {
        let window_handle = cx
            .open_window(WindowOptions::default(), |window, cx| {
                window.set_window_title("Ocotelolco Notes");
                let focus_handle = cx.focus_handle();

                cx.new(|cx| {
                    cx.subscribe_self(Model::handled_note_event).detach();
                    cx.subscribe_self(Model::handled_spreadsheet_event).detach();

                    let mut model = Model {
                        focus_handle,
                        state: LoadingState::Loading,
                    };
                    model.load(cx);
                    model
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

fn clipboard_text(cx: &mut Context<Model>) -> Option<String> {
    cx.read_from_clipboard().and_then(|item| item.text())
}

struct Model {
    focus_handle: FocusHandle,
    state: LoadingState,
}

enum LoadingState {
    Loading,
    Loaded(LoadedState),
}

struct LoadedState {
    windows: HashMap<WindowId, Window>,
    window_order: Vec<WindowId>,
    active_field: Option<ActiveFieldId>,
    next_note_id: NoteId,
    next_spreadsheet_id: SpreadsheetId,
    pointer_interaction: Option<PointerInteraction>,
    pressed_button: Option<ButtonId>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Storage {
    windows: Vec<StorageWindow>,
}

#[derive(Clone, Deserialize, Serialize)]
struct StorageWindow {
    x: f32,
    y: f32,
    height: f32,
    width: f32,
    content: StorageWindowContent,
}

#[derive(Clone, Deserialize, Serialize)]
enum StorageWindowContent {
    Note(note::StorageState),
    Spreadsheet(spreadsheet::StorageState),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum WindowId {
    Note(NoteId),
    Spreadsheet(SpreadsheetId),
    NotePicker,
    SpreadsheetPicker,
}

struct Window {
    x: f32,
    y: f32,
    height: f32,
    width: f32,
    content: WindowContent,
}

enum WindowContent {
    Note(note::Model),
    Spreadsheet(spreadsheet::Model),
    NotePicker { paths: Vec<PathBuf> },
    SpreadsheetPicker { paths: Vec<PathBuf> },
}

enum WindowContentError {
    ExpectedNote,
    ExpectedSpreadsheet,
}

impl Window {
    fn new_note(note_id: NoteId, ordinal: usize, offset: f32) -> Self {
        Self {
            x: 32.0 + offset,
            y: 32.0 + offset,
            height: DEFAULT_WINDOW_SIZE,
            width: DEFAULT_WINDOW_SIZE,
            content: WindowContent::Note(note::Model::new(note_id, ordinal)),
        }
    }

    fn new_spreadsheet(spreadsheet_id: SpreadsheetId, ordinal: usize, offset: f32) -> Self {
        Self {
            x: 48.0 + offset,
            y: 48.0 + offset,
            height: DEFAULT_WINDOW_SIZE,
            width: DEFAULT_WINDOW_SIZE * 1.5,
            content: WindowContent::Spreadsheet(spreadsheet::Model::new(spreadsheet_id, ordinal)),
        }
    }

    fn new_note_picker(paths: Vec<PathBuf>, offset: f32) -> Self {
        Self {
            x: 64.0 + offset,
            y: 64.0 + offset,
            height: DEFAULT_WINDOW_SIZE,
            width: 320.0,
            content: WindowContent::NotePicker { paths },
        }
    }

    fn new_spreadsheet_picker(paths: Vec<PathBuf>, offset: f32) -> Self {
        Self {
            x: 64.0 + offset,
            y: 64.0 + offset,
            height: DEFAULT_WINDOW_SIZE,
            width: 320.0,
            content: WindowContent::SpreadsheetPicker { paths },
        }
    }

    fn from_storage(storage: StorageWindow, content: WindowContent) -> Self {
        Self {
            x: storage.x,
            y: storage.y,
            height: storage.height.max(MIN_WINDOW_SIZE),
            width: storage.width.max(MIN_WINDOW_SIZE),
            content,
        }
    }

    fn to_storage(&self) -> Option<StorageWindow> {
        let content = match &self.content {
            WindowContent::Note(note) => StorageWindowContent::Note(note.to_storage_state()),
            WindowContent::Spreadsheet(spreadsheet) => {
                StorageWindowContent::Spreadsheet(spreadsheet.to_storage_state())
            }
            WindowContent::NotePicker { .. } | WindowContent::SpreadsheetPicker { .. } => {
                return None;
            }
        };

        Some(StorageWindow {
            x: self.x,
            y: self.y,
            height: self.height,
            width: self.width,
            content,
        })
    }

    fn note(&self) -> Result<&note::Model, WindowContentError> {
        match &self.content {
            WindowContent::Note(note) => Ok(note),
            WindowContent::Spreadsheet(_)
            | WindowContent::NotePicker { .. }
            | WindowContent::SpreadsheetPicker { .. } => Err(WindowContentError::ExpectedNote),
        }
    }

    fn note_mut(&mut self) -> Result<&mut note::Model, WindowContentError> {
        match &mut self.content {
            WindowContent::Note(note) => Ok(note),
            WindowContent::Spreadsheet(_)
            | WindowContent::NotePicker { .. }
            | WindowContent::SpreadsheetPicker { .. } => Err(WindowContentError::ExpectedNote),
        }
    }

    fn spreadsheet(&self) -> Result<&spreadsheet::Model, WindowContentError> {
        match &self.content {
            WindowContent::Spreadsheet(spreadsheet) => Ok(spreadsheet),
            WindowContent::Note(_)
            | WindowContent::NotePicker { .. }
            | WindowContent::SpreadsheetPicker { .. } => {
                Err(WindowContentError::ExpectedSpreadsheet)
            }
        }
    }

    fn spreadsheet_mut(&mut self) -> Result<&mut spreadsheet::Model, WindowContentError> {
        match &mut self.content {
            WindowContent::Spreadsheet(spreadsheet) => Ok(spreadsheet),
            WindowContent::Note(_)
            | WindowContent::NotePicker { .. }
            | WindowContent::SpreadsheetPicker { .. } => {
                Err(WindowContentError::ExpectedSpreadsheet)
            }
        }
    }
}

impl WindowId {
    fn note_id(self) -> Option<NoteId> {
        match self {
            Self::Note(note_id) => Some(note_id),
            Self::Spreadsheet(_) | Self::NotePicker | Self::SpreadsheetPicker => None,
        }
    }

    fn spreadsheet_id(self) -> Option<SpreadsheetId> {
        match self {
            Self::Spreadsheet(spreadsheet_id) => Some(spreadsheet_id),
            Self::Note(_) | Self::NotePicker | Self::SpreadsheetPicker => None,
        }
    }
}

impl From<NoteId> for WindowId {
    fn from(note_id: NoteId) -> Self {
        Self::Note(note_id)
    }
}

impl From<SpreadsheetId> for WindowId {
    fn from(spreadsheet_id: SpreadsheetId) -> Self {
        Self::Spreadsheet(spreadsheet_id)
    }
}

#[derive(Clone, PartialEq, Eq)]
enum ButtonId {
    NewNote,
    NewSpreadsheet,
    OpenNotePicker,
    OpenSpreadsheetPicker,
    NoteButtonId {
        note_id: NoteId,
        button_id: note::ButtonId,
    },
    SpreadsheetButtonId {
        spreadsheet_id: SpreadsheetId,
        button_id: spreadsheet::ButtonId,
    },
}

enum Effect {
    SaveNote(note::SaveRequest),
    SaveSpreadsheet(spreadsheet::SaveRequest),
}

enum Event<'a> {
    Note(note::IdEvent),
    Spreadsheet(spreadsheet::IdEvent),
    PressedNewNoteButton,
    ClickedNewNoteButton,
    ReleasedNewNoteButtonOutside,
    PressedNewSpreadsheetButton,
    ClickedNewSpreadsheetButton,
    ReleasedNewSpreadsheetButtonOutside,
    PressedOpenNotePickerButton,
    ClickedOpenNotePickerButton,
    ReleasedOpenNotePickerButtonOutside,
    PressedOpenSpreadsheetPickerButton,
    ClickedOpenSpreadsheetPickerButton,
    ReleasedOpenSpreadsheetPickerButtonOutside,
    ClickedSavedNote(PathBuf),
    ClickedCloseNotePicker,
    PressedNotePickerHeader { x: f32, y: f32 },
    PressedNotePickerResizeHandle { x: f32, y: f32 },
    ClickedSavedSpreadsheet(PathBuf),
    ClickedCloseSpreadsheetPicker,
    PressedSpreadsheetPickerHeader { x: f32, y: f32 },
    PressedSpreadsheetPickerResizeHandle { x: f32, y: f32 },
    MovedMouse(&'a MouseMoveEvent),
    ReleasedMouse,
}

enum PointerInteraction {
    Drag(PointerInteractionState),
    Resize(PointerInteractionState),
    ColumnResize(ColumnResizeInteractionState),
}

struct PointerInteractionState {
    window_id: WindowId,
    last_x: f32,
    last_y: f32,
}

struct ColumnResizeInteractionState {
    spreadsheet_id: SpreadsheetId,
    column: usize,
    last_x: f32,
}

#[derive(Clone, PartialEq, Eq)]
enum ActiveFieldId {
    Note(FieldId),
    Spreadsheet(spreadsheet::CellFieldId),
    SpreadsheetName(SpreadsheetId),
}

#[derive(Clone, Copy)]
struct ActiveNoteField {
    note_id: NoteId,
    kind: NoteFieldKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NoteFieldKind {
    Body,
    Name,
}

impl Model {
    fn load(&mut self, cx: &mut Context<Model>) {
        let storage = match load_storage_file() {
            Ok(storage) => Some(storage),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => {
                eprintln!("failed to load ui storage: {error}");
                None
            }
        };

        self.state = LoadingState::Loaded(match storage {
            Some(storage) => LoadedState::from_storage(storage),
            None => LoadedState::default(),
        });
        cx.notify();
    }

    fn loaded_mut(&mut self) -> Option<&mut LoadedState> {
        match &mut self.state {
            LoadingState::Loaded(state) => Some(state),
            LoadingState::Loading => None,
        }
    }

    fn handle_event(&mut self, event: Event<'_>, cx: &mut Context<Model>) {
        let Some(state) = self.loaded_mut() else {
            return;
        };
        state.handle_event(event, cx);
    }

    fn handled_note_event(&mut self, note_event: &note::IdEvent, cx: &mut Context<Model>) {
        self.handle_event(Event::Note(note_event.clone()), cx);
    }

    fn handled_spreadsheet_event(
        &mut self,
        spreadsheet_event: &spreadsheet::IdEvent,
        cx: &mut Context<Model>,
    ) {
        self.handle_event(Event::Spreadsheet(spreadsheet_event.clone()), cx);
    }

    fn finish_saving_note(
        &mut self,
        note_id: NoteId,
        generation: u64,
        result: Result<PathBuf, String>,
        cx: &mut Context<Model>,
    ) {
        let Some(state) = self.loaded_mut() else {
            cx.notify();
            return;
        };
        let Some(note) = state
            .windows
            .get_mut(&WindowId::from(note_id))
            .and_then(|window| window.note_mut().ok())
        else {
            cx.notify();
            return;
        };

        if let Err(error) = &result {
            eprintln!("failed to save note {}: {error}", note_id.0);
        }
        note.finished_saving(generation, result);
        state.save_storage();
        state.refresh_note_picker();
        cx.notify();
    }

    fn finish_saving_spreadsheet(
        &mut self,
        spreadsheet_id: SpreadsheetId,
        generation: u64,
        result: Result<PathBuf, String>,
        cx: &mut Context<Model>,
    ) {
        let Some(state) = self.loaded_mut() else {
            cx.notify();
            return;
        };
        let Some(spreadsheet) = state
            .windows
            .get_mut(&WindowId::from(spreadsheet_id))
            .and_then(|window| window.spreadsheet_mut().ok())
        else {
            cx.notify();
            return;
        };

        if let Err(error) = &result {
            eprintln!("failed to save spreadsheet {}: {error}", spreadsheet_id.0);
        }
        spreadsheet.finished_saving(generation, result);
        state.save_storage();
        state.refresh_spreadsheet_picker();
        cx.notify();
    }
}

impl Default for LoadedState {
    fn default() -> Self {
        Self {
            windows: HashMap::new(),
            window_order: Vec::new(),
            active_field: None,
            next_note_id: NoteId(1),
            next_spreadsheet_id: SpreadsheetId(1),
            pointer_interaction: None,
            pressed_button: None,
        }
    }
}

impl LoadedState {
    fn from_storage(storage: Storage) -> Self {
        let mut state = Self::default();

        for storage_window in storage.windows.into_iter() {
            let content = match storage_window.content.clone() {
                StorageWindowContent::Note(note::StorageState::Saved { path }) => {
                    let note_id = state.next_note_id;
                    let Some(note_storage) = load_storage_window_note(&path) else {
                        continue;
                    };
                    state.next_note_id = NoteId(state.next_note_id.0 + 1);
                    WindowContent::Note(note::Model::from_storage(
                        note_id,
                        note_storage,
                        Some(path),
                    ))
                }
                StorageWindowContent::Note(note::StorageState::Unsaved(note_storage)) => {
                    let note_id = state.next_note_id;
                    state.next_note_id = NoteId(state.next_note_id.0 + 1);
                    WindowContent::Note(note::Model::from_storage(note_id, note_storage, None))
                }
                StorageWindowContent::Spreadsheet(spreadsheet::StorageState::Saved {
                    path,
                    column_widths,
                }) => {
                    let spreadsheet_id = state.next_spreadsheet_id;
                    let Some(mut spreadsheet_storage) = load_storage_window_spreadsheet(&path)
                    else {
                        continue;
                    };
                    spreadsheet_storage.column_widths = column_widths;
                    state.next_spreadsheet_id = SpreadsheetId(state.next_spreadsheet_id.0 + 1);
                    WindowContent::Spreadsheet(spreadsheet::Model::from_storage(
                        spreadsheet_id,
                        spreadsheet_storage,
                        Some(path),
                    ))
                }
                StorageWindowContent::Spreadsheet(spreadsheet::StorageState::Unsaved(
                    spreadsheet_storage,
                )) => {
                    let spreadsheet_id = state.next_spreadsheet_id;
                    state.next_spreadsheet_id = SpreadsheetId(state.next_spreadsheet_id.0 + 1);
                    WindowContent::Spreadsheet(spreadsheet::Model::from_storage(
                        spreadsheet_id,
                        spreadsheet_storage,
                        None,
                    ))
                }
            };
            let window_id = match &content {
                WindowContent::Note(note) => WindowId::from(note.id),
                WindowContent::Spreadsheet(spreadsheet) => WindowId::from(spreadsheet.id),
                WindowContent::NotePicker { .. } => WindowId::NotePicker,
                WindowContent::SpreadsheetPicker { .. } => WindowId::SpreadsheetPicker,
            };
            state.window_order.push(window_id);
            state
                .windows
                .insert(window_id, Window::from_storage(storage_window, content));
        }

        state
    }

    fn to_storage(&self) -> Storage {
        let windows = self
            .window_order
            .iter()
            .filter_map(|window_id| self.windows.get(window_id).and_then(Window::to_storage))
            .collect();

        Storage { windows }
    }

    fn save_storage(&self) {
        if let Err(error) = save_storage_file(&self.to_storage()) {
            eprintln!("failed to save ui storage: {error}");
        }
    }

    fn dispatch_effect(&mut self, effect: Effect, cx: &mut Context<Model>) {
        match effect {
            Effect::SaveNote(save_note) => {
                cx.spawn(async move |model, cx| {
                    let note_id = save_note.note_id;
                    let generation = save_note.generation;
                    let result = note::save_note_file(save_note).map_err(|error| error.to_string());
                    let _ = model.update(cx, |model, cx| {
                        model.finish_saving_note(note_id, generation, result, cx);
                    });
                })
                .detach();
            }
            Effect::SaveSpreadsheet(save_spreadsheet) => {
                cx.spawn(async move |model, cx| {
                    let spreadsheet_id = save_spreadsheet.spreadsheet_id;
                    let generation = save_spreadsheet.generation;
                    let result = spreadsheet::save_spreadsheet_file(save_spreadsheet)
                        .map_err(|error| error.to_string());
                    let _ = model.update(cx, |model, cx| {
                        model.finish_saving_spreadsheet(spreadsheet_id, generation, result, cx);
                    });
                })
                .detach();
            }
        }
    }

    fn handle_event(&mut self, event: Event<'_>, cx: &mut Context<Model>) {
        match event {
            Event::Note(note_event) => {
                self.update_note(note_event, cx);
            }
            Event::Spreadsheet(spreadsheet_event) => {
                self.update_spreadsheet(spreadsheet_event, cx);
            }
            Event::PressedNewNoteButton => {
                self.pressed_button = Some(ButtonId::NewNote);
                cx.notify();
            }
            Event::ClickedNewNoteButton => {
                self.pressed_button = None;
                let offset = self.window_order.len() as f32 * 24.0;
                let note_id = self.next_note_id;
                let window_id = WindowId::Note(note_id);
                self.next_note_id = NoteId(self.next_note_id.0 + 1);
                let body_field_id = FieldId(format!("note-{}/body", note_id.0));
                self.windows.insert(
                    window_id,
                    Window::new_note(note_id, self.window_order.len() + 1, offset),
                );
                self.window_order.push(window_id);
                self.active_field = Some(ActiveFieldId::Note(body_field_id));
                self.save_storage();
                cx.notify();
            }
            Event::ReleasedNewNoteButtonOutside => {
                self.pressed_button = None;
                cx.notify();
            }
            Event::PressedNewSpreadsheetButton => {
                self.pressed_button = Some(ButtonId::NewSpreadsheet);
                cx.notify();
            }
            Event::ClickedNewSpreadsheetButton => {
                self.pressed_button = None;
                let offset = self.window_order.len() as f32 * 24.0;
                let spreadsheet_id = self.next_spreadsheet_id;
                let window_id = WindowId::Spreadsheet(spreadsheet_id);
                self.next_spreadsheet_id = SpreadsheetId(self.next_spreadsheet_id.0 + 1);
                self.windows.insert(
                    window_id,
                    Window::new_spreadsheet(spreadsheet_id, self.window_order.len() + 1, offset),
                );
                self.window_order.push(window_id);
                self.active_field = Some(ActiveFieldId::Spreadsheet(
                    spreadsheet::CellFieldId::new(spreadsheet_id, 0, 0),
                ));
                self.save_storage();
                cx.notify();
            }
            Event::ReleasedNewSpreadsheetButtonOutside => {
                self.pressed_button = None;
                cx.notify();
            }
            Event::PressedOpenNotePickerButton => {
                self.pressed_button = Some(ButtonId::OpenNotePicker);
                cx.notify();
            }
            Event::ClickedOpenNotePickerButton => {
                self.pressed_button = None;
                self.toggle_note_picker();
                cx.notify();
            }
            Event::ReleasedOpenNotePickerButtonOutside => {
                self.pressed_button = None;
                cx.notify();
            }
            Event::PressedOpenSpreadsheetPickerButton => {
                self.pressed_button = Some(ButtonId::OpenSpreadsheetPicker);
                cx.notify();
            }
            Event::ClickedOpenSpreadsheetPickerButton => {
                self.pressed_button = None;
                self.toggle_spreadsheet_picker();
                cx.notify();
            }
            Event::ReleasedOpenSpreadsheetPickerButtonOutside => {
                self.pressed_button = None;
                cx.notify();
            }
            Event::ClickedSavedNote(path) => {
                self.open_saved_note(path);
                self.close_note_picker();
                cx.notify();
            }
            Event::ClickedCloseNotePicker => {
                self.close_note_picker();
                cx.notify();
            }
            Event::PressedNotePickerHeader { x, y } => {
                if self.bring_window_to_front(WindowId::NotePicker).is_none() {
                    return;
                }
                self.pointer_interaction =
                    Some(PointerInteraction::Drag(PointerInteractionState {
                        window_id: WindowId::NotePicker,
                        last_x: x,
                        last_y: y,
                    }));
                cx.notify();
            }
            Event::PressedNotePickerResizeHandle { x, y } => {
                if self.bring_window_to_front(WindowId::NotePicker).is_none() {
                    return;
                }
                self.pointer_interaction =
                    Some(PointerInteraction::Resize(PointerInteractionState {
                        window_id: WindowId::NotePicker,
                        last_x: x,
                        last_y: y,
                    }));
                cx.notify();
            }
            Event::ClickedSavedSpreadsheet(path) => {
                self.open_saved_spreadsheet(path);
                self.close_spreadsheet_picker();
                cx.notify();
            }
            Event::ClickedCloseSpreadsheetPicker => {
                self.close_spreadsheet_picker();
                cx.notify();
            }
            Event::PressedSpreadsheetPickerHeader { x, y } => {
                if self
                    .bring_window_to_front(WindowId::SpreadsheetPicker)
                    .is_none()
                {
                    return;
                }
                self.pointer_interaction =
                    Some(PointerInteraction::Drag(PointerInteractionState {
                        window_id: WindowId::SpreadsheetPicker,
                        last_x: x,
                        last_y: y,
                    }));
                cx.notify();
            }
            Event::PressedSpreadsheetPickerResizeHandle { x, y } => {
                if self
                    .bring_window_to_front(WindowId::SpreadsheetPicker)
                    .is_none()
                {
                    return;
                }
                self.pointer_interaction =
                    Some(PointerInteraction::Resize(PointerInteractionState {
                        window_id: WindowId::SpreadsheetPicker,
                        last_x: x,
                        last_y: y,
                    }));
                cx.notify();
            }
            Event::MovedMouse(event) => {
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

                        let Some(ui_window) = self.pointer_window_mut(state.window_id, cx) else {
                            return;
                        };
                        ui_window.x += dx;
                        ui_window.y += dy;
                        self.save_storage();
                    }
                    PointerInteraction::Resize(state) => {
                        let dx = x - state.last_x;
                        let dy = y - state.last_y;
                        state.last_x = x;
                        state.last_y = y;

                        let Some(ui_window) = self.pointer_window_mut(state.window_id, cx) else {
                            return;
                        };
                        ui_window.width = (ui_window.width + dx).max(MIN_WINDOW_SIZE);
                        ui_window.height = (ui_window.height + dy).max(MIN_WINDOW_SIZE);
                        self.save_storage();
                    }
                    PointerInteraction::ColumnResize(state) => {
                        let dx = x - state.last_x;
                        state.last_x = x;

                        let Some(ui_window) =
                            self.windows.get_mut(&WindowId::from(state.spreadsheet_id))
                        else {
                            return;
                        };
                        let Ok(spreadsheet) = ui_window.spreadsheet_mut() else {
                            return;
                        };
                        spreadsheet.resize_column_by(state.column, dx);
                        self.save_storage();
                    }
                }
                self.pointer_interaction = Some(pointer_interaction);
                cx.notify();
            }
            Event::ReleasedMouse => {
                self.pointer_interaction = None;
                cx.notify();
            }
        }
    }

    fn active_note_field(&self) -> Option<ActiveNoteField> {
        let ActiveFieldId::Note(active_field_id) = self.active_field.as_ref()? else {
            return None;
        };
        self.windows.iter().find_map(|(window_id, ui_window)| {
            if let WindowContent::Note(note) = &ui_window.content {
                if &note.body_field_id() == active_field_id {
                    Some(ActiveNoteField {
                        note_id: window_id.note_id()?,
                        kind: NoteFieldKind::Body,
                    })
                } else if &note.name_field_id() == active_field_id {
                    Some(ActiveNoteField {
                        note_id: window_id.note_id()?,
                        kind: NoteFieldKind::Name,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    fn pointer_window_mut(
        &mut self,
        window_id: WindowId,
        cx: &mut Context<Model>,
    ) -> Option<&mut Window> {
        if !self.windows.contains_key(&window_id) {
            self.pointer_interaction = None;
            cx.notify();
            return None;
        }

        self.windows.get_mut(&window_id)
    }

    fn bring_window_to_front(&mut self, window_id: WindowId) -> Option<WindowId> {
        if !self.windows.contains_key(&window_id) {
            return None;
        }

        self.window_order
            .retain(|ordered_window_id| *ordered_window_id != window_id);
        self.window_order.push(window_id);
        Some(window_id)
    }

    fn activate_note_body(&mut self, note_id: NoteId) -> Option<NoteId> {
        let front_window_id = self.bring_window_to_front(WindowId::from(note_id))?;
        self.active_field = Some(ActiveFieldId::Note(
            self.windows
                .get(&front_window_id)?
                .note()
                .ok()?
                .body_field_id(),
        ));
        Some(front_window_id.note_id()?)
    }

    fn activate_note_name(&mut self, note_id: NoteId) -> Option<NoteId> {
        let front_window_id = self.bring_window_to_front(WindowId::from(note_id))?;
        self.active_field = Some(ActiveFieldId::Note(
            self.windows
                .get(&front_window_id)?
                .note()
                .ok()?
                .name_field_id(),
        ));
        Some(front_window_id.note_id()?)
    }

    fn activate_spreadsheet_cell(
        &mut self,
        spreadsheet_id: SpreadsheetId,
        row: usize,
        column: usize,
    ) -> Option<SpreadsheetId> {
        let front_window_id = self.bring_window_to_front(WindowId::from(spreadsheet_id))?;
        self.active_field = Some(ActiveFieldId::Spreadsheet(spreadsheet::CellFieldId::new(
            front_window_id.spreadsheet_id()?,
            row,
            column,
        )));
        Some(front_window_id.spreadsheet_id()?)
    }

    fn activate_spreadsheet_name(
        &mut self,
        spreadsheet_id: SpreadsheetId,
    ) -> Option<SpreadsheetId> {
        let front_window_id = self.bring_window_to_front(WindowId::from(spreadsheet_id))?;
        self.active_field = Some(ActiveFieldId::SpreadsheetName(
            front_window_id.spreadsheet_id()?,
        ));
        Some(front_window_id.spreadsheet_id()?)
    }

    fn save_note(&mut self, note_id: NoteId, cx: &mut Context<Model>) -> bool {
        let Some(front_window_id) = self.bring_window_to_front(WindowId::from(note_id)) else {
            return false;
        };
        let Some(note) = self
            .windows
            .get_mut(&front_window_id)
            .and_then(|window| window.note_mut().ok())
        else {
            return false;
        };

        let save_note = note.save();
        self.dispatch_effect(Effect::SaveNote(save_note), cx);
        true
    }

    fn toggle_note_picker(&mut self) {
        if self.windows.contains_key(&WindowId::NotePicker) {
            self.refresh_note_picker();
            return;
        }

        let offset = self.window_order.len() as f32 * 24.0;
        self.windows.insert(
            WindowId::NotePicker,
            Window::new_note_picker(self.available_saved_note_paths(), offset),
        );
        self.window_order.push(WindowId::NotePicker);
    }

    fn refresh_note_picker(&mut self) {
        let paths = self.available_saved_note_paths();
        if let Some(Window {
            content: WindowContent::NotePicker {
                paths: picker_paths,
            },
            ..
        }) = self.windows.get_mut(&WindowId::NotePicker)
        {
            *picker_paths = paths;
        }
    }

    fn available_saved_note_paths(&self) -> Vec<PathBuf> {
        let mut paths = saved_note_paths().unwrap_or_else(|error| {
            eprintln!("failed to list saved notes: {error}");
            Vec::new()
        });
        paths.retain(|path| !self.note_path_is_open(path));
        paths
    }

    fn close_note_picker(&mut self) {
        self.windows.remove(&WindowId::NotePicker);
        self.window_order
            .retain(|window_id| *window_id != WindowId::NotePicker);
        if matches!(
            self.pointer_interaction,
            Some(PointerInteraction::Drag(PointerInteractionState {
                window_id: WindowId::NotePicker,
                ..
            })) | Some(PointerInteraction::Resize(PointerInteractionState {
                window_id: WindowId::NotePicker,
                ..
            }))
        ) {
            self.pointer_interaction = None;
        }
    }

    fn open_saved_note(&mut self, path: PathBuf) {
        if self.note_path_is_open(&path) {
            return;
        }

        let Some(storage) = load_storage_window_note(&path) else {
            return;
        };

        let offset = self.window_order.len() as f32 * 24.0;
        let note_id = self.next_note_id;
        let window_id = WindowId::Note(note_id);
        self.next_note_id = NoteId(self.next_note_id.0 + 1);
        self.windows.insert(
            window_id,
            Window {
                x: 32.0 + offset,
                y: 32.0 + offset,
                height: DEFAULT_WINDOW_SIZE,
                width: DEFAULT_WINDOW_SIZE,
                content: WindowContent::Note(note::Model::from_storage(
                    note_id,
                    storage,
                    Some(path),
                )),
            },
        );
        self.window_order.push(window_id);
        self.active_field = Some(ActiveFieldId::Note(
            self.windows
                .get(&window_id)
                .and_then(|window| window.note().ok())
                .expect("inserted note window")
                .body_field_id(),
        ));

        self.save_storage();
    }

    fn note_path_is_open(&self, path: &Path) -> bool {
        self.windows.values().any(|window| {
            window
                .note()
                .ok()
                .and_then(note::Model::path)
                .is_some_and(|open_path| open_path == path)
        })
    }

    fn commit_rename_and_focus_body(&mut self, note_id: NoteId) -> bool {
        let Some(note) = self
            .windows
            .get_mut(&WindowId::from(note_id))
            .and_then(|window| window.note_mut().ok())
        else {
            self.active_field = None;
            return false;
        };

        note.commit_rename();
        self.active_field = Some(ActiveFieldId::Note(note.body_field_id()));
        true
    }

    fn pressed_note_name_key(
        &mut self,
        active_field: ActiveNoteField,
        key_press: &note::KeyPress,
        cx: &mut Context<Model>,
    ) {
        match key_press {
            note::KeyPress::Backspace => {
                if let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(active_field.note_id))
                    .and_then(|window| window.note_mut().ok())
                {
                    note.pressed_name_backspace();
                }
                cx.notify();
            }
            note::KeyPress::OptionBackspace => {
                if let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(active_field.note_id))
                    .and_then(|window| window.note_mut().ok())
                {
                    note.pressed_name_option_backspace();
                }
                cx.notify();
            }
            note::KeyPress::CommandBackspace => {
                if let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(active_field.note_id))
                    .and_then(|window| window.note_mut().ok())
                {
                    note.pressed_name_command_backspace();
                }
                cx.notify();
            }
            note::KeyPress::Enter => {
                if !self.commit_rename_and_focus_body(active_field.note_id) {
                    return;
                }
                cx.notify();
            }
            note::KeyPress::Save => {
                if !self.commit_rename_and_focus_body(active_field.note_id) {
                    return;
                }
                self.save_note(active_field.note_id, cx);
                cx.notify();
            }
            note::KeyPress::Paste => {
                let Some(text) = clipboard_text(cx) else {
                    return;
                };
                if let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(active_field.note_id))
                    .and_then(|window| window.note_mut().ok())
                {
                    note.pressed_name_key(&text);
                }
                cx.notify();
            }
            note::KeyPress::ArrowUp => {
                if let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(active_field.note_id))
                    .and_then(|window| window.note_mut().ok())
                {
                    note.move_name_cursor_up();
                }
                cx.notify();
            }
            note::KeyPress::ArrowDown => {
                if let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(active_field.note_id))
                    .and_then(|window| window.note_mut().ok())
                {
                    note.move_name_cursor_down();
                }
                cx.notify();
            }
            note::KeyPress::ArrowLeft => {
                if let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(active_field.note_id))
                    .and_then(|window| window.note_mut().ok())
                {
                    note.move_name_cursor_left();
                }
                cx.notify();
            }
            note::KeyPress::ArrowRight => {
                if let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(active_field.note_id))
                    .and_then(|window| window.note_mut().ok())
                {
                    note.move_name_cursor_right();
                }
                cx.notify();
            }
            note::KeyPress::Text(key_char) => {
                if let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(active_field.note_id))
                    .and_then(|window| window.note_mut().ok())
                {
                    note.pressed_name_key(key_char);
                }
                cx.notify();
            }
        }
    }

    fn pressed_note_body_key(
        &mut self,
        active_field: ActiveNoteField,
        key_press: &note::KeyPress,
        cx: &mut Context<Model>,
    ) {
        let Some(note) = self
            .windows
            .get_mut(&WindowId::from(active_field.note_id))
            .and_then(|window| window.note_mut().ok())
        else {
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
                note.pressed_body_enter();
                cx.notify();
            }
            note::KeyPress::Save => {
                let save_note = note.save();
                self.dispatch_effect(Effect::SaveNote(save_note), cx);
                cx.notify();
            }
            note::KeyPress::Paste => {
                let Some(text) = clipboard_text(cx) else {
                    return;
                };
                note.pressed_body_key(&text);
                cx.notify();
            }
            note::KeyPress::Text(key_char) => {
                note.pressed_body_key(key_char);
                cx.notify();
            }
            note::KeyPress::ArrowUp => {
                note.move_body_cursor_up();
                cx.notify();
            }
            note::KeyPress::ArrowDown => {
                note.move_body_cursor_down();
                cx.notify();
            }
            note::KeyPress::ArrowLeft => {
                note.move_body_cursor_left();
                cx.notify();
            }
            note::KeyPress::ArrowRight => {
                note.move_body_cursor_right();
                cx.notify();
            }
        }
    }

    fn update_note(&mut self, note_event: note::IdEvent, cx: &mut Context<Model>) {
        let note_id = note_event.note_id;

        match &note_event.event {
            note::Event::PressedHeader { x, y } => {
                let Some(front_note_id) = self.activate_note_body(note_id) else {
                    return;
                };
                self.pointer_interaction =
                    Some(PointerInteraction::Drag(PointerInteractionState {
                        window_id: WindowId::from(front_note_id),
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
                        window_id: WindowId::from(front_note_id),
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

                if let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(front_note_id))
                    .and_then(|window| window.note_mut().ok())
                {
                    note.start_renaming();
                    self.active_field = Some(ActiveFieldId::Note(note.name_field_id()));
                }
                cx.notify();
            }
            note::Event::ClickedSaveName => {
                let Some(front_window_id) = self.bring_window_to_front(WindowId::from(note_id))
                else {
                    return;
                };
                if let Some(note) = self
                    .windows
                    .get_mut(&front_window_id)
                    .and_then(|window| window.note_mut().ok())
                {
                    note.commit_rename();
                    if self.active_field == Some(ActiveFieldId::Note(note.name_field_id())) {
                        self.active_field = Some(ActiveFieldId::Note(note.body_field_id()));
                    }
                }
                cx.notify();
            }
            note::Event::ClickedSaveButton => {
                self.pressed_button = None;
                self.save_note(note_id, cx);
                cx.notify();
            }
            note::Event::ClickedCloseButton => {
                let Some(front_window_id) = self.bring_window_to_front(WindowId::from(note_id))
                else {
                    return;
                };

                if let Some(closed_note) = self
                    .windows
                    .get(&front_window_id)
                    .and_then(|window| window.note().ok())
                {
                    self.pressed_button = None;
                    if self.active_field == Some(ActiveFieldId::Note(closed_note.name_field_id()))
                        || self.active_field
                            == Some(ActiveFieldId::Note(closed_note.body_field_id()))
                    {
                        self.active_field = None;
                    }
                    self.windows.remove(&front_window_id);
                    self.window_order
                        .retain(|ordered_window_id| *ordered_window_id != front_window_id);
                    self.pointer_interaction = None;
                    self.refresh_note_picker();
                    self.refresh_spreadsheet_picker();
                    cx.notify();
                }
            }
            note::Event::PressedButton { button_id } => {
                let Some(front_window_id) = self.bring_window_to_front(WindowId::from(note_id))
                else {
                    return;
                };
                let Some(front_note_id) = front_window_id.note_id() else {
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
                let Some(active_field) = self.active_note_field() else {
                    self.active_field = None;
                    return;
                };

                match active_field.kind {
                    NoteFieldKind::Name => self.pressed_note_name_key(active_field, key_press, cx),
                    NoteFieldKind::Body => self.pressed_note_body_key(active_field, key_press, cx),
                }
            }
        }
        self.save_storage();
    }

    fn save_spreadsheet(&mut self, spreadsheet_id: SpreadsheetId, cx: &mut Context<Model>) -> bool {
        let Some(front_window_id) = self.bring_window_to_front(WindowId::from(spreadsheet_id))
        else {
            return false;
        };
        let Some(spreadsheet) = self
            .windows
            .get_mut(&front_window_id)
            .and_then(|window| window.spreadsheet_mut().ok())
        else {
            return false;
        };

        let save_spreadsheet = spreadsheet.save();
        self.dispatch_effect(Effect::SaveSpreadsheet(save_spreadsheet), cx);
        true
    }

    fn toggle_spreadsheet_picker(&mut self) {
        if self.windows.contains_key(&WindowId::SpreadsheetPicker) {
            self.refresh_spreadsheet_picker();
            return;
        }

        let offset = self.window_order.len() as f32 * 24.0;
        self.windows.insert(
            WindowId::SpreadsheetPicker,
            Window::new_spreadsheet_picker(self.available_saved_spreadsheet_paths(), offset),
        );
        self.window_order.push(WindowId::SpreadsheetPicker);
    }

    fn refresh_spreadsheet_picker(&mut self) {
        let paths = self.available_saved_spreadsheet_paths();
        if let Some(Window {
            content:
                WindowContent::SpreadsheetPicker {
                    paths: picker_paths,
                },
            ..
        }) = self.windows.get_mut(&WindowId::SpreadsheetPicker)
        {
            *picker_paths = paths;
        }
    }

    fn available_saved_spreadsheet_paths(&self) -> Vec<PathBuf> {
        let mut paths = saved_spreadsheet_paths().unwrap_or_else(|error| {
            eprintln!("failed to list saved spreadsheets: {error}");
            Vec::new()
        });
        paths.retain(|path| !self.spreadsheet_path_is_open(path));
        paths
    }

    fn close_spreadsheet_picker(&mut self) {
        self.windows.remove(&WindowId::SpreadsheetPicker);
        self.window_order
            .retain(|window_id| *window_id != WindowId::SpreadsheetPicker);
        if matches!(
            self.pointer_interaction,
            Some(PointerInteraction::Drag(PointerInteractionState {
                window_id: WindowId::SpreadsheetPicker,
                ..
            })) | Some(PointerInteraction::Resize(PointerInteractionState {
                window_id: WindowId::SpreadsheetPicker,
                ..
            }))
        ) {
            self.pointer_interaction = None;
        }
    }

    fn open_saved_spreadsheet(&mut self, path: PathBuf) {
        if self.spreadsheet_path_is_open(&path) {
            return;
        }

        let Some(storage) = load_storage_window_spreadsheet(&path) else {
            return;
        };

        let offset = self.window_order.len() as f32 * 24.0;
        let spreadsheet_id = self.next_spreadsheet_id;
        let window_id = WindowId::Spreadsheet(spreadsheet_id);
        self.next_spreadsheet_id = SpreadsheetId(self.next_spreadsheet_id.0 + 1);
        self.windows.insert(
            window_id,
            Window {
                x: 48.0 + offset,
                y: 48.0 + offset,
                height: DEFAULT_WINDOW_SIZE,
                width: DEFAULT_WINDOW_SIZE * 1.5,
                content: WindowContent::Spreadsheet(spreadsheet::Model::from_storage(
                    spreadsheet_id,
                    storage,
                    Some(path),
                )),
            },
        );
        self.window_order.push(window_id);
        self.active_field = Some(ActiveFieldId::Spreadsheet(spreadsheet::CellFieldId::new(
            spreadsheet_id,
            0,
            0,
        )));

        self.save_storage();
    }

    fn spreadsheet_path_is_open(&self, path: &Path) -> bool {
        self.windows.values().any(|window| {
            window
                .spreadsheet()
                .ok()
                .and_then(spreadsheet::Model::path)
                .is_some_and(|open_path| open_path == path)
        })
    }

    fn commit_spreadsheet_rename_and_focus_cell(&mut self, spreadsheet_id: SpreadsheetId) -> bool {
        let Some(spreadsheet) = self
            .windows
            .get_mut(&WindowId::from(spreadsheet_id))
            .and_then(|window| window.spreadsheet_mut().ok())
        else {
            self.active_field = None;
            return false;
        };

        spreadsheet.commit_rename();
        self.active_field = Some(ActiveFieldId::Spreadsheet(
            spreadsheet.active_cell_field_id(),
        ));
        true
    }

    fn pressed_spreadsheet_name_key(
        &mut self,
        spreadsheet_id: SpreadsheetId,
        key_press: &spreadsheet::KeyPress,
        cx: &mut Context<Model>,
    ) {
        match key_press {
            spreadsheet::KeyPress::Backspace => {
                if let Some(spreadsheet) = self
                    .windows
                    .get_mut(&WindowId::from(spreadsheet_id))
                    .and_then(|window| window.spreadsheet_mut().ok())
                {
                    spreadsheet.pressed_name_backspace();
                }
                cx.notify();
            }
            spreadsheet::KeyPress::OptionBackspace => {
                if let Some(spreadsheet) = self
                    .windows
                    .get_mut(&WindowId::from(spreadsheet_id))
                    .and_then(|window| window.spreadsheet_mut().ok())
                {
                    spreadsheet.pressed_name_option_backspace();
                }
                cx.notify();
            }
            spreadsheet::KeyPress::CommandBackspace => {
                if let Some(spreadsheet) = self
                    .windows
                    .get_mut(&WindowId::from(spreadsheet_id))
                    .and_then(|window| window.spreadsheet_mut().ok())
                {
                    spreadsheet.pressed_name_command_backspace();
                }
                cx.notify();
            }
            spreadsheet::KeyPress::Enter => {
                if !self.commit_spreadsheet_rename_and_focus_cell(spreadsheet_id) {
                    return;
                }
                cx.notify();
            }
            spreadsheet::KeyPress::Save => {
                if !self.commit_spreadsheet_rename_and_focus_cell(spreadsheet_id) {
                    return;
                }
                self.save_spreadsheet(spreadsheet_id, cx);
                cx.notify();
            }
            spreadsheet::KeyPress::Text(key_char) => {
                if let Some(spreadsheet) = self
                    .windows
                    .get_mut(&WindowId::from(spreadsheet_id))
                    .and_then(|window| window.spreadsheet_mut().ok())
                {
                    spreadsheet.pressed_name_key(key_char);
                }
                cx.notify();
            }
            spreadsheet::KeyPress::Paste => {
                let Some(text) = clipboard_text(cx) else {
                    return;
                };
                if let Some(spreadsheet) = self
                    .windows
                    .get_mut(&WindowId::from(spreadsheet_id))
                    .and_then(|window| window.spreadsheet_mut().ok())
                {
                    spreadsheet.pressed_name_key(&text);
                }
                cx.notify();
            }
            spreadsheet::KeyPress::ShiftEnter
            | spreadsheet::KeyPress::Tab
            | spreadsheet::KeyPress::ShiftTab
            | spreadsheet::KeyPress::ArrowUp
            | spreadsheet::KeyPress::ArrowDown
            | spreadsheet::KeyPress::ArrowLeft
            | spreadsheet::KeyPress::ArrowRight => {}
        }
    }

    fn update_spreadsheet_shape(
        &mut self,
        spreadsheet_id: SpreadsheetId,
        update: impl FnOnce(&mut spreadsheet::Model),
    ) {
        let Some(front_window_id) = self.bring_window_to_front(WindowId::from(spreadsheet_id))
        else {
            return;
        };
        let Some(spreadsheet) = self
            .windows
            .get_mut(&front_window_id)
            .and_then(|window| window.spreadsheet_mut().ok())
        else {
            self.active_field = None;
            return;
        };

        update(spreadsheet);
        self.active_field = Some(ActiveFieldId::Spreadsheet(
            spreadsheet.active_cell_field_id(),
        ));
    }

    fn update_spreadsheet(
        &mut self,
        spreadsheet_event: spreadsheet::IdEvent,
        cx: &mut Context<Model>,
    ) {
        let spreadsheet_id = spreadsheet_event.spreadsheet_id;

        match &spreadsheet_event.event {
            spreadsheet::Event::PressedHeader { x, y } => {
                let Some(front_spreadsheet_id) =
                    self.activate_spreadsheet_cell(spreadsheet_id, 0, 0)
                else {
                    return;
                };
                self.pointer_interaction =
                    Some(PointerInteraction::Drag(PointerInteractionState {
                        window_id: WindowId::from(front_spreadsheet_id),
                        last_x: *x,
                        last_y: *y,
                    }));
                cx.notify();
            }
            spreadsheet::Event::PressedResizeHandle { x, y } => {
                let Some(front_spreadsheet_id) =
                    self.activate_spreadsheet_cell(spreadsheet_id, 0, 0)
                else {
                    return;
                };
                self.pointer_interaction =
                    Some(PointerInteraction::Resize(PointerInteractionState {
                        window_id: WindowId::from(front_spreadsheet_id),
                        last_x: *x,
                        last_y: *y,
                    }));
                cx.notify();
            }
            spreadsheet::Event::PressedCell { row, column } => {
                self.activate_spreadsheet_cell(spreadsheet_id, *row, *column);
                cx.notify();
            }
            spreadsheet::Event::PressedRowHeader { row } => {
                let Some(front_window_id) =
                    self.bring_window_to_front(WindowId::from(spreadsheet_id))
                else {
                    return;
                };
                if let Some(spreadsheet) = self
                    .windows
                    .get_mut(&front_window_id)
                    .and_then(|window| window.spreadsheet_mut().ok())
                {
                    spreadsheet.select_row(*row);
                    self.active_field = Some(ActiveFieldId::Spreadsheet(
                        spreadsheet.active_cell_field_id(),
                    ));
                }
                cx.notify();
            }
            spreadsheet::Event::PressedColumnHeader { column } => {
                let Some(front_window_id) =
                    self.bring_window_to_front(WindowId::from(spreadsheet_id))
                else {
                    return;
                };
                if let Some(spreadsheet) = self
                    .windows
                    .get_mut(&front_window_id)
                    .and_then(|window| window.spreadsheet_mut().ok())
                {
                    spreadsheet.select_column(*column);
                    self.active_field = Some(ActiveFieldId::Spreadsheet(
                        spreadsheet.active_cell_field_id(),
                    ));
                }
                cx.notify();
            }
            spreadsheet::Event::PressedColumnResizeHandle { column, x } => {
                let Some(front_window_id) =
                    self.bring_window_to_front(WindowId::from(spreadsheet_id))
                else {
                    return;
                };
                self.active_field = self
                    .windows
                    .get(&front_window_id)
                    .and_then(|window| window.spreadsheet().ok())
                    .map(|spreadsheet| {
                        ActiveFieldId::Spreadsheet(spreadsheet.active_cell_field_id())
                    });
                self.pointer_interaction = Some(PointerInteraction::ColumnResize(
                    ColumnResizeInteractionState {
                        spreadsheet_id,
                        column: *column,
                        last_x: *x,
                    },
                ));
                cx.notify();
            }
            spreadsheet::Event::ClickedInsertRowAbove { row } => {
                self.update_spreadsheet_shape(spreadsheet_id, |spreadsheet| {
                    spreadsheet.insert_row_above(*row);
                });
                cx.notify();
            }
            spreadsheet::Event::ClickedInsertRowBelow { row } => {
                self.update_spreadsheet_shape(spreadsheet_id, |spreadsheet| {
                    spreadsheet.insert_row_below(*row);
                });
                cx.notify();
            }
            spreadsheet::Event::ClickedDeleteRow { row } => {
                self.update_spreadsheet_shape(spreadsheet_id, |spreadsheet| {
                    spreadsheet.delete_row(*row);
                });
                cx.notify();
            }
            spreadsheet::Event::ClickedInsertColumnLeft { column } => {
                self.update_spreadsheet_shape(spreadsheet_id, |spreadsheet| {
                    spreadsheet.insert_column_left(*column);
                });
                cx.notify();
            }
            spreadsheet::Event::ClickedInsertColumnRight { column } => {
                self.update_spreadsheet_shape(spreadsheet_id, |spreadsheet| {
                    spreadsheet.insert_column_right(*column);
                });
                cx.notify();
            }
            spreadsheet::Event::ClickedDeleteColumn { column } => {
                self.update_spreadsheet_shape(spreadsheet_id, |spreadsheet| {
                    spreadsheet.delete_column(*column);
                });
                cx.notify();
            }
            spreadsheet::Event::PressedNameEditor => {
                self.activate_spreadsheet_name(spreadsheet_id);
                cx.notify();
            }
            spreadsheet::Event::ClickedRename => {
                let Some(front_spreadsheet_id) = self.activate_spreadsheet_name(spreadsheet_id)
                else {
                    return;
                };

                if let Some(spreadsheet) = self
                    .windows
                    .get_mut(&WindowId::from(front_spreadsheet_id))
                    .and_then(|window| window.spreadsheet_mut().ok())
                {
                    spreadsheet.start_renaming();
                    self.active_field = Some(ActiveFieldId::SpreadsheetName(spreadsheet.id));
                }
                cx.notify();
            }
            spreadsheet::Event::ClickedSaveName => {
                let Some(front_window_id) =
                    self.bring_window_to_front(WindowId::from(spreadsheet_id))
                else {
                    return;
                };
                if let Some(spreadsheet) = self
                    .windows
                    .get_mut(&front_window_id)
                    .and_then(|window| window.spreadsheet_mut().ok())
                {
                    spreadsheet.commit_rename();
                    if self.active_field == Some(ActiveFieldId::SpreadsheetName(spreadsheet.id)) {
                        self.active_field = Some(ActiveFieldId::Spreadsheet(
                            spreadsheet.active_cell_field_id(),
                        ));
                    }
                }
                cx.notify();
            }
            spreadsheet::Event::ClickedSaveButton => {
                self.pressed_button = None;
                self.save_spreadsheet(spreadsheet_id, cx);
                cx.notify();
            }
            spreadsheet::Event::ClickedCloseButton => {
                let Some(front_window_id) =
                    self.bring_window_to_front(WindowId::from(spreadsheet_id))
                else {
                    return;
                };

                if let Some(closed_spreadsheet) = self
                    .windows
                    .get(&front_window_id)
                    .and_then(|window| window.spreadsheet().ok())
                {
                    self.pressed_button = None;
                    if matches!(
                        &self.active_field,
                        Some(ActiveFieldId::Spreadsheet(cell))
                            if cell.spreadsheet_id == closed_spreadsheet.id
                    ) {
                        self.active_field = None;
                    }
                    self.windows.remove(&front_window_id);
                    self.window_order
                        .retain(|ordered_window_id| *ordered_window_id != front_window_id);
                    self.pointer_interaction = None;
                    cx.notify();
                }
            }
            spreadsheet::Event::PressedButton { button_id } => {
                let Some(front_window_id) =
                    self.bring_window_to_front(WindowId::from(spreadsheet_id))
                else {
                    return;
                };
                let Some(front_spreadsheet_id) = front_window_id.spreadsheet_id() else {
                    return;
                };
                self.pressed_button = Some(ButtonId::SpreadsheetButtonId {
                    spreadsheet_id: front_spreadsheet_id,
                    button_id: button_id.clone(),
                });
                cx.notify();
            }
            spreadsheet::Event::ReleasedButton => {
                self.pressed_button = None;
                cx.notify();
            }
            spreadsheet::Event::PressedKey(key_press) => {
                let active_field = self.active_field.clone();

                if active_field == Some(ActiveFieldId::SpreadsheetName(spreadsheet_id)) {
                    self.pressed_spreadsheet_name_key(spreadsheet_id, key_press, cx);
                    self.save_storage();
                    return;
                }

                let Some(ActiveFieldId::Spreadsheet(active_cell)) = active_field else {
                    self.active_field = None;
                    return;
                };
                if active_cell.spreadsheet_id != spreadsheet_id {
                    return;
                }

                if matches!(key_press, spreadsheet::KeyPress::Save) {
                    self.save_spreadsheet(spreadsheet_id, cx);
                    cx.notify();
                    self.save_storage();
                    return;
                }

                let pasted_text = if matches!(key_press, spreadsheet::KeyPress::Paste) {
                    clipboard_text(cx)
                } else {
                    None
                };

                let Some(spreadsheet) = self
                    .windows
                    .get_mut(&WindowId::from(spreadsheet_id))
                    .and_then(|window| window.spreadsheet_mut().ok())
                else {
                    self.active_field = None;
                    return;
                };

                if let Some(text) = pasted_text {
                    spreadsheet.paste_text(active_cell.row, active_cell.column, &text);
                } else {
                    spreadsheet.pressed_key(active_cell.row, active_cell.column, key_press);
                }
                self.active_field = Some(ActiveFieldId::Spreadsheet(
                    spreadsheet.active_cell_field_id(),
                ));
                cx.notify();
            }
        }
        self.save_storage();
    }
}

fn storage_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui.json")
}

fn load_storage_file() -> io::Result<Storage> {
    let contents = std::fs::read_to_string(storage_path())?;
    serde_json::from_str(&contents).map_err(io::Error::other)
}

fn load_storage_window_note(note_path: &Path) -> Option<note::Storage> {
    match note::load_note_file(note_path) {
        Ok(note) => Some(note),
        Err(error) => {
            eprintln!("failed to load note {}: {error}", note_path.display());
            None
        }
    }
}

fn load_storage_window_spreadsheet(spreadsheet_path: &Path) -> Option<spreadsheet::Storage> {
    match spreadsheet::load_spreadsheet_file(spreadsheet_path) {
        Ok(spreadsheet) => Some(spreadsheet),
        Err(error) => {
            eprintln!(
                "failed to load spreadsheet {}: {error}",
                spreadsheet_path.display()
            );
            None
        }
    }
}

fn saved_note_paths() -> io::Result<Vec<PathBuf>> {
    let notes_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("notes");
    let mut paths = Vec::new();
    let entries = match std::fs::read_dir(notes_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(paths),
        Err(error) => return Err(error),
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            paths.push(path);
        }
    }

    paths.sort();
    Ok(paths)
}

fn saved_spreadsheet_paths() -> io::Result<Vec<PathBuf>> {
    let spreadsheets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spreadsheets");
    let mut paths = Vec::new();
    let entries = match std::fs::read_dir(spreadsheets_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(paths),
        Err(error) => return Err(error),
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("csv") {
            paths.push(path);
        }
    }

    paths.sort();
    Ok(paths)
}

fn save_storage_file(storage: &Storage) -> io::Result<PathBuf> {
    let path = storage_path();
    let contents = serde_json::to_string_pretty(storage).map_err(io::Error::other)?;
    std::fs::write(&path, contents)?;
    Ok(path)
}

impl EventEmitter<note::IdEvent> for Model {}
impl EventEmitter<spreadsheet::IdEvent> for Model {}

impl Focusable for Model {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl LoadedState {
    fn render(
        &mut self,
        focus_handle: &FocusHandle,
        window: &mut gpui::Window,
        cx: &mut Context<Model>,
    ) -> gpui::Div {
        let is_focused = focus_handle.is_focused(window);
        let ui_windows = self
            .window_order
            .iter()
            .filter_map(|window_id| {
                let Some(ui_window) = self.windows.get(window_id) else {
                    eprintln!("window order referenced a missing window");
                    return None;
                };
                let rendered_content = match &ui_window.content {
                    WindowContent::Note(note) => {
                        let pressed_note_button = match self.pressed_button.as_ref() {
                            Some(ButtonId::NoteButtonId {
                                note_id: pressed_note_id,
                                button_id,
                            }) if pressed_note_id == &note.id => Some(button_id),
                            _ => None,
                        };
                        let active_note_field = match self.active_field.as_ref() {
                            Some(ActiveFieldId::Note(field_id)) => Some(field_id),
                            _ => None,
                        };

                        note::render(
                            note,
                            focus_handle,
                            pressed_note_button,
                            active_note_field,
                            is_focused,
                            cx,
                        )
                    }
                    WindowContent::Spreadsheet(spreadsheet) => {
                        let pressed_spreadsheet_button = match self.pressed_button.as_ref() {
                            Some(ButtonId::SpreadsheetButtonId {
                                spreadsheet_id: pressed_spreadsheet_id,
                                button_id,
                            }) if pressed_spreadsheet_id == &spreadsheet.id => Some(button_id),
                            _ => None,
                        };
                        let active_cell = match self.active_field.as_ref() {
                            Some(ActiveFieldId::Spreadsheet(cell))
                                if cell.spreadsheet_id == spreadsheet.id =>
                            {
                                Some(cell)
                            }
                            _ => None,
                        };
                        let show_name_cursor = self.active_field
                            == Some(ActiveFieldId::SpreadsheetName(spreadsheet.id));

                        spreadsheet::render(
                            spreadsheet,
                            focus_handle,
                            pressed_spreadsheet_button,
                            active_cell,
                            show_name_cursor,
                            is_focused,
                            cx,
                        )
                    }
                    WindowContent::NotePicker { paths } => {
                        render_saved_note_picker(paths, focus_handle, is_focused, cx)
                    }
                    WindowContent::SpreadsheetPicker { paths } => {
                        render_saved_spreadsheet_picker(paths, focus_handle, is_focused, cx)
                    }
                };

                Some(
                    rendered_content
                        .absolute()
                        .left(gpui::px(ui_window.x))
                        .top(gpui::px(ui_window.y))
                        .w(gpui::px(ui_window.width))
                        .h(gpui::px(ui_window.height)),
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
            .on_mouse_move(cx.listener(|model, event, _, cx| {
                model.handle_event(Event::MovedMouse(event), cx);
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|model, _event, _, cx| {
                    model.handle_event(Event::ReleasedMouse, cx);
                }),
            )
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
                    .children(ui_windows),
            )
            .child(toolbar(
                self.pressed_button == Some(ButtonId::NewNote),
                self.pressed_button == Some(ButtonId::NewSpreadsheet),
                self.pressed_button == Some(ButtonId::OpenNotePicker),
                self.pressed_button == Some(ButtonId::OpenSpreadsheetPicker),
                cx,
            ))
    }
}

impl Render for Model {
    fn render(&mut self, window: &mut gpui::Window, cx: &mut Context<Model>) -> impl IntoElement {
        match &mut self.state {
            LoadingState::Loading => gpui::div()
                .size_full()
                .font_family(s::FONT)
                .bg(s::GREEN3)
                .text_color(s::GRAY6)
                .child("loading..."),
            LoadingState::Loaded(state) => state.render(&self.focus_handle, window, cx),
        }
    }
}

fn toolbar(
    new_note_button_pressed: bool,
    new_spreadsheet_button_pressed: bool,
    open_note_picker_button_pressed: bool,
    open_spreadsheet_picker_button_pressed: bool,
    cx: &mut Context<Model>,
) -> impl IntoElement {
    gpui::div()
        .flex()
        .items_center()
        .border_t_2()
        .border_color(s::GRAY3)
        .bg(s::GRAY2)
        .p(s::S3)
        .gap_3()
        .child(
            view::button::from_text("new note", new_note_button_pressed)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|model, _event, _, cx| {
                        model.handle_event(Event::PressedNewNoteButton, cx);
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|model, _event, window, cx| {
                        window.focus(&model.focus_handle);
                        model.handle_event(Event::ClickedNewNoteButton, cx);
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|model, _event, _, cx| {
                        model.handle_event(Event::ReleasedNewNoteButtonOutside, cx);
                    }),
                ),
        )
        .child(
            view::button::from_text("new sheet", new_spreadsheet_button_pressed)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|model, _event, _, cx| {
                        model.handle_event(Event::PressedNewSpreadsheetButton, cx);
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|model, _event, window, cx| {
                        window.focus(&model.focus_handle);
                        model.handle_event(Event::ClickedNewSpreadsheetButton, cx);
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|model, _event, _, cx| {
                        model.handle_event(Event::ReleasedNewSpreadsheetButtonOutside, cx);
                    }),
                ),
        )
        .child(
            view::button::from_text("open note", open_note_picker_button_pressed)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|model, _event, _, cx| {
                        model.handle_event(Event::PressedOpenNotePickerButton, cx);
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|model, _event, window, cx| {
                        window.focus(&model.focus_handle);
                        model.handle_event(Event::ClickedOpenNotePickerButton, cx);
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|model, _event, _, cx| {
                        model.handle_event(Event::ReleasedOpenNotePickerButtonOutside, cx);
                    }),
                ),
        )
        .child(
            view::button::from_text("open sheet", open_spreadsheet_picker_button_pressed)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|model, _event, _, cx| {
                        model.handle_event(Event::PressedOpenSpreadsheetPickerButton, cx);
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|model, _event, window, cx| {
                        window.focus(&model.focus_handle);
                        model.handle_event(Event::ClickedOpenSpreadsheetPickerButton, cx);
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|model, _event, _, cx| {
                        model.handle_event(Event::ReleasedOpenSpreadsheetPickerButtonOutside, cx);
                    }),
                ),
        )
}

fn render_saved_note_picker(
    paths: &[PathBuf],
    focus_handle: &FocusHandle,
    is_focused: bool,
    cx: &mut Context<Model>,
) -> gpui::Div {
    let header_focus_handle = focus_handle.clone();
    let note_rows = if paths.is_empty() {
        vec![gpui::div()
            .p(s::S3)
            .text_color(s::YELLOW3)
            .child("no saved notes")]
    } else {
        paths
            .iter()
            .cloned()
            .map(|path| {
                gpui::div()
                    .w_full()
                    .min_h(s::S6)
                    .flex()
                    .items_center()
                    .px(s::S4)
                    .py(s::S3)
                    .bg(s::GREEN2)
                    .text_color(s::GRAY6)
                    .cursor_pointer()
                    .hover(|row| row.bg(s::GREEN4))
                    .child(saved_note_label(&path))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |model, _event, window, cx| {
                            window.focus(&model.focus_handle);
                            cx.stop_propagation();
                            model.handle_event(Event::ClickedSavedNote(path.clone()), cx);
                        }),
                    )
            })
            .collect()
    };

    let title = if is_focused { "open note" } else { "open note" };

    s::raised(
        gpui::div()
            .flex()
            .flex_col()
            .size_full()
            .bg(s::GRAY2)
            .p(s::S3)
            .child(
                gpui::div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_between()
                    .bg(s::GRAY5)
                    .text_color(s::GREEN1)
                    .p(s::S3)
                    .px(s::S4)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |model, event: &MouseDownEvent, window, cx| {
                            window.focus(&header_focus_handle);
                            cx.stop_propagation();
                            model.handle_event(
                                Event::PressedNotePickerHeader {
                                    x: event.position.x.into(),
                                    y: event.position.y.into(),
                                },
                                cx,
                            );
                        }),
                    )
                    .child(title)
                    .child(view::button::x(false).on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|model, _event, _, cx| {
                            cx.stop_propagation();
                            model.handle_event(Event::ClickedCloseNotePicker, cx);
                        }),
                    )),
            )
            .child(
                s::sunken(
                    gpui::div()
                        .id("saved-note-picker-list")
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_h(gpui::px(0.0))
                        .bg(s::GREEN1)
                        .p(s::S3)
                        .children(note_rows),
                )
                .flex_1()
                .min_h(gpui::px(0.0)),
            ),
    )
    .child(resize_note_picker_handle(focus_handle, cx))
}

fn render_saved_spreadsheet_picker(
    paths: &[PathBuf],
    focus_handle: &FocusHandle,
    is_focused: bool,
    cx: &mut Context<Model>,
) -> gpui::Div {
    let header_focus_handle = focus_handle.clone();
    let sheet_rows = if paths.is_empty() {
        vec![gpui::div()
            .p(s::S3)
            .text_color(s::YELLOW3)
            .child("no saved sheets")]
    } else {
        paths
            .iter()
            .cloned()
            .map(|path| {
                gpui::div()
                    .w_full()
                    .min_h(s::S6)
                    .flex()
                    .items_center()
                    .px(s::S4)
                    .py(s::S3)
                    .bg(s::GREEN2)
                    .text_color(s::GRAY6)
                    .cursor_pointer()
                    .hover(|row| row.bg(s::GREEN4))
                    .child(saved_spreadsheet_label(&path))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |model, _event, window, cx| {
                            window.focus(&model.focus_handle);
                            cx.stop_propagation();
                            model.handle_event(Event::ClickedSavedSpreadsheet(path.clone()), cx);
                        }),
                    )
            })
            .collect()
    };

    let title = if is_focused {
        "open sheet"
    } else {
        "open sheet"
    };

    s::raised(
        gpui::div()
            .flex()
            .flex_col()
            .size_full()
            .bg(s::GRAY2)
            .p(s::S3)
            .child(
                gpui::div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_between()
                    .bg(s::GRAY5)
                    .text_color(s::GREEN1)
                    .p(s::S3)
                    .px(s::S4)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |model, event: &MouseDownEvent, window, cx| {
                            window.focus(&header_focus_handle);
                            cx.stop_propagation();
                            model.handle_event(
                                Event::PressedSpreadsheetPickerHeader {
                                    x: event.position.x.into(),
                                    y: event.position.y.into(),
                                },
                                cx,
                            );
                        }),
                    )
                    .child(title)
                    .child(view::button::x(false).on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|model, _event, _, cx| {
                            cx.stop_propagation();
                            model.handle_event(Event::ClickedCloseSpreadsheetPicker, cx);
                        }),
                    )),
            )
            .child(
                s::sunken(
                    gpui::div()
                        .id("saved-spreadsheet-picker-list")
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_h(gpui::px(0.0))
                        .bg(s::GREEN1)
                        .p(s::S3)
                        .children(sheet_rows),
                )
                .flex_1()
                .min_h(gpui::px(0.0)),
            ),
    )
    .child(resize_spreadsheet_picker_handle(focus_handle, cx))
}

fn resize_note_picker_handle(
    focus_handle: &FocusHandle,
    cx: &mut Context<Model>,
) -> impl IntoElement {
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
            cx.listener(move |model, event: &MouseDownEvent, window, cx| {
                window.focus(&focus_handle);
                cx.stop_propagation();
                model.handle_event(
                    Event::PressedNotePickerResizeHandle {
                        x: event.position.x.into(),
                        y: event.position.y.into(),
                    },
                    cx,
                );
            }),
        )
}

fn resize_spreadsheet_picker_handle(
    focus_handle: &FocusHandle,
    cx: &mut Context<Model>,
) -> impl IntoElement {
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
            cx.listener(move |model, event: &MouseDownEvent, window, cx| {
                window.focus(&focus_handle);
                cx.stop_propagation();
                model.handle_event(
                    Event::PressedSpreadsheetPickerResizeHandle {
                        x: event.position.x.into(),
                        y: event.position.y.into(),
                    },
                    cx,
                );
            }),
        )
}

fn saved_spreadsheet_label(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("spreadsheet")
        .replace('-', " ")
}

fn saved_note_label(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("note")
        .replace('-', " ")
}
