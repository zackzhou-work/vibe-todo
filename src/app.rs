use crate::db::Database;
use crate::icons::*;
use crate::model::{ColumnType, Task, WindowState};
use crate::theme::*;
use crate::window_level::set_window_always_on_top;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    actions, div, ease_out_quint, px, relative, rgb, Animation, AnimationExt as _, AnyElement,
    AppContext as _, Bounds, ClickEvent, Context, Entity, FocusHandle, Focusable as _, FontWeight,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, Pixels, Render,
    ScrollHandle, SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Window,
    WindowBounds,
};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::tooltip::Tooltip;
use std::collections::HashMap;
use std::time::Duration;

actions!(
    vibe_todo,
    [NewTask, TogglePin, ToggleHistory, Cancel, DeleteHovered]
);

/// Travels with a dragged row: where it came from and what to show under the
/// cursor.
#[derive(Clone)]
pub struct DraggedTask {
    pub id: String,
    pub title: SharedString,
}

/// The card that follows the cursor during a drag.
pub struct DragGhost {
    title: SharedString,
}

impl Render for DragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .max_w(px(240.))
            .px(px(10.))
            .py(px(7.))
            .rounded(px(8.))
            .bg(rgb(RAISED))
            .shadow(ghost_shadow())
            .font_family(".SystemUIFont")
            .text_size(px(13.))
            .text_color(rgb(INK))
            .whitespace_nowrap()
            .overflow_hidden()
            .text_ellipsis()
            .child(self.title.clone())
    }
}

/// Where a checked row is in its two-step exit.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Completing {
    /// Struck through and holding still. Clicking the box again puts it back.
    Held,
    /// Collapsing out of the list. Past the point of return.
    Leaving,
}

/// Long enough to read the strike-through, and the whole of the window in which
/// a mis-click can be taken back.
const COMPLETION_HOLD: Duration = Duration::from_millis(240);
/// The row folding shut afterwards, which commits the write.
const COMPLETION_COLLAPSE: Duration = Duration::from_millis(160);
/// The checkbox filling in under the cursor.
const TICK: Duration = Duration::from_millis(140);
/// An armed destructive button forgets it was armed after this, so a click
/// landing minutes later cannot finish something started by accident.
const CONFIRM_TIMEOUT: Duration = Duration::from_secs(3);
/// Geometry is read on every frame; this is how long a resize has to settle
/// before it reaches the database.
const BOUNDS_SETTLE: Duration = Duration::from_millis(500);

/// The OS owns the top-left corner, so the top strip stays clear for the
/// traffic lights. It carries no surface and no rule of its own — the glass
/// runs straight through it into the window.
const CHROME_HEIGHT: f32 = 34.;
const GUTTER: f32 = 12.;
const ROW_RADIUS: f32 = 6.;
const ROW_HEIGHT: f32 = 36.;

/// The tray's left inset doubles as the length of its fade: wide enough that a
/// couple of characters dissolve instead of being cut mid-stroke, and no wider,
/// so the first button still lands on solid colour.
const TRAY_FADE: f32 = 24.;

const SECTION_HEADER_HEIGHT: f32 = 28.;
/// How far the label is indented, which is also where the checkbox column
/// starts: heading and rows are read as one left edge or the eye catches it.
const LABEL_INDENT: f32 = 16.;

pub struct VibeTodoApp {
    db: Database,
    inbox_tasks: Vec<Task>,
    ongoing_tasks: Vec<Task>,
    completed_tasks: Vec<Task>,
    input: Entity<InputState>,
    /// Renaming gets its own field rather than a mode flag on the one above:
    /// an unsaved new task and an edit to a saved one end differently, so they
    /// do not share a buffer.
    edit_input: Entity<InputState>,
    pub is_pinned: bool,
    pub is_history_open: bool,
    pub is_creating: bool,
    /// Whose title is open for editing.
    editing_id: Option<String>,
    /// The row under the pointer. Only ⌘⌫ reads it; hover styling is still the
    /// framework's own.
    hovered_id: Option<String>,
    /// A delete armed by a first click, waiting for the second.
    confirm_delete: Option<String>,
    /// Rows that have been checked and are playing out their exit.
    completing: HashMap<String, Completing>,
    /// "Clear all" is armed by a first click and fires on the second.
    confirm_clear: bool,
    last_bounds: Option<Bounds<Pixels>>,
    bounds_write_queued: bool,
    /// The window scrolls as one, so a long ongoing list can push the inbox off
    /// the bottom. This is how ⌘N gets the field back on screen.
    body_scroll: ScrollHandle,
    pub focus_handle: FocusHandle,
    _input_sub: Subscription,
    _edit_sub: Subscription,
}

