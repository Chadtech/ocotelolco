use gpui::{
    prelude::*, Context, EventEmitter, FocusHandle, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseUpEvent,
};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
};

use crate::ui::{style as s, view};

const DEFAULT_ROWS: usize = 12;
const DEFAULT_COLUMNS: usize = 8;
const CELL_WIDTH: f32 = 96.0;
const MIN_CELL_WIDTH: f32 = 40.0;
const CELL_HEIGHT: f32 = 28.0;
const ROW_HEADER_WIDTH: f32 = 36.0;
const ROW_ACTION_WIDTH: f32 = 84.0;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpreadsheetId(pub u64);

#[derive(Clone, PartialEq, Eq)]
pub struct CellFieldId {
    pub spreadsheet_id: SpreadsheetId,
    pub row: usize,
    pub column: usize,
}

impl CellFieldId {
    pub fn new(spreadsheet_id: SpreadsheetId, row: usize, column: usize) -> Self {
        Self {
            spreadsheet_id,
            row,
            column,
        }
    }
}

pub struct Model {
    pub id: SpreadsheetId,
    pub name: String,
    pub renaming: RenamingState,
    save_state: SaveState,
    edit_generation: u64,
    rows: Vec<Vec<String>>,
    active_row: usize,
    active_column: usize,
    selected_row: Option<usize>,
    selected_column: Option<usize>,
    column_widths: Vec<f32>,
    path: Option<PathBuf>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Storage {
    pub name: String,
    pub rows: Vec<Vec<String>>,
    #[serde(default)]
    pub column_widths: Vec<f32>,
}

#[derive(Clone, Deserialize, Serialize)]
pub enum StorageState {
    Saved {
        path: PathBuf,
        #[serde(default)]
        column_widths: Vec<f32>,
    },
    Unsaved(Storage),
}

enum InitFlags {
    New {
        ordinal: usize,
    },
    FromStorage {
        storage: Storage,
        path: Option<PathBuf>,
    },
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
    Rename,
    Save,
    Delete,
    ConfirmDelete,
    CancelDelete,
    X,
}

#[derive(Clone)]
pub enum Event {
    PressedHeader { x: f32, y: f32 },
    PressedResizeHandle { x: f32, y: f32 },
    PressedCell { row: usize, column: usize },
    PressedRowHeader { row: usize },
    PressedColumnHeader { column: usize },
    PressedColumnResizeHandle { column: usize, x: f32 },
    ClickedInsertRowAbove { row: usize },
    ClickedInsertRowBelow { row: usize },
    ClickedDeleteRow { row: usize },
    ClickedInsertColumnLeft { column: usize },
    ClickedInsertColumnRight { column: usize },
    ClickedDeleteColumn { column: usize },
    PressedNameEditor,
    ClickedRename,
    ClickedSaveName,
    ClickedSaveButton,
    ClickedDeleteButton,
    ClickedConfirmDeleteButton,
    ClickedCancelDeleteButton,
    ClickedCloseButton,
    PressedButton { button_id: ButtonId },
    ReleasedButton,
    PressedKey(KeyPress),
}

#[derive(Clone)]
pub struct IdEvent {
    pub spreadsheet_id: SpreadsheetId,
    pub event: Event,
}

pub struct SaveRequest {
    pub spreadsheet_id: SpreadsheetId,
    pub generation: u64,
    storage: Storage,
}

pub struct DeleteRequest {
    path: Option<PathBuf>,
}

#[derive(Clone)]
pub enum KeyPress {
    Backspace,
    OptionBackspace,
    CommandBackspace,
    Enter,
    ShiftEnter,
    Tab,
    ShiftTab,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Save,
    Paste,
    Text(String),
}

impl Model {
    pub fn new(id: SpreadsheetId, ordinal: usize) -> Self {
        Self::initialize(id, InitFlags::New { ordinal })
    }

    pub fn from_storage(id: SpreadsheetId, storage: Storage, path: Option<PathBuf>) -> Self {
        Self::initialize(id, InitFlags::FromStorage { storage, path })
    }