impl VibeTodoApp {
    pub fn new(db: Database, is_pinned: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let inbox_tasks = db.get_active_tasks(ColumnType::Inbox).unwrap_or_default();
        let ongoing_tasks = db.get_active_tasks(ColumnType::Ongoing).unwrap_or_default();
        let completed_tasks = db.get_completed_tasks().unwrap_or_default();

        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("记一笔，回车保存")
                .submit_on_enter(true)
        });
        let edit_input = cx.new(|cx| InputState::new(window, cx).submit_on_enter(true));

        let input_sub =
            cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.commit_new_task(window, cx);
                }
            });
        let edit_sub = cx.subscribe_in(
            &edit_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.commit_edit(window, cx);
                }
            },
        );

        // Actions dispatch along the focus path, so the root has to hold focus
        // from the first frame or none of the shortcuts have anywhere to land.
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);

        Self {
            db,
            inbox_tasks,
            ongoing_tasks,
            completed_tasks,
            input,
            edit_input,
            is_pinned,
            is_history_open: false,
            is_creating: false,
            editing_id: None,
            hovered_id: None,
            confirm_delete: None,
            completing: HashMap::new(),
            confirm_clear: false,
            last_bounds: None,
            bounds_write_queued: false,
            body_scroll: ScrollHandle::new(),
            focus_handle,
            _input_sub: input_sub,
            _edit_sub: edit_sub,
        }
    }

    pub fn refresh_tasks(&mut self) {
        if let Ok(tasks) = self.db.get_active_tasks(ColumnType::Inbox) {
            self.inbox_tasks = tasks;
        }
        if let Ok(tasks) = self.db.get_active_tasks(ColumnType::Ongoing) {
            self.ongoing_tasks = tasks;
        }
        if let Ok(tasks) = self.db.get_completed_tasks() {
            self.completed_tasks = tasks;
        }
    }

    pub fn toggle_pin(&mut self, cx: &mut Context<Self>) {
        self.is_pinned = !self.is_pinned;
        set_window_always_on_top(self.is_pinned);
        self.persist_window_state();
        cx.notify();
    }

    pub fn toggle_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Opening a panel finishes whatever was half-written: a new task is
        // dropped, a rename is kept.
        self.commit_edit(window, cx);
        if self.is_creating {
            self.cancel_new_task(window, cx);
        }
        self.is_history_open = !self.is_history_open;
        self.confirm_clear = false;
        self.confirm_delete = None;
        // The scrim covers the rows, so nothing under the pointer is reachable
        // any more and ⌘⌫ must not still be aimed at one.
        self.hovered_id = None;
        if self.is_history_open {
            self.refresh_tasks();
        }
        cx.notify();
    }

    pub fn start_creating(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Re-entering while the field is already open would wipe what is in it.
        if self.is_creating {
            return;
        }
        self.commit_edit(window, cx);
        self.is_creating = true;
        self.is_history_open = false;
        self.input
            .update(cx, |state, cx| state.set_value("", window, cx));
        // The field opens at the top of the inbox, which may be scrolled well
        // past the bottom edge. Asking for the inbox is enough to reveal it,
        // and gpui holds the request until the row it needs has been laid out.
        self.body_scroll.scroll_to_item(BODY_INBOX_INDEX);
        let handle = self.input.focus_handle(cx);
        handle.focus(window, cx);
        cx.notify();
    }

    pub fn commit_new_task(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = self.input.read(cx).value().trim().to_string();
        if !title.is_empty() {
            let _ = self.db.insert_task(&title, false, ColumnType::Inbox);
            self.refresh_tasks();
        }
        // One task per ⌘N. The field used to stay open for a second thought,
        // but it is rare to have one, and an open field left behind is the
        // more common cost.
        self.input
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.cancel_new_task(window, cx);
    }

    pub fn cancel_new_task(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.is_creating = false;
        self.focus_handle.clone().focus(window, cx);
        cx.notify();
    }

    /// Opens a title for editing. Clicking away keeps what was typed, the same
    /// as the new-task field.
    pub fn start_editing(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing_id.as_deref() == Some(id) {
            return;
        }
        self.commit_edit(window, cx);
        if self.is_creating {
            self.cancel_new_task(window, cx);
        }
        let Some(title) = self.find_task_title(id) else {
            return;
        };
        self.is_history_open = false;
        self.confirm_delete = None;
        self.editing_id = Some(id.to_string());
        self.edit_input
            .update(cx, |state, cx| state.set_value(title, window, cx));
        let handle = self.edit_input.focus_handle(cx);
        handle.focus(window, cx);
        cx.notify();
    }

    pub fn commit_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.editing_id.take() else {
            return;
        };
        let title = self.edit_input.read(cx).value().trim().to_string();
        // Emptying a title is not a way to delete: the old one stands.
        if !title.is_empty() {
            let _ = self.db.update_task_title(&id, &title);
            self.refresh_tasks();
        }
        self.focus_handle.clone().focus(window, cx);
        cx.notify();
    }

    pub fn cancel_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editing_id = None;
        self.focus_handle.clone().focus(window, cx);
        cx.notify();
    }

    fn find_task_title(&self, id: &str) -> Option<String> {
        self.inbox_tasks
            .iter()
            .chain(self.ongoing_tasks.iter())
            .find(|task| task.id == id)
            .map(|task| task.title.clone())
    }

    /// Marks the row done on screen, holds it there long enough to be taken
    /// back, then folds it away and writes it.
    pub fn complete_task(&mut self, id: &str, cx: &mut Context<Self>) {
        match self.completing.get(id) {
            // Still holding: a second click is a change of mind.
            Some(Completing::Held) => {
                self.completing.remove(id);
                cx.notify();
                return;
            }
            Some(Completing::Leaving) => return,
            None => {}
        }
        self.completing.insert(id.to_string(), Completing::Held);
        cx.notify();

        let id = id.to_string();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COMPLETION_HOLD).await;
            let still_held = this
                .update(cx, |this, cx| {
                    if this.completing.get(&id) != Some(&Completing::Held) {
                        return false;
                    }
                    this.completing.insert(id.clone(), Completing::Leaving);
                    cx.notify();
                    true
                })
                .unwrap_or(false);
            if !still_held {
                return;
            }

            cx.background_executor().timer(COMPLETION_COLLAPSE).await;
            let _ = this.update(cx, |this, cx| {
                this.completing.remove(&id);
                let _ = this.db.complete_task(&id);
                this.refresh_tasks();
                cx.notify();
            });
        })
        .detach();
    }

    pub fn restore_task(&mut self, id: &str, cx: &mut Context<Self>) {
        let _ = self.db.restore_task(id);
        self.refresh_tasks();
        cx.notify();
    }

    /// The first click arms; the second deletes. Bound to the keyboard the
    /// arming step is dropped — reaching for ⌘⌫ is already deliberate in a way
    /// that brushing a 21px button is not.
    pub fn request_delete(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.confirm_delete.as_deref() == Some(id) {
            self.confirm_delete = None;
            self.delete_task(id, cx);
            return;
        }
        self.confirm_delete = Some(id.to_string());
        cx.notify();

        let id = id.to_string();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(CONFIRM_TIMEOUT).await;
            let _ = this.update(cx, |this, cx| {
                if this.confirm_delete.as_deref() == Some(id.as_str()) {
                    this.confirm_delete = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub fn delete_task(&mut self, id: &str, cx: &mut Context<Self>) {
        let _ = self.db.delete_task(id);
        if self.hovered_id.as_deref() == Some(id) {
            self.hovered_id = None;
        }
        self.refresh_tasks();
        cx.notify();
    }

    pub fn reposition_task(
        &mut self,
        id: &str,
        column: ColumnType,
        before: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        let _ = self.db.reposition_task(id, column, before);
        self.refresh_tasks();
        cx.notify();
    }

    pub fn move_task(&mut self, id: &str, target: ColumnType, cx: &mut Context<Self>) {
        let _ = self.db.move_task_column(id, target);
        self.refresh_tasks();
        cx.notify();
    }

    pub fn toggle_priority(&mut self, id: &str, cx: &mut Context<Self>) {
        let _ = self.db.toggle_task_priority(id);
        self.refresh_tasks();
        cx.notify();
    }

    pub fn clear_completed(&mut self, cx: &mut Context<Self>) {
        if !self.confirm_clear {
            self.confirm_clear = true;
            cx.notify();

            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(CONFIRM_TIMEOUT).await;
                let _ = this.update(cx, |this, cx| {
                    if this.confirm_clear {
                        this.confirm_clear = false;
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        let _ = self.db.clear_completed_tasks();
        self.confirm_clear = false;
        self.refresh_tasks();
        cx.notify();
    }

    /// The window remembers where it was. gpui has no resize callback, so the
    /// geometry is read off each frame and written once the movement stops.
    fn note_window_bounds(&mut self, window: &Window, cx: &mut Context<Self>) {
        let WindowBounds::Windowed(bounds) = window.window_bounds() else {
            return;
        };
        if self.last_bounds == Some(bounds) {
            return;
        }
        self.last_bounds = Some(bounds);
        if self.bounds_write_queued {
            return;
        }
        self.bounds_write_queued = true;

        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(BOUNDS_SETTLE).await;
            let _ = this.update(cx, |this, _| {
                this.bounds_write_queued = false;
                this.persist_window_state();
            });
        })
        .detach();
    }

    fn persist_window_state(&self) {
        let Some(bounds) = self.last_bounds else {
            return;
        };
        let _ = self.db.save_window_state(&WindowState {
            x: f32::from(bounds.origin.x),
            y: f32::from(bounds.origin.y),
            width: f32::from(bounds.size.width),
            height: f32::from(bounds.size.height),
            is_pinned: self.is_pinned,
        });
    }

    fn is_editing(&self) -> bool {
        self.is_creating || self.editing_id.is_some()
    }
}

/// A square hover action: move, delete, priority, restore.
///
/// Pressing has to show up the instant the button goes down. gpui has no
/// transforms, so the only channel for it is one more step of colour.
fn action_button(
    id: impl Into<SharedString>,
    size: f32,
    skin: Surface,
    filled: bool,
    icon: gpui::Svg,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id.into())
        .w(px(size))
        .h(px(size))
        .rounded(px(5.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .when(filled, |s| s.bg(rgb(skin.action_bg)))
        .hover(|s| s.bg(rgb(skin.action_bg_hover)))
        .active(|s| s.bg(rgb(skin.action_bg_active)))
        .child(icon)
}

/// Marks exactly where a dragged row will land: a ring on the left end of a
/// straight rule, sitting in the gap above the row it will be inserted before.
///
/// Drawn as a real element rather than a border on the row itself, because a
/// border follows the row's corner radius and bows at both ends.
fn drop_indicator(group: &'static str) -> impl IntoElement {
    div()
        .absolute()
        .top(px(-3.5))
        .left(px(4.))
        .right(px(6.))
        .h(px(7.))
        .flex()
        .flex_row()
        .items_center()
        .invisible()
        // gpui only allocates a hitbox for an element that carries `drag_over`
        // styles of its own; a `group_drag_over` rule alone does not earn one,
        // and without a hitbox the rule below is never evaluated. This no-op
        // refinement is what makes the element eligible.
        .drag_over::<DraggedTask>(|s, _, _, _| s)
        .group_drag_over::<DraggedTask>(group, |s| s.visible())
        .child(
            div()
                .w(px(7.))
                .h(px(7.))
                .flex_none()
                .rounded_full()
                .border_1()
                .border_color(rgb(INK)),
        )
        .child(div().flex_1().h(px(1.5)).bg(rgb(INK)))
}

/// Names the section and says how much is in it. The two headings are the
/// only thing separating the sections apart from the hairline, so neither can
/// afford to sit off the column the rows below it start on.
fn section_heading(label: &'static str, count: usize, skin: Surface) -> impl IntoElement {
    div()
        .h(px(SECTION_HEADER_HEIGHT))
        .flex_none()
        .pl(px(4.))
        .pr(px(6.))
        .pt(px(8.))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .child(
            div()
                // Clears the grip column, landing the label on the checkboxes.
                .pl(px(LABEL_INDENT))
                .text_size(px(11.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(INK_SOFT))
                .child(label),
        )
        .child(
            div()
                .text_size(px(10.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(INK_SOFT))
                .bg(rgb(skin.action_bg))
                .px(px(5.))
                .py(px(1.))
                .rounded(px(8.))
                .child(count.to_string()),
        )
}

/// All that divides the two sections, now that neither has a surface of its
/// own. Inset to the gutter so it reads as a fold in the paper rather than as
/// an edge of the window.
///
/// Only a bottom margin: the space above is the ongoing section's tail, which
/// has to be there anyway to catch a drop past the last row. Adding a top
/// margin as well pushed the line down against the heading below it.
fn section_hairline() -> impl IntoElement {
    div()
        .flex_none()
        .mx(px(GUTTER))
        .mb(px(6.))
        .h(px(1.))
        .bg(rgb(HAIRLINE))
}

/// An empty region says what to do next rather than sitting blank.
fn empty_lines(line: &'static str, hint: &'static str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(3.))
        .child(
            div()
                .text_size(px(12.5))
                .text_color(rgb(INK_SOFT))
                .child(line),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(rgb(INK_FAINT))
                .child(hint),
        )
}

impl VibeTodoApp {
    fn render_task_row(
        &self,
        task: &Task,
        column: ColumnType,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // Both sections stand on the same paper, so they are painted with the
        // same surface; POPOVER is left to the one plane that is still white.
        let skin = PAPER;
        let phase = self.completing.get(&task.id).copied();
        let is_done = phase.is_some();
        let is_editing = self.editing_id.as_deref() == Some(task.id.as_str());
        let is_armed = self.confirm_delete.as_deref() == Some(task.id.as_str());

        let id_complete = task.id.clone();
        let id_move = task.id.clone();
        let id_delete = task.id.clone();
        let id_priority = task.id.clone();
        let id_edit = task.id.clone();
        let id_hover = task.id.clone();

        let ghost_title = SharedString::from(task.title.clone());
        let drag_payload = DraggedTask {
            id: task.id.clone(),
            title: ghost_title.clone(),
        };
        let anchor_id = task.id.clone();
        let other_column = match column {
            ColumnType::Ongoing => ColumnType::Inbox,
            ColumnType::Inbox => ColumnType::Ongoing,
        };

        let row = div()
            .id(SharedString::from(format!("task-row-{}", task.id)))
            .group("task-row")
            .relative()
            .h(px(ROW_HEIGHT))
            // gpui defaults flex_shrink to 1, so without this a short window
            // compresses every row instead of scrolling.
            .flex_none()
            .pl(px(4.))
            .pr(px(6.))
            .flex()
            .flex_row()
            .items_center()
            .rounded(px(ROW_RADIUS))
            .hover(|s| s.bg(rgb(skin.row_hover)))
            .when(is_editing, |s| {
                // Clicking away keeps the edit — same idea as the new-task
                // field, which commits when there is text to save.
                s.bg(rgb(skin.row_hover))
                    .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                        this.commit_edit(window, cx);
                    }))
            })
            // ⌘⌫ acts on whatever the pointer is over, so the row has to say so
            // in state; an armed delete that scrolls out of reach is a trap, so
            // leaving the row disarms it.
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                if *hovered {
                    this.hovered_id = Some(id_hover.clone());
                } else {
                    if this.hovered_id.as_deref() == Some(id_hover.as_str()) {
                        this.hovered_id = None;
                    }
                    if this.confirm_delete.as_deref() == Some(id_hover.as_str()) {
                        this.confirm_delete = None;
                    }
                }
                cx.notify();
            }))
            // Dropping onto a row inserts above it, which is what the insertion
            // line drawn on drag_over promises.
            .on_drop(cx.listener(move |this, dragged: &DraggedTask, _, cx| {
                this.reposition_task(&dragged.id, column, Some(&anchor_id), cx);
                cx.stop_propagation();
            }))
            .child(drop_indicator("task-row"))
            .child(
                div()
                    .id(SharedString::from(format!("grip-{}", task.id)))
                    // Margins pull the box back to the 12px it used to occupy,
                    // so the target grows without the row's layout moving.
                    .w(px(20.))
                    .h(px(28.))
                    .mx(px(-4.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_grab()
                    .active(|s| s.cursor_grabbing())
                    // Hidden at rest, by opacity rather than by visibility:
                    // gpui skips painting a hidden element and with it the drag
                    // listener. The hover tray wants that — it covers the title
                    // and must not be grabbable through it — the grip does not.
                    .opacity(0.)
                    .group_hover("task-row", |s| s.opacity(1.))
                    .child(icon_grip(hsl(INK_FAINT)).size(px(12.)))
                    .when(!is_editing, |s| {
                        s.on_drag(drag_payload, move |dragged, _offset, _window, cx| {
                            let title = dragged.title.clone();
                            cx.new(|_| DragGhost { title })
                        })
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(7.))
                    .flex_1()
                    .min_w(px(0.))
                    .pl(px(4.))
                    .child(self.render_checkbox(
                        task,
                        skin,
                        phase,
                        cx.listener(move |this, _, _, cx| {
                            this.complete_task(&id_complete, cx);
                        }),
                    ))
                    .when(task.is_priority && !is_editing, |this| {
                        this.child(
                            div()
                                .flex_none()
                                .flex()
                                .items_center()
                                .child(icon_flame(hsl(PRIORITY)).size(px(13.))),
                        )
                    })
                    .when(is_editing, |this| {
                        this.child(
                            div().flex_1().min_w(px(0.)).child(
                                Input::new(&self.edit_input)
                                    .appearance(false)
                                    .focus_bordered(false)
                                    .px(px(0.))
                                    .text_size(px(13.)),
                            ),
                        )
                    })
                    .when(!is_editing, |this| {
                        this.child(
                            div()
                                .id(SharedString::from(format!("title-{}", task.id)))
                                .flex_1()
                                .min_w(px(0.))
                                .text_size(px(13.))
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .text_ellipsis()
                                // The only hint that a title can be opened. An
                                // icon or a rule here would show up on every row
                                // at rest, which the list cannot afford.
                                .cursor_text()
                                .when(is_done, |s| s.line_through().text_color(rgb(INK_FAINT)))
                                .when(!is_done, |s| {
                                    s.text_color(rgb(INK)).font_weight(if task.is_priority {
                                        FontWeight::MEDIUM
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                })
                                .on_click(cx.listener(
                                    move |this, event: &ClickEvent, window, cx| {
                                        if event.click_count() >= 2 {
                                            this.start_editing(&id_edit, window, cx);
                                        }
                                    },
                                ))
                                .child(task.title.clone()),
                        )
                    }),
            )
            .when(!is_done && !is_editing, |this| {
                this.child(
                    // The tray still sits over the title — it fades in from
                    // transparent at its left edge so a long title dissolves
                    // under it instead of being cut mid-character.
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .right(px(6.))
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(4.))
                        .pl(px(TRAY_FADE))
                        .bg(tray_fade(skin))
                        .invisible()
                        .group_hover("task-row", |s| s.visible())
                        .child(
                            action_button(
                                format!("prio-{}", task.id),
                                21.,
                                skin,
                                false,
                                icon_flame(if task.is_priority {
                                    hsl(PRIORITY)
                                } else {
                                    hsl(INK_SOFT)
                                })
                                .size(px(11.)),
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.toggle_priority(&id_priority, cx);
                                },
                            )),
                        )
                        .child(
                            // Drag is the pleasant way across; this is the
                            // reliable one when the list is scrolled away from
                            // the other section.
                            action_button(
                                format!("mv-{}", task.id),
                                21.,
                                skin,
                                false,
                                match column {
                                    ColumnType::Ongoing => icon_arrow_down(hsl(INK_SOFT)),
                                    ColumnType::Inbox => icon_arrow_up(hsl(INK_SOFT)),
                                }
                                .size(px(11.)),
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.move_task(&id_move, other_column, cx);
                                },
                            )),
                        )
                        .child(
                            action_button(
                                format!("del-{}", task.id),
                                21.,
                                skin,
                                false,
                                icon_trash(if is_armed {
                                    hsl(0xFFFFFF)
                                } else {
                                    hsl(INK_SOFT)
                                })
                                .size(px(11.)),
                            )
                            // Armed. There is no room for words at this size, so
                            // the colour is the whole of the warning.
                            .when(is_armed, |s| {
                                s.bg(rgb(PRIORITY))
                                    .hover(|s| s.bg(rgb(PRIORITY)))
                                    .active(|s| s.bg(rgb(PRIORITY)))
                            })
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.request_delete(&id_delete, cx);
                                },
                            )),
                        ),
                )
            });

        match phase {
            Some(Completing::Leaving) => row
                .overflow_hidden()
                .with_animation(
                    SharedString::from(format!("leave-{}", task.id)),
                    Animation::new(COMPLETION_COLLAPSE).with_easing(ease_out_quint()),
                    |el, delta| el.h(px(ROW_HEIGHT * (1. - delta))).opacity(1. - delta),
                )
                .into_any_element(),
            _ => row.into_any_element(),
        }
    }

    /// The most-clicked control in the window, so its target is the largest
    /// thing here that does not show. Pressing previews the result rather than
    /// just darkening: you see what the click is about to do before you let go.
    fn render_checkbox(
        &self,
        task: &Task,
        skin: Surface,
        phase: Option<Completing>,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    ) -> impl IntoElement {
        let is_done = phase.is_some();
        let tick_id = SharedString::from(format!("tick-{}", task.id));
        // The circle reacts to the whole target around it, so hover and press
        // are read off the group rather than off the circle's own 16px hitbox.
        let group = SharedString::from(format!("cb-group-{}", task.id));

        div()
            .id(SharedString::from(format!("cb-{}", task.id)))
            .group(group.clone())
            // 26 x 28 of target inside 16px of layout: the margins give the
            // extra back so nothing around it moves.
            .w(px(26.))
            .h(px(28.))
            .mx(px(-5.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .on_click(on_click)
            .child(
                div()
                    .id(SharedString::from(format!("cb-mark-{}", task.id)))
                    .relative()
                    .w(px(16.))
                    .h(px(16.))
                    .rounded_full()
                    .border_1()
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(is_done, |s| s.border_color(rgb(DONE)).bg(rgb(skin.field)))
                    .when(!is_done, |s| {
                        s.bg(rgb(skin.field))
                            .border_color(rgb(INK_FAINT))
                            .group_hover(group.clone(), |s| s.border_color(rgb(INK)))
                            // Pressing previews the outcome instead of merely
                            // darkening: the colour under the cursor is the one
                            // the click is about to commit to.
                            .group_active(group.clone(), |s| {
                                s.border_color(rgb(DONE)).bg(hsl(DONE).opacity(0.12))
                            })
                    })
                    .when(is_done, |s| {
                        s.child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .right_0()
                                .bottom_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .with_animation(
                                    tick_id,
                                    Animation::new(TICK).with_easing(ease_out_quint()),
                                    |el, delta| {
                                        // The fill grows from the middle. With
                                        // no transforms available, an absolutely
                                        // positioned disc is the only way to do
                                        // it without reflowing the row.
                                        let d = 14. * delta;
                                        el.child(
                                            div()
                                                .absolute()
                                                .w(px(d))
                                                .h(px(d))
                                                .left(px((14. - d) / 2.))
                                                .top(px((14. - d) / 2.))
                                                .rounded_full()
                                                .bg(rgb(DONE)),
                                        )
                                        .child(
                                            div()
                                                .opacity(((delta - 0.45) / 0.55).clamp(0., 1.))
                                                .child(icon_check(hsl(0xFFFFFF)).size(px(9.))),
                                        )
                                    },
                                ),
                        )
                    }),
            )
    }

    fn render_history_popover(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let skin = POPOVER;
        let confirm_clear = self.confirm_clear;
        let is_empty = self.completed_tasks.is_empty();

        // The popover hangs under the button that opens it, but 272 x 300 is a
        // wish, not a size: at the smallest window it allows it would run off
        // the left edge and the bottom. So it stretches inside a frame cut to
        // the window instead, and takes whichever is smaller.
        div()
            .absolute()
            .top(px(CHROME_HEIGHT + 2.))
            .left(px(GUTTER))
            .right(px(GUTTER))
            .bottom(px(GUTTER))
            .max_h(px(300.))
            .flex()
            .justify_end()
            .items_start()
            .child(
                div()
                    .id("history-popover")
                    .w_full()
                    .max_w(px(272.))
                    .max_h(relative(1.))
                    .bg(rgb(RAISED))
                    .border_1()
                    .border_color(rgb(HAIRLINE))
                    .rounded(px(10.))
                    .shadow(popover_shadow())
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                        div()
                            .px(px(10.))
                            .pt(px(9.))
                            .pb(px(4.))
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(INK_SOFT))
                                    .child("已完成"),
                            )
                            // Clearing history cannot be undone, so the first click only
                            // arms the button and says what the second one will do. It
                            // disarms itself on a timer and when the pointer leaves, so
                            // a click landing later cannot finish it.
                            .when(!is_empty, |this| {
                                this.child(
                                    div()
                                        .id("clear-completed")
                                        .h(px(19.))
                                        .px(px(5.))
                                        .rounded(px(4.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .hover(|s| s.bg(rgb(skin.action_bg)))
                                        .active(|s| s.bg(rgb(skin.action_bg_active)))
                                        .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                                            if !*hovered && this.confirm_clear {
                                                this.confirm_clear = false;
                                                cx.notify();
                                            }
                                        }))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.clear_completed(cx);
                                        }))
                                        .when(confirm_clear, |s| {
                                            s.child(
                                                div()
                                                    .text_size(px(11.))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(rgb(PRIORITY))
                                                    .child("再点一次清空"),
                                            )
                                        })
                                        .when(!confirm_clear, |s| {
                                            s.child(icon_trash(hsl(INK_SOFT)).size(px(11.)))
                                        }),
                                )
                            }),
                    )
                    .when(is_empty, |this| {
                        this.child(
                            div()
                                .px(px(12.))
                                .pt(px(2.))
                                .pb(px(14.))
                                .text_size(px(11.5))
                                .text_color(rgb(INK_FAINT))
                                .child("勾掉的任务会留在这里"),
                        )
                    })
                    .when(!is_empty, |this| {
                        this.child(
                            div()
                                .id("completed-list")
                                .px(px(6.))
                                .pb(px(6.))
                                .pt(px(2.))
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .overflow_y_scroll()
                                .children(self.completed_tasks.iter().map(|task| {
                                    let id_restore = task.id.clone();
                                    let id_delete = task.id.clone();
                                    let id_hover = task.id.clone();
                                    let is_armed =
                                        self.confirm_delete.as_deref() == Some(task.id.as_str());

                                    div()
                                        .id(SharedString::from(format!("completed-{}", task.id)))
                                        .group("completed-row")
                                        .h(px(30.))
                                        .px(px(6.))
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .justify_between()
                                        .rounded(px(5.))
                                        .hover(|s| s.bg(rgb(POPOVER.row_hover)))
                                        .on_hover(cx.listener(
                                            move |this, hovered: &bool, _, cx| {
                                                if !*hovered
                                                    && this.confirm_delete.as_deref()
                                                        == Some(id_hover.as_str())
                                                {
                                                    this.confirm_delete = None;
                                                    cx.notify();
                                                }
                                            },
                                        ))
                                        .child(
                                            div()
                                                .flex()
                                                .flex_row()
                                                .items_center()
                                                .gap(px(8.))
                                                .flex_1()
                                                .min_w(px(0.))
                                                .child(
                                                    div()
                                                        .w(px(16.))
                                                        .h(px(16.))
                                                        .flex_none()
                                                        .rounded_full()
                                                        .bg(rgb(DONE))
                                                        .border_1()
                                                        .border_color(rgb(DONE))
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .child(
                                                            icon_check(hsl(0xFFFFFF)).size(px(9.)),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w(px(0.))
                                                        .text_size(px(12.5))
                                                        .text_color(rgb(INK_SOFT))
                                                        .line_through()
                                                        .whitespace_nowrap()
                                                        .overflow_hidden()
                                                        .text_ellipsis()
                                                        .child(task.title.clone()),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .flex_row()
                                                .items_center()
                                                .gap(px(4.))
                                                .flex_none()
                                                .invisible()
                                                .group_hover("completed-row", |s| s.visible())
                                                .child(
                                                    action_button(
                                                        format!("restore-{}", task.id),
                                                        19.,
                                                        skin,
                                                        false,
                                                        icon_rotate_ccw(hsl(INK_SOFT))
                                                            .size(px(10.)),
                                                    )
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.restore_task(&id_restore, cx);
                                                    })),
                                                )
                                                .child(
                                                    action_button(
                                                        format!("cdel-{}", task.id),
                                                        19.,
                                                        skin,
                                                        false,
                                                        icon_trash(if is_armed {
                                                            hsl(0xFFFFFF)
                                                        } else {
                                                            hsl(INK_SOFT)
                                                        })
                                                        .size(px(10.)),
                                                    )
                                                    .when(is_armed, |s| {
                                                        s.bg(rgb(PRIORITY))
                                                            .hover(|s| s.bg(rgb(PRIORITY)))
                                                            .active(|s| s.bg(rgb(PRIORITY)))
                                                    })
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.request_delete(&id_delete, cx);
                                                    })),
                                                ),
                                        )
                                })),
                        )
                    }),
            )
    }
}