    fn initialize(id: SpreadsheetId, init_flags: InitFlags) -> Self {
        let (name, rows, column_widths, path) = match init_flags {
            InitFlags::New { ordinal } => (
                format!("spreadsheet {ordinal}"),
                vec![vec![String::new(); DEFAULT_COLUMNS]; DEFAULT_ROWS],
                vec![CELL_WIDTH; DEFAULT_COLUMNS],
                None,
            ),
            InitFlags::FromStorage { storage, path } => {
                (storage.name, storage.rows, storage.column_widths, path)
            }
        };

        let mut spreadsheet = Self {
            id,
            name,
            renaming: RenamingState::NotRenaming,
            save_state: SaveState::Idle,
            edit_generation: 0,
            rows,
            active_row: 0,
            active_column: 0,
            selected_row: None,
            selected_column: None,
            column_widths,
            path,
        };
        spreadsheet.normalize_shape();
        spreadsheet
    }

    pub fn to_storage(&self) -> Storage {
        Storage {
            name: self.name.clone(),
            rows: self.rows.clone(),
            column_widths: self.column_widths.clone(),
        }
    }

    pub fn to_storage_state(&self) -> StorageState {
        match self.path.as_ref() {
            Some(path) => StorageState::Saved {
                path: PathBuf::from(path),
                column_widths: self.column_widths.clone(),
            },
            None => StorageState::Unsaved(self.to_storage()),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn delete_request(&self) -> DeleteRequest {
        DeleteRequest {
            path: self.path.clone(),
        }
    }

    pub fn active_cell_field_id(&self) -> CellFieldId {
        CellFieldId::new(self.id, self.active_row, self.active_column)
    }

    pub fn select_row(&mut self, row: usize) {
        self.selected_row = Some(row.min(self.rows.len().saturating_sub(1)));
        self.selected_column = None;
    }

    pub fn select_column(&mut self, column: usize) {
        self.selected_column = Some(column.min(self.column_count().saturating_sub(1)));
        self.selected_row = None;
    }

    pub fn insert_row_above(&mut self, row: usize) {
        let row = row.min(self.rows.len());
        self.rows
            .insert(row, vec![String::new(); self.column_count()]);
        self.active_row = row;
        self.selected_row = Some(row);
        self.selected_column = None;
        self.started_editing();
    }

    pub fn insert_row_below(&mut self, row: usize) {
        let row = row.saturating_add(1).min(self.rows.len());
        self.rows
            .insert(row, vec![String::new(); self.column_count()]);
        self.active_row = row;
        self.selected_row = Some(row);
        self.selected_column = None;
        self.started_editing();
    }

    pub fn delete_row(&mut self, row: usize) {
        if self.rows.len() <= 1 {
            return;
        }

        let row = row.min(self.rows.len().saturating_sub(1));
        self.rows.remove(row);
        self.active_row = row.min(self.rows.len().saturating_sub(1));
        self.selected_row = Some(self.active_row);
        self.selected_column = None;
        self.started_editing();
    }

    pub fn insert_column_left(&mut self, column: usize) {
        let column_count = self.column_count();
        let column = column.min(column_count);
        for row in &mut self.rows {
            row.insert(column, String::new());
        }
        self.column_widths.insert(column, CELL_WIDTH);
        self.active_column = column;
        self.selected_column = Some(column);
        self.selected_row = None;
        self.started_editing();
    }

    pub fn insert_column_right(&mut self, column: usize) {
        let column_count = self.column_count();
        let column = column.saturating_add(1).min(column_count);
        for row in &mut self.rows {
            row.insert(column, String::new());
        }
        self.column_widths.insert(column, CELL_WIDTH);
        self.active_column = column;
        self.selected_column = Some(column);
        self.selected_row = None;
        self.started_editing();
    }

    pub fn delete_column(&mut self, column: usize) {
        if self.column_count() <= 1 {
            return;
        }

        let column_count = self.column_count();
        let column = column.min(column_count.saturating_sub(1));
        for row in &mut self.rows {
            row.remove(column);
        }
        self.column_widths.remove(column);
        self.active_column = column.min(self.column_count().saturating_sub(1));
        self.selected_column = Some(self.active_column);
        self.selected_row = None;
        self.started_editing();
    }

    pub fn save(&mut self) -> SaveRequest {
        self.save_state = SaveState::Saving;
        SaveRequest {
            spreadsheet_id: self.id,
            generation: self.edit_generation,
            storage: self.to_storage(),
        }
    }

    pub fn start_renaming(&mut self) {
        self.renaming = RenamingState::Renaming {
            name_field: self.name.clone(),
        };
    }

    pub fn commit_rename(&mut self) {
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
            name_field.clear();
        }
    }

    pub fn pressed_name_key(&mut self, key_char: &str) {
        if let RenamingState::Renaming { name_field } = &mut self.renaming {
            name_field.push_str(key_char);
        }
    }

    pub fn finished_saving(&mut self, generation: u64, result: Result<PathBuf, String>) {
        if generation != self.edit_generation {
            return;
        }

        self.save_state = match result {
            Ok(path) => {
                self.path = Some(path);
                SaveState::Saved
            }
            Err(_) => SaveState::Failed,
        };
    }

    pub fn pressed_key(&mut self, row: usize, column: usize, key_press: &KeyPress) {
        self.set_active_cell(row, column);

        match key_press {
            KeyPress::Backspace => {
                self.cell_mut(row, column).pop();
                self.started_editing();
            }
            KeyPress::OptionBackspace => {
                delete_previous_word(self.cell_mut(row, column));
                self.started_editing();
            }
            KeyPress::CommandBackspace => {
                self.cell_mut(row, column).clear();
                self.started_editing();
            }
            KeyPress::Enter | KeyPress::ArrowDown => self.move_active_cell(1, 0),
            KeyPress::ShiftEnter | KeyPress::ArrowUp => self.move_active_cell(-1, 0),
            KeyPress::Tab | KeyPress::ArrowRight => self.move_active_cell(0, 1),
            KeyPress::ShiftTab | KeyPress::ArrowLeft => self.move_active_cell(0, -1),
            KeyPress::Save => {}
            KeyPress::Paste => {}
            KeyPress::Text(key_char) => {
                self.cell_mut(row, column).push_str(key_char);
                self.started_editing();
            }
        }
    }

    pub fn paste_text(&mut self, row: usize, column: usize, text: &str) {
        self.set_active_cell(row, column);
        self.cell_mut(row, column).push_str(text);
        self.started_editing();
    }

    pub fn resize_column_by(&mut self, column: usize, dx: f32) {
        self.normalize_column_widths();
        if let Some(width) = self.column_widths.get_mut(column) {
            *width = (*width + dx).max(MIN_CELL_WIDTH);
        }
    }

    fn column_width(&self, column: usize) -> f32 {
        self.column_widths
            .get(column)
            .copied()
            .unwrap_or(CELL_WIDTH)
    }

    fn grid_width(&self) -> f32 {
        ROW_ACTION_WIDTH + ROW_HEADER_WIDTH + self.column_widths.iter().sum::<f32>()
    }

    fn started_editing(&mut self) {
        self.edit_generation += 1;
        self.save_state = SaveState::Idle;
    }

    fn set_active_cell(&mut self, row: usize, column: usize) {
        self.active_row = row.min(self.rows.len().saturating_sub(1));
        self.active_column = column.min(self.column_count().saturating_sub(1));
        self.selected_row = None;
        self.selected_column = None;
    }

    fn move_active_cell(&mut self, row_delta: isize, column_delta: isize) {
        let row = self
            .active_row
            .saturating_add_signed(row_delta)
            .min(self.rows.len().saturating_sub(1));
        let column = self
            .active_column
            .saturating_add_signed(column_delta)
            .min(self.column_count().saturating_sub(1));
        self.active_row = row;
        self.active_column = column;
    }

    fn cell_mut(&mut self, row: usize, column: usize) -> &mut String {
        &mut self.rows[row][column]
    }

    fn column_count(&self) -> usize {
        self.rows.first().map_or(DEFAULT_COLUMNS, Vec::len)
    }

    fn normalize_shape(&mut self) {
        if self.rows.is_empty() {
            self.rows.push(Vec::new());
        }

        let column_count = self
            .rows
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(DEFAULT_COLUMNS)
            .max(DEFAULT_COLUMNS);
        for row in &mut self.rows {
            row.resize(column_count, String::new());
        }

        while self.rows.len() < DEFAULT_ROWS {
            self.rows.push(vec![String::new(); column_count]);
        }

        self.normalize_column_widths();
    }

    fn normalize_column_widths(&mut self) {
        let column_count = self.column_count();
        self.column_widths.resize(column_count, CELL_WIDTH);
        for width in &mut self.column_widths {
            *width = (*width).max(MIN_CELL_WIDTH);
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

pub fn save_spreadsheet_file(save_request: SaveRequest) -> io::Result<PathBuf> {
    let spreadsheets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spreadsheets");
    std::fs::create_dir_all(&spreadsheets_dir)?;

    let file_slug = spreadsheet_name_slug(&save_request.storage.name).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "spreadsheet name must contain at least one letter or number",
        )
    })?;
    let path = spreadsheets_dir.join(format!("{file_slug}.csv"));
    std::fs::write(&path, storage_to_csv(&save_request.storage))?;

    Ok(path)
}

pub fn delete_spreadsheet_file(delete_request: DeleteRequest) -> io::Result<()> {
    let Some(path) = delete_request.path else {
        return Ok(());
    };

    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub fn load_spreadsheet_file(path: &Path) -> io::Result<Storage> {
    let contents = std::fs::read_to_string(path)?;
    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("spreadsheet")
        .replace('-', " ");

    Ok(Storage {
        name,
        rows: csv_to_rows(&contents),
        column_widths: Vec::new(),
    })
}

fn storage_to_csv(storage: &Storage) -> String {
    let mut csv = String::new();
    for (row_index, row) in storage.rows.iter().enumerate() {
        if row_index > 0 {
            csv.push('\n');
        }
        for (column_index, cell) in row.iter().enumerate() {
            if column_index > 0 {
                csv.push(',');
            }
            csv.push_str(&escape_csv_cell(cell));
        }
    }
    csv
}

fn escape_csv_cell(cell: &str) -> String {
    if cell.contains([',', '"', '\n']) {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_string()
    }
}

fn csv_to_rows(contents: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut chars = contents.chars().peekable();
    let mut in_quotes = false;

    while let Some(character) = chars.next() {
        match character {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                row.push(std::mem::take(&mut cell));
            }
            '\n' if !in_quotes => {
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
            }
            '\r' if !in_quotes => {}
            _ => cell.push(character),
        }
    }

    row.push(cell);
    rows.push(row);
    rows
}

fn spreadsheet_name_slug(spreadsheet_name: &str) -> Option<String> {
    let mut slug = String::new();
    let mut previous_was_separator = false;

    for character in spreadsheet_name.chars().flat_map(char::to_lowercase) {
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
    spreadsheet: &Model,
    focus_handle: &FocusHandle,
    pressed_button: Option<&ButtonId>,
    active_cell: Option<&CellFieldId>,
    show_name_cursor: bool,
    is_focused: bool,
    cx: &mut Context<T>,
) -> gpui::Div
where
    T: EventEmitter<IdEvent> + 'static,
{
    let emitter = IdEmitter {
        spreadsheet_id: spreadsheet.id,
    };
    let header_focus_handle = focus_handle.clone();
    let close_button_pressed = pressed_button == Some(&ButtonId::X);
    let save_button_pressed = pressed_button == Some(&ButtonId::Save);
    let rename_button_pressed = pressed_button == Some(&ButtonId::Rename);
    let delete_button_pressed = pressed_button == Some(&ButtonId::Delete);
    let confirm_delete_button_pressed = pressed_button == Some(&ButtonId::ConfirmDelete);
    let cancel_delete_button_pressed = pressed_button == Some(&ButtonId::CancelDelete);
    let confirm_delete = pressed_button.is_some_and(|button_id| {
        matches!(
            button_id,
            ButtonId::Delete | ButtonId::ConfirmDelete | ButtonId::CancelDelete
        )
    });

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
                    .flex_none()
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
                    .child(spreadsheet.name.clone())
                    .child(close_button(emitter, close_button_pressed, cx)),
            )
            .child(rename_row(
                emitter,
                spreadsheet,
                focus_handle,
                show_name_cursor && is_focused,
                rename_button_pressed,
                cx,
            ))
            .child(
                gpui::div()
                    .p(s::S3)
                    .pt(s::S0)
                    .flex_1()
                    .min_w(gpui::px(0.0))
                    .min_h(gpui::px(0.0))
                    .overflow_hidden()
                    .bg(s::GRAY2)
                    .child(
                        s::sunken(
                            gpui::div()
                                .size_full()
                                .id(("spreadsheet-grid", spreadsheet.id.0))
                                .overflow_scroll()
                                .scrollbar_width(s::S3)
                                .bg(s::GREEN3)
                                .track_focus(focus_handle)
                                .key_context("SpreadsheetEditor")
                                .on_key_down(cx.listener(move |_, event, _, cx| {
                                    emitted_key_event(emitter, event, cx);
                                }))
                                .child(render_grid(
                                    spreadsheet,
                                    focus_handle,
                                    active_cell,
                                    is_focused,
                                    emitter,
                                    cx,
                                )),
                        )
                        .size_full()
                        .overflow_hidden(),
                    ),
            )
            .child(action_row(
                emitter,
                spreadsheet,
                save_button_pressed,
                delete_button_pressed,
                confirm_delete_button_pressed,
                cancel_delete_button_pressed,
                confirm_delete,
                cx,
            )),
    )
    .overflow_hidden()
    .child(resize_handle(emitter, focus_handle, cx))
}

#[derive(Clone, Copy)]
struct IdEmitter {
    spreadsheet_id: SpreadsheetId,
}

impl IdEmitter {
    fn emit<T>(self, cx: &mut Context<T>, event: Event)
    where
        T: EventEmitter<IdEvent>,
    {
        cx.emit(IdEvent {
            spreadsheet_id: self.spreadsheet_id,
            event,
        });
    }
}

fn rename_row<T>(
    emitter: IdEmitter,
    spreadsheet: &Model,
    focus_handle: &FocusHandle,
    show_name_cursor: bool,
    rename_button_pressed: bool,
    cx: &mut Context<T>,
) -> impl IntoElement
where
    T: EventEmitter<IdEvent> + 'static,
{
    let name_focus_handle = focus_handle.clone();
    let rename_button_focus_handle = focus_handle.clone();
    let rename_control = match &spreadsheet.renaming {
        RenamingState::Renaming { name_field } => {
            let name_field_with_cursor = if show_name_cursor {
                format!("{name_field}|")
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
                            .key_context("SpreadsheetNameEditor")
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
            view::button::from_text("rename", rename_button_pressed)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |_, _: &MouseDownEvent, window, cx| {
                        window.focus(&rename_button_focus_handle);
                        cx.stop_propagation();
                        emitter.emit(
                            cx,
                            Event::PressedButton {
                                button_id: ButtonId::Rename,
                            },
                        );
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(move |_, _: &MouseUpEvent, _, cx| {
                        cx.stop_propagation();
                        emitter.emit(cx, Event::ClickedRename);
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(move |_, _: &MouseUpEvent, _, cx| {
                        emitter.emit(cx, Event::ReleasedButton);
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

fn render_grid<T>(
    spreadsheet: &Model,
    focus_handle: &FocusHandle,
    active_cell: Option<&CellFieldId>,
    is_focused: bool,
    emitter: IdEmitter,
    cx: &mut Context<T>,
) -> gpui::Div
where
    T: EventEmitter<IdEvent> + 'static,
{
    let grid_width = spreadsheet.grid_width();
    let grid_height = (spreadsheet.rows.len() + 2) as f32 * CELL_HEIGHT;
    let mut column_action_cells = Vec::new();
    for column in 0..spreadsheet.column_count() {
        let column_action_focus_handle = focus_handle.clone();
        let selected = spreadsheet.selected_column == Some(column);
        let column_width = spreadsheet.column_width(column);
        column_action_cells.push(
            gpui::div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .w(gpui::px(column_width))
                .min_w(gpui::px(column_width))
                .max_w(gpui::px(column_width))
                .h(gpui::px(CELL_HEIGHT))
                .min_h(gpui::px(CELL_HEIGHT))
                .max_h(gpui::px(CELL_HEIGHT))
                .bg(if selected { s::GREEN4 } else { s::GREEN1 })
                .border_1()
                .border_color(s::GREEN5)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |_, _: &MouseDownEvent, window, cx| {
                        window.focus(&column_action_focus_handle);
                        cx.stop_propagation();
                        emitter.emit(cx, Event::PressedColumnHeader { column });
                    }),
                )
                .children(if selected {
                    vec![
                        grid_action_button(
                            "<+",
                            emitter,
                            Event::ClickedInsertColumnLeft { column },
                            cx,
                        ),
                        grid_action_button(
                            "+>",
                            emitter,
                            Event::ClickedInsertColumnRight { column },
                            cx,
                        ),
                        grid_action_button("X", emitter, Event::ClickedDeleteColumn { column }, cx),
                    ]
                } else {
                    Vec::new()
                }),
        );
    }

    let mut header_cells = Vec::new();
    for column in 0..spreadsheet.column_count() {
        let focus_handle = focus_handle.clone();
        let column_width = spreadsheet.column_width(column);
        header_cells.push(
            gpui::div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .relative()
                .w(gpui::px(column_width))
                .min_w(gpui::px(column_width))
                .max_w(gpui::px(column_width))
                .h(gpui::px(CELL_HEIGHT))
                .min_h(gpui::px(CELL_HEIGHT))
                .max_h(gpui::px(CELL_HEIGHT))
                .bg(s::GREEN2)
                .text_color(s::GRAY6)
                .border_1()
                .border_color(s::GREEN5)
                .child(column_label(column))
                .child(column_resize_handle(column, emitter, &focus_handle, cx))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |_, _: &MouseDownEvent, window, cx| {
                        window.focus(&focus_handle);
                        cx.stop_propagation();
                        emitter.emit(cx, Event::PressedColumnHeader { column });
                    }),
                ),
        );
    }

    let mut row_elements = Vec::new();
    for (row_index, row) in spreadsheet.rows.iter().enumerate() {
        let row_focus_handle = focus_handle.clone();
        let mut cell_elements = Vec::new();
        for (column_index, cell) in row.iter().enumerate() {
            cell_elements.push(render_cell(
                spreadsheet.id,
                row_index,
                column_index,
                cell,
                spreadsheet.column_width(column_index),
                focus_handle,
                active_cell,
                is_focused,
                emitter,
                cx,
            ));
        }

        row_elements.push(
            gpui::div()
                .flex()
                .flex_none()
                .child(row_action_cell(
                    spreadsheet.selected_row == Some(row_index),
                    row_index,
                    focus_handle,
                    emitter,
                    cx,
                ))
                .child(
                    gpui::div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_center()
                        .w(gpui::px(ROW_HEADER_WIDTH))
                        .min_w(gpui::px(ROW_HEADER_WIDTH))
                        .max_w(gpui::px(ROW_HEADER_WIDTH))
                        .h(gpui::px(CELL_HEIGHT))
                        .min_h(gpui::px(CELL_HEIGHT))
                        .max_h(gpui::px(CELL_HEIGHT))
                        .bg(s::GREEN2)
                        .text_color(s::GRAY6)
                        .border_1()
                        .border_color(s::GREEN5)
                        .child((row_index + 1).to_string())
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |_, _: &MouseDownEvent, window, cx| {
                                window.focus(&row_focus_handle);
                                cx.stop_propagation();
                                emitter.emit(cx, Event::PressedRowHeader { row: row_index });
                            }),
                        ),
                )
                .children(cell_elements),
        );
    }

    gpui::div()
        .flex()
        .flex_none()
        .flex_col()
        .w(gpui::px(grid_width))
        .min_w(gpui::px(grid_width))
        .max_w(gpui::px(grid_width))
        .h(gpui::px(grid_height))
        .min_h(gpui::px(grid_height))
        .max_h(gpui::px(grid_height))
        .child(
            gpui::div()
                .flex()
                .flex_none()
                .child(grid_corner_cell(
                    ROW_ACTION_WIDTH + ROW_HEADER_WIDTH,
                    s::GREEN1,
                    focus_handle,
                    cx,
                ))
                .children(column_action_cells),
        )
        .child(
            gpui::div()
                .flex()
                .flex_none()
                .child(grid_corner_cell(
                    ROW_ACTION_WIDTH,
                    s::GREEN2,
                    focus_handle,
                    cx,
                ))
                .child(grid_corner_cell(
                    ROW_HEADER_WIDTH,
                    s::GREEN2,
                    focus_handle,
                    cx,
                ))
                .children(header_cells),
        )
        .children(row_elements)
}

fn render_cell<T>(
    spreadsheet_id: SpreadsheetId,
    row: usize,
    column: usize,
    cell: &str,
    width: f32,
    focus_handle: &FocusHandle,
    active_cell: Option<&CellFieldId>,
    is_focused: bool,
    emitter: IdEmitter,
    cx: &mut Context<T>,
) -> gpui::Div
where
    T: EventEmitter<IdEvent>,
{
    let is_active = active_cell
        .map(|active| {
            active.spreadsheet_id == spreadsheet_id && active.row == row && active.column == column
        })
        .unwrap_or(false);
    let focus_handle = focus_handle.clone();
    let text = if is_active && is_focused {
        format!("{cell}|")
    } else if cell.is_empty() {
        " ".to_string()
    } else {
        cell.to_string()
    };

    let bg = if is_active { s::GREEN3 } else { s::GREEN2 };
    let border = if is_active { s::YELLOW6 } else { s::GREEN5 };

    gpui::div()
        .flex()
        .flex_none()
        .items_center()
        .w(gpui::px(width))
        .min_w(gpui::px(width))
        .max_w(gpui::px(width))
        .h(gpui::px(CELL_HEIGHT))
        .min_h(gpui::px(CELL_HEIGHT))
        .max_h(gpui::px(CELL_HEIGHT))
        .px(s::S3)
        .truncate()
        .bg(bg)
        .text_color(s::GRAY6)
        .border_1()
        .border_color(border)
        .child(text)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |_, _: &MouseDownEvent, window, cx| {
                window.focus(&focus_handle);
                cx.stop_propagation();
                emitter.emit(cx, Event::PressedCell { row, column });
            }),
        )
}

fn row_action_cell<T>(
    selected: bool,
    row: usize,
    focus_handle: &FocusHandle,
    emitter: IdEmitter,
    cx: &mut Context<T>,
) -> gpui::Div
where
    T: EventEmitter<IdEvent>,
{
    let focus_handle = focus_handle.clone();
    let actions = if selected {
        vec![
            grid_action_button("^+", emitter, Event::ClickedInsertRowAbove { row }, cx),
            grid_action_button("v+", emitter, Event::ClickedInsertRowBelow { row }, cx),
            grid_action_button("X", emitter, Event::ClickedDeleteRow { row }, cx),
        ]
    } else {
        Vec::new()
    };

    gpui::div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .w(gpui::px(ROW_ACTION_WIDTH))
        .min_w(gpui::px(ROW_ACTION_WIDTH))
        .max_w(gpui::px(ROW_ACTION_WIDTH))
        .h(gpui::px(CELL_HEIGHT))
        .min_h(gpui::px(CELL_HEIGHT))
        .max_h(gpui::px(CELL_HEIGHT))
        .bg(if selected { s::GREEN4 } else { s::GREEN1 })
        .border_1()
        .border_color(s::GREEN5)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |_, _: &MouseDownEvent, window, cx| {
                window.focus(&focus_handle);
                cx.stop_propagation();
                emitter.emit(cx, Event::PressedRowHeader { row });
            }),
        )
        .children(actions)
}