impl VibeTodoApp {
    /// The top strip: traffic lights on the left (drawn by macOS), the app's
    /// three actions on the right, and nothing else — no surface, no rule. It
    /// is still the window's drag handle.
    fn render_chrome(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_history_open = self.is_history_open;
        let is_pinned = self.is_pinned;

        div()
            .h(px(CHROME_HEIGHT))
            .flex_none()
            .px(px(GUTTER))
            .flex()
            .flex_row()
            .items_center()
            .justify_end()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, window, _| window.start_window_move()),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(2.))
                    .child(
                        // No state of its own: when this is open the field is
                        // sitting in the list saying so already.
                        action_button(
                            "btn-add",
                            22.,
                            PAPER,
                            false,
                            icon_plus(hsl(INK_SOFT)).size(px(13.)),
                        )
                        .tooltip(|window, cx| {
                            Tooltip::new("记一笔")
                                .action(&NewTask, Some("VibeTodo"))
                                .build(window, cx)
                        })
                        .tooltip_show_delay(TOOLTIP_DELAY)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.start_creating(window, cx);
                        })),
                    )
                    .child(
                        action_button(
                            "btn-history",
                            22.,
                            PAPER,
                            is_history_open,
                            icon_history(hsl(INK_SOFT)).size(px(12.)),
                        )
                        .tooltip(|window, cx| {
                            Tooltip::new("已完成")
                                .action(&ToggleHistory, Some("VibeTodo"))
                                .build(window, cx)
                        })
                        .tooltip_show_delay(TOOLTIP_DELAY)
                        .on_click(
                            cx.listener(|this, _, window, cx| this.toggle_history(window, cx)),
                        ),
                    )
                    .child(
                        // Pinning is the one mode that stays on, so it is said
                        // in the ink the rest of the window already speaks:
                        // a solid glyph at full strength. A filled swatch behind
                        // it out-shouted the paper it sits above.
                        action_button(
                            "btn-pin",
                            22.,
                            PAPER,
                            false,
                            if is_pinned {
                                icon_pin_filled(hsl(INK)).size(px(12.))
                            } else {
                                icon_pin(hsl(INK_SOFT)).size(px(12.))
                            },
                        )
                        .tooltip(|window, cx| {
                            Tooltip::new("窗口置顶")
                                .action(&TogglePin, Some("VibeTodo"))
                                .build(window, cx)
                        })
                        .tooltip_show_delay(TOOLTIP_DELAY)
                        .on_click(cx.listener(|this, _, _, cx| this.toggle_pin(cx))),
                    ),
            )
    }

    /// One section of tasks, on the shared paper ground. Both are built here so
    /// the two cannot drift apart: a heading, the rows, then a tail that takes
    /// drops landing past the last row and carries the empty copy when there is
    /// no row to drop onto yet.
    fn render_section(&self, column: ColumnType, cx: &mut Context<Self>) -> impl IntoElement {
        let (id, tail_id, label, empty_line, empty_hint, tasks) = match column {
            ColumnType::Ongoing => (
                "ongoing-section",
                "ongoing-tail",
                "进行中",
                "还没有开始的事",
                "从下面拖一件上来",
                &self.ongoing_tasks,
            ),
            ColumnType::Inbox => (
                "inbox-section",
                "inbox-tail",
                "收件箱",
                "收件箱是空的",
                "⌘N 记一笔",
                &self.inbox_tasks,
            ),
        };
        let show_add_row = matches!(column, ColumnType::Inbox) && self.is_creating;
        let is_empty = tasks.is_empty() && !show_add_row;

        div()
            .id(id)
            .flex_none()
            .px(px(GUTTER))
            .flex()
            .flex_col()
            .gap(px(1.))
            // The fallback for a drop that lands on the heading or in a gap. A
            // row or the tail under the cursor is more specific and stops the
            // event before it reaches here.
            .on_drop(cx.listener(move |this, dragged: &DraggedTask, _, cx| {
                this.reposition_task(&dragged.id, column, None, cx);
            }))
            .child(section_heading(label, tasks.len(), PAPER))
            .when(show_add_row, |this| this.child(self.render_add_row(cx)))
            .children(
                tasks
                    .iter()
                    .map(|task| self.render_task_row(task, column, cx)),
            )
            .child(
                div()
                    .id(tail_id)
                    .group(tail_id)
                    .relative()
                    .flex_none()
                    .min_h(px(12.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_drop(cx.listener(move |this, dragged: &DraggedTask, _, cx| {
                        this.reposition_task(&dragged.id, column, None, cx);
                        cx.stop_propagation();
                    }))
                    .child(drop_indicator(tail_id))
                    .when(is_empty, |this| {
                        this.py(px(18.)).child(empty_lines(empty_line, empty_hint))
                    }),
            )
    }

    fn render_add_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("add-row")
            .h(px(ROW_HEIGHT))
            .flex_none()
            .pl(px(4.))
            .pr(px(6.))
            .flex()
            .flex_row()
            .items_center()
            .rounded(px(ROW_RADIUS))
            // Typing into the list looks the same whether the row is new or
            // being renamed, so this is the tint a row wears while editing.
            .bg(rgb(PAPER.row_hover))
            // Clicking away is the same as pressing enter: keep whatever was
            // typed, drop an empty field.
            .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                this.commit_new_task(window, cx);
            }))
            // Stands in for the grip column so the field lines up with the rows
            // it is about to join.
            .child(div().w(px(12.)).flex_none())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(7.))
                    .flex_1()
                    .min_w(px(0.))
                    .pl(px(4.))
                    .child(
                        div()
                            .w(px(16.))
                            .h(px(16.))
                            .flex_none()
                            .rounded_full()
                            .border_1()
                            .border_color(rgb(INK_FAINT))
                            .bg(rgb(PAPER.field)),
                    )
                    .child(
                        div().flex_1().min_w(px(0.)).child(
                            // Input carries its own horizontal padding
                            // regardless of `appearance`, which would
                            // otherwise double the gap after the box.
                            Input::new(&self.input)
                                .appearance(false)
                                .focus_bordered(false)
                                .px(px(0.))
                                .text_size(px(13.)),
                        ),
                    ),
            )
    }
}