fn grid_corner_cell<T>(
    width: f32,
    bg: gpui::Rgba,
    focus_handle: &FocusHandle,
    cx: &mut Context<T>,
) -> gpui::Div
where
    T: 'static,
{
    let focus_handle = focus_handle.clone();
    gpui::div()
        .flex_none()
        .w(gpui::px(width))
        .min_w(gpui::px(width))
        .max_w(gpui::px(width))
        .h(gpui::px(CELL_HEIGHT))
        .min_h(gpui::px(CELL_HEIGHT))
        .max_h(gpui::px(CELL_HEIGHT))
        .bg(bg)
        .border_1()
        .border_color(s::GREEN5)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |_, _: &MouseDownEvent, window, cx| {
                window.focus(&focus_handle);
                cx.stop_propagation();
            }),
        )
}

fn column_resize_handle<T>(
    column: usize,
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
        .top_0()
        .bottom_0()
        .w(s::S4)
        .cursor_col_resize()
        .hover(|handle| handle.bg(s::GREEN5))
        .child(
            gpui::canvas(
                |_, _, _| {},
                |bounds, _, window, _| {
                    let mut builder = gpui::PathBuilder::stroke(s::S1);
                    builder.move_to(gpui::point(bounds.right() - s::S2, bounds.top() + s::S3));
                    builder.line_to(gpui::point(bounds.right() - s::S2, bounds.bottom() - s::S3));
                    if let Ok(path) = builder.build() {
                        window.paint_path(path, s::GRAY4);
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
                    Event::PressedColumnResizeHandle {
                        column,
                        x: event.position.x.into(),
                    },
                );
            }),
        )
}