/// Long enough that sweeping past a button never summons one. These three are
/// the only labels in the window, and it is a window people glance at all day.
const TOOLTIP_DELAY: Duration = Duration::from_millis(600);

/// Where the inbox sits among the body scroller's children. `scroll_to_item`
/// addresses them by index, so this and the order built in `render` have to
/// stay in step.
const BODY_INBOX_INDEX: usize = 2;

impl Render for VibeTodoApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.note_window_bounds(window, cx);
        let is_history_open = self.is_history_open;

        div()
            .id("vibe-todo-root")
            .track_focus(&self.focus_handle)
            // While a field has focus the window drops out of the context that
            // owns ⌘⌫, so it reaches the text instead of the hovered task.
            .key_context(if self.is_editing() {
                "VibeTodo editing"
            } else {
                "VibeTodo"
            })
            .on_action(cx.listener(|this, _: &NewTask, window, cx| {
                this.start_creating(window, cx);
            }))
            .on_action(cx.listener(|this, _: &TogglePin, _, cx| this.toggle_pin(cx)))
            .on_action(
                cx.listener(|this, _: &ToggleHistory, window, cx| this.toggle_history(window, cx)),
            )
            .on_action(cx.listener(|this, _: &Cancel, window, cx| {
                if this.editing_id.is_some() {
                    this.cancel_edit(window, cx);
                } else if this.is_creating {
                    this.cancel_new_task(window, cx);
                } else if this.is_history_open {
                    this.toggle_history(window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &DeleteHovered, _, cx| {
                if let Some(id) = this.hovered_id.clone() {
                    this.delete_task(&id, cx);
                }
            }))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(GROUND))
            .font_family(".SystemUIFont")
            .text_color(rgb(INK))
            .child(self.render_chrome(cx))
            // Both sections share one scroller, so each keeps the height of
            // what is in it and the window moves as a single sheet of paper.
            .child(
                div()
                    .id("body-scroll")
                    .track_scroll(&self.body_scroll)
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .child(self.render_section(ColumnType::Ongoing, cx))
                    .child(section_hairline())
                    .child(self.render_section(ColumnType::Inbox, cx)),
            )
            // The scrim starts below the chrome so a second click on the history
            // button reaches the button instead of being cancelled out by it.
            .when(is_history_open, |this| {
                this.child(
                    div()
                        .id("history-scrim")
                        .absolute()
                        .top(px(CHROME_HEIGHT))
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .on_click(
                            cx.listener(|this, _, window, cx| this.toggle_history(window, cx)),
                        ),
                )
                .child(self.render_history_popover(cx))
            })
    }
}