fn grid_action_button<T>(
    label: &'static str,
    emitter: IdEmitter,
    event: Event,
    cx: &mut Context<T>,
) -> gpui::Div
where
    T: EventEmitter<IdEvent>,
{
    gpui::div()
        .flex()
        .items_center()
        .justify_center()
        .min_w(s::S6)
        .h(gpui::px(CELL_HEIGHT - 4.0))
        .px(s::S2)
        .text_color(s::GRAY6)
        .cursor_pointer()
        .hover(|button| button.bg(s::GREEN5))
        .child(label)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_, _: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
            }),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(move |_, _: &MouseUpEvent, _, cx| {
                cx.stop_propagation();
                emitter.emit(cx, event.clone());
            }),
        )
}

fn action_row<T>(
    emitter: IdEmitter,
    spreadsheet: &Model,
    save_button_pressed: bool,
    delete_button_pressed: bool,
    confirm_delete_button_pressed: bool,
    cancel_delete_button_pressed: bool,
    confirm_delete: bool,
    cx: &mut Context<T>,
) -> impl IntoElement
where
    T: EventEmitter<IdEvent>,
{
    let status = match spreadsheet.save_state {
        SaveState::Idle => "",
        SaveState::Saving => "saving...",
        SaveState::Saved => "saved csv",
        SaveState::Failed => "save failed",
    };

    let actions = if confirm_delete {
        gpui::div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                view::button::from_text("cancel", cancel_delete_button_pressed)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |_, _: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            emitter.emit(
                                cx,
                                Event::PressedButton {
                                    button_id: ButtonId::CancelDelete,
                                },
                            );
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |_, _: &MouseUpEvent, _, cx| {
                            cx.stop_propagation();
                            emitter.emit(cx, Event::ClickedCancelDeleteButton);
                        }),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(move |_, _: &MouseUpEvent, _, cx| {
                            emitter.emit(cx, Event::ReleasedButton);
                        }),
                    ),
            )
            .child(
                view::button::from_text("delete forever", confirm_delete_button_pressed)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |_, _: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            emitter.emit(
                                cx,
                                Event::PressedButton {
                                    button_id: ButtonId::ConfirmDelete,
                                },
                            );
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |_, _: &MouseUpEvent, _, cx| {
                            cx.stop_propagation();
                            emitter.emit(cx, Event::ClickedConfirmDeleteButton);
                        }),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(move |_, _: &MouseUpEvent, _, cx| {
                            emitter.emit(cx, Event::ReleasedButton);
                        }),
                    ),
            )
    } else {
        gpui::div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                view::button::from_text("delete", delete_button_pressed)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |_, _: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            emitter.emit(
                                cx,
                                Event::PressedButton {
                                    button_id: ButtonId::Delete,
                                },
                            );
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |_, _: &MouseUpEvent, _, cx| {
                            cx.stop_propagation();
                            emitter.emit(cx, Event::ClickedDeleteButton);
                        }),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(move |_, _: &MouseUpEvent, _, cx| {
                            emitter.emit(cx, Event::ReleasedButton);
                        }),
                    ),
            )
            .child(
                view::button::from_text("save", save_button_pressed)
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
    };

    gpui::div()
        .flex()
        .flex_none()
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
        .child(actions)
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
        "s" if event.keystroke.modifiers.platform => KeyPress::Save,
        "v" if event.keystroke.modifiers.platform => KeyPress::Paste,
        "backspace" if event.keystroke.modifiers.platform => KeyPress::CommandBackspace,
        "backspace" if event.keystroke.modifiers.alt => KeyPress::OptionBackspace,
        "backspace" => KeyPress::Backspace,
        "enter" if event.keystroke.modifiers.shift => KeyPress::ShiftEnter,
        "enter" => KeyPress::Enter,
        "tab" if event.keystroke.modifiers.shift => KeyPress::ShiftTab,
        "tab" => KeyPress::Tab,
        "up" => KeyPress::ArrowUp,
        "down" => KeyPress::ArrowDown,
        "left" => KeyPress::ArrowLeft,
        "right" => KeyPress::ArrowRight,
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

fn column_label(mut column: usize) -> String {
    let mut label = String::new();
    loop {
        let letter = (b'A' + (column % 26) as u8) as char;
        label.insert(0, letter);
        column /= 26;
        if column == 0 {
            break;
        }
        column -= 1;
    }
    label
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_csv_cells_that_need_quotes() {
        let storage = Storage {
            name: "spreadsheet".to_string(),
            rows: vec![vec![
                "plain".to_string(),
                "has,comma".to_string(),
                "has \"quote\"".to_string(),
                "two\nlines".to_string(),
            ]],
            column_widths: Vec::new(),
        };

        assert_eq!(
            storage_to_csv(&storage),
            "plain,\"has,comma\",\"has \"\"quote\"\"\",\"two\nlines\""
        );
    }

    #[test]
    fn parses_quoted_csv_cells() {
        assert_eq!(
            csv_to_rows("plain,\"has,comma\",\"has \"\"quote\"\"\",\"two\nlines\""),
            vec![vec![
                "plain".to_string(),
                "has,comma".to_string(),
                "has \"quote\"".to_string(),
                "two\nlines".to_string(),
            ]]
        );
    }
}
