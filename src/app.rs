use crate::db::Database;
use crate::icons::*;
use crate::model::{ColumnType, Task};
use crate::theme::*;
use crate::window_level::set_window_always_on_top;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, rgb, AppContext as _, Context, Entity, FocusHandle, Focusable as _, FontWeight,
    InteractiveElement as _, IntoElement, KeyDownEvent, MouseButton, ParentElement as _, Render,
    SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use std::collections::HashSet;
use std::time::Duration;

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
            .bg(rgb(CARD_SURFACE))
            .shadow(card_shadow())
            .font_family(".SystemUIFont")
            .text_size(px(13.))
            .text_color(rgb(INK))
            .whitespace_nowrap()
            .overflow_hidden()
            .text_ellipsis()
            .child(self.title.clone())
    }
}

/// How long a checked row stays visible, struck through, before it leaves.
const COMPLETION_DWELL: Duration = Duration::from_millis(240);

/// The OS owns the top-left corner, so the top strip stays clear for the
/// traffic lights. It carries no surface and no rule of its own — the glass
/// runs straight through it into the window.
const CHROME_HEIGHT: f32 = 34.;
const GUTTER: f32 = 12.;
const CARD_PADDING: f32 = 8.;
const ROW_RADIUS: f32 = 6.;
/// Apple's concentric rule: an outer radius equals the inner one plus the
/// padding between them, so the two curves stay parallel. (gpui draws plain
/// circular arcs — the continuous curvature half of the spec is not available.)
const CARD_RADIUS: f32 = ROW_RADIUS + CARD_PADDING;
const ROW_HEIGHT: f32 = 36.;

/// The card grows with what is in it, then scrolls. Both ends are pinned so the
/// inbox underneath can never be squeezed out of the window.
const CARD_HEADER_HEIGHT: f32 = 28.;
const CARD_MIN_HEIGHT: f32 = 92.;
const CARD_MAX_HEIGHT: f32 = 192.;

pub struct VibeTodoApp {
    db: Database,
    inbox_tasks: Vec<Task>,
    ongoing_tasks: Vec<Task>,
    completed_tasks: Vec<Task>,
    input: Entity<InputState>,
    pub is_pinned: bool,
    pub is_history_open: bool,
    pub is_creating: bool,
    /// Rows that have been checked and are playing out their dwell.
    completing: HashSet<String>,
    /// "Clear all" is armed by a first click and fires on the second.
    confirm_clear: bool,
    pub focus_handle: FocusHandle,
    _input_sub: Subscription,
}

impl VibeTodoApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let db = Database::new().expect("Failed to initialize SQLite database");
        let inbox_tasks = db.get_active_tasks(ColumnType::Inbox).unwrap_or_default();
        let ongoing_tasks = db.get_active_tasks(ColumnType::Ongoing).unwrap_or_default();
        let completed_tasks = db.get_completed_tasks().unwrap_or_default();

        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("记一笔，回车保存")
                .submit_on_enter(true)
        });

        let input_sub =
            cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.commit_new_task(window, cx);
                }
            });

        Self {
            db,
            inbox_tasks,
            ongoing_tasks,
            completed_tasks,
            input,
            is_pinned: false,
            is_history_open: false,
            is_creating: false,
            completing: HashSet::new(),
            confirm_clear: false,
            focus_handle: cx.focus_handle(),
            _input_sub: input_sub,
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
        cx.notify();
    }

    pub fn toggle_history(&mut self, cx: &mut Context<Self>) {
        self.is_history_open = !self.is_history_open;
        self.confirm_clear = false;
        if self.is_history_open {
            self.refresh_tasks();
        }
        cx.notify();
    }

    pub fn start_creating(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.is_creating = true;
        self.is_history_open = false;
        self.input
            .update(cx, |state, cx| state.set_value("", window, cx));
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
        // Stay open so several thoughts can be parked in a row.
        self.input
            .update(cx, |state, cx| state.set_value("", window, cx));
        cx.notify();
    }

    pub fn cancel_new_task(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.is_creating = false;
        self.focus_handle.clone().focus(window, cx);
        cx.notify();
    }

    /// Marks the row done on screen, then writes it away once the dwell ends.
    pub fn complete_task(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.completing.insert(id.to_string()) {
            return;
        }
        cx.notify();

        let id = id.to_string();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COMPLETION_DWELL).await;
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

    pub fn delete_task(&mut self, id: &str, cx: &mut Context<Self>) {
        let _ = self.db.delete_task(id);
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
            return;
        }
        let _ = self.db.clear_completed_tasks();
        self.confirm_clear = false;
        self.refresh_tasks();
        cx.notify();
    }
}

/// A square hover action: move, delete, priority, restore.
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

/// The card names itself; the count sits opposite. The inbox below stays
/// unlabelled — it is everything else, which needs no introduction.
fn card_heading(count: usize) -> impl IntoElement {
    div()
        .h(px(CARD_HEADER_HEIGHT))
        .flex_none()
        .px(px(CARD_PADDING + 4.))
        .pt(px(8.))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .child(
            div()
                .text_size(px(11.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(INK_SOFT))
                .child("进行中"),
        )
        .child(
            div()
                .text_size(px(10.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(INK_SOFT))
                .bg(rgb(CARD.action_bg))
                .px(px(5.))
                .py(px(1.))
                .rounded(px(8.))
                .child(format!("{}", count)),
        )
}

/// An empty region says what to do next rather than sitting blank. There is no
/// heading anywhere in the window, so for the card this copy is also the only
/// thing that says what the card is for.
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
        on_card: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let skin = if on_card { CARD } else { LIST };
        let is_done = self.completing.contains(&task.id);

        let id_complete = task.id.clone();
        let id_move = task.id.clone();
        let id_delete = task.id.clone();
        let id_priority = task.id.clone();

        let ghost_title = SharedString::from(task.title.clone());
        let drag_payload = DraggedTask {
            id: task.id.clone(),
            title: ghost_title.clone(),
        };
        let anchor_id = task.id.clone();
        let drop_column = if on_card {
            ColumnType::Ongoing
        } else {
            ColumnType::Inbox
        };

        div()
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
            // Dropping onto a row inserts above it, which is what the insertion
            // line drawn on drag_over promises.
            .on_drop(cx.listener(move |this, dragged: &DraggedTask, _, cx| {
                this.reposition_task(&dragged.id, drop_column, Some(&anchor_id), cx);
                cx.stop_propagation();
            }))
            .child(drop_indicator("task-row"))
            .child(
                div()
                    .id(SharedString::from(format!("grip-{}", task.id)))
                    .w(px(12.))
                    .h(px(20.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_grab()
                    .child(icon_grip(hsl(INK_FAINT)).size(px(12.)))
                    .on_drag(drag_payload, move |dragged, _offset, _window, cx| {
                        let title = dragged.title.clone();
                        cx.new(|_| DragGhost { title })
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
                    .child(
                        div()
                            .id(SharedString::from(format!("cb-{}", task.id)))
                            .w(px(16.))
                            .h(px(16.))
                            .flex_none()
                            .rounded_full()
                            .border_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .when(is_done, |s| {
                                s.bg(rgb(DONE))
                                    .border_color(rgb(DONE))
                                    .child(icon_check(hsl(0xFFFFFF)).size(px(9.)))
                            })
                            .when(!is_done, |s| {
                                s.bg(rgb(skin.field))
                                    .border_color(rgb(INK_FAINT))
                                    .hover(|s| s.border_color(rgb(INK)))
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.complete_task(&id_complete, cx);
                            })),
                    )
                    .when(task.is_priority, |this| {
                        this.child(
                            div()
                                .flex_none()
                                .flex()
                                .items_center()
                                .child(icon_flame(hsl(PRIORITY)).size(px(13.))),
                        )
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .text_size(px(13.))
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .when(is_done, |s| s.line_through().text_color(rgb(INK_FAINT)))
                            .when(!is_done, |s| {
                                s.text_color(rgb(INK)).font_weight(if task.is_priority {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                            })
                            .child(task.title.clone()),
                    ),
            )
            .child(
                // Opaque on purpose: this slides in over the title and has to
                // hide it, so it matches the row's own hover colour exactly.
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .right(px(6.))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.))
                    .pl(px(10.))
                    .bg(rgb(skin.row_hover))
                    .invisible()
                    .when(!is_done, |s| s.group_hover("task-row", |s| s.visible()))
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
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_priority(&id_priority, cx);
                        })),
                    )
                    .child(
                        // Drag is the pleasant way across; this is the reliable
                        // one when the list is scrolled away from the card.
                        action_button(
                            format!("mv-{}", task.id),
                            21.,
                            skin,
                            false,
                            if on_card {
                                icon_arrow_down(hsl(INK_SOFT)).size(px(11.))
                            } else {
                                icon_arrow_up(hsl(INK_SOFT)).size(px(11.))
                            },
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let target = if on_card {
                                ColumnType::Inbox
                            } else {
                                ColumnType::Ongoing
                            };
                            this.move_task(&id_move, target, cx);
                        })),
                    )
                    .child(
                        action_button(
                            format!("del-{}", task.id),
                            21.,
                            skin,
                            false,
                            icon_trash(hsl(INK_SOFT)).size(px(11.)),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.delete_task(&id_delete, cx);
                        })),
                    ),
            )
    }

    fn render_history_popover(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let skin = CARD;
        let confirm_clear = self.confirm_clear;
        let is_empty = self.completed_tasks.is_empty();

        div()
            .id("history-popover")
            .absolute()
            .top(px(CHROME_HEIGHT + 2.))
            .right(px(GUTTER))
            .w(px(272.))
            .max_h(px(300.))
            .bg(rgb(CARD_SURFACE))
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
                    // arms the button and says what the second one will do.
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
                                .hover(|s| s.bg(rgb(CARD.row_hover)))
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
                                                .child(icon_check(hsl(0xFFFFFF)).size(px(9.))),
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
                                                icon_rotate_ccw(hsl(INK_SOFT)).size(px(10.)),
                                            )
                                            .on_click(
                                                cx.listener(move |this, _, _, cx| {
                                                    this.restore_task(&id_restore, cx);
                                                }),
                                            ),
                                        )
                                        .child(
                                            action_button(
                                                format!("cdel-{}", task.id),
                                                19.,
                                                skin,
                                                false,
                                                icon_trash(hsl(INK_SOFT)).size(px(10.)),
                                            )
                                            .on_click(
                                                cx.listener(move |this, _, _, cx| {
                                                    this.delete_task(&id_delete, cx);
                                                }),
                                            ),
                                        ),
                                )
                        })),
                )
            })
    }
}

impl VibeTodoApp {
    /// The top strip: traffic lights on the left (drawn by macOS), the app's
    /// three actions on the right, and nothing else — no surface, no rule. It
    /// is still the window's drag handle.
    fn render_chrome(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_creating = self.is_creating;
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
                        action_button(
                            "btn-add",
                            22.,
                            LIST,
                            false,
                            icon_plus(if is_creating {
                                hsl(0xFFFFFF)
                            } else {
                                hsl(INK_SOFT)
                            })
                            .size(px(13.)),
                        )
                        .when(is_creating, |s| s.bg(rgb(INK)))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.start_creating(window, cx);
                        })),
                    )
                    .child(
                        action_button(
                            "btn-history",
                            22.,
                            LIST,
                            is_history_open,
                            icon_history(hsl(INK_SOFT)).size(px(12.)),
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.toggle_history(cx))),
                    )
                    .child(
                        action_button(
                            "btn-pin",
                            22.,
                            LIST,
                            false,
                            icon_pin(if is_pinned {
                                hsl(0xFFFFFF)
                            } else {
                                hsl(INK_SOFT)
                            })
                            .size(px(12.)),
                        )
                        .when(is_pinned, |s| s.bg(rgb(INK)))
                        .on_click(cx.listener(|this, _, _, cx| this.toggle_pin(cx))),
                    ),
            )
    }

    /// What is actually being worked on. The only opaque surface in the window,
    /// which is the whole of how it says "this is the live one" — the rows
    /// inside are the same rows as the inbox below.
    fn render_ongoing_card(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_empty = self.ongoing_tasks.is_empty();

        div()
            .id("ongoing-card")
            .flex_none()
            .mx(px(GUTTER))
            .min_h(px(CARD_MIN_HEIGHT))
            .max_h(px(CARD_MAX_HEIGHT))
            .flex()
            .flex_col()
            .rounded(px(CARD_RADIUS))
            .bg(rgb(CARD_SURFACE))
            .shadow(card_shadow())
            // The border is what makes the white card meet the grey ground
            // cleanly; on drag it darkens rather than appearing, so arming the
            // drop target cannot shift the card by a pixel.
            .border_1()
            .border_color(rgb(HAIRLINE))
            .drag_over::<DraggedTask>(|s, _, _, _| s.border_color(rgb(INK_FAINT)))
            .overflow_hidden()
            // Dropping anywhere on the card appends; a row under the cursor
            // takes precedence and inserts above itself instead.
            .on_drop(cx.listener(|this, dragged: &DraggedTask, _, cx| {
                this.reposition_task(&dragged.id, ColumnType::Ongoing, None, cx);
            }))
            .child(card_heading(self.ongoing_tasks.len()))
            .when(is_empty, |this| {
                this.child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(empty_lines("还没有开始的事", "从下面拖一件上来")),
                )
            })
            .when(!is_empty, |this| {
                this.child(
                    div()
                        .id("ongoing-list")
                        .flex_1()
                        .min_h(px(0.))
                        .p(px(CARD_PADDING))
                        .flex()
                        .flex_col()
                        .gap(px(1.))
                        .overflow_y_scroll()
                        .children(
                            self.ongoing_tasks
                                .iter()
                                .map(|task| self.render_task_row(task, true, cx)),
                        ),
                )
            })
    }

    /// Everything parked. Sits directly on the glass with no container of its
    /// own — the card above is the only thing in the window that gets one.
    fn render_inbox_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let show_add_row = self.is_creating;
        let is_empty = self.inbox_tasks.is_empty() && !show_add_row;

        div()
            .id("inbox-list")
            .flex_1()
            .min_h(px(0.))
            // GUTTER + the card's own inner padding, so a row in the list and a
            // row in the card start at exactly the same x.
            .px(px(GUTTER + CARD_PADDING))
            .pt(px(10.))
            .pb(px(6.))
            .flex()
            .flex_col()
            .gap(px(1.))
            .overflow_y_scroll()
            .when(show_add_row, |this| this.child(self.render_add_row(cx)))
            .children(
                self.inbox_tasks
                    .iter()
                    .map(|task| self.render_task_row(task, false, cx)),
            )
            // The tail takes drops that land past the last row, and is where
            // the empty copy goes so that an empty inbox is still a target.
            .child(
                div()
                    .id("drop-end")
                    .group("drop-end")
                    .relative()
                    .flex_1()
                    .min_h(px(12.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_drop(cx.listener(|this, dragged: &DraggedTask, _, cx| {
                        this.reposition_task(&dragged.id, ColumnType::Inbox, None, cx);
                    }))
                    .child(drop_indicator("drop-end"))
                    .when(is_empty, |this| {
                        this.child(empty_lines("收件箱是空的", "⌘N 记一笔"))
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
            .rounded(px(6.))
            .bg(rgb(CARD_SURFACE))
            // Clicking anywhere else drops the half-written task.
            .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                this.cancel_new_task(window, cx);
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
                            .bg(rgb(CARD_SURFACE)),
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

impl Render for VibeTodoApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_history_open = self.is_history_open;

        div()
            .id("vibe-todo-root")
            .track_focus(&self.focus_handle)
            .key_context("VibeTodo")
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(GROUND))
            .font_family(".SystemUIFont")
            .text_color(rgb(INK))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                let key = event.keystroke.key.to_lowercase();
                let cmd = event.keystroke.modifiers.platform;

                if cmd && key == "n" {
                    this.start_creating(window, cx);
                } else if cmd && event.keystroke.modifiers.shift && key == "p" {
                    this.toggle_pin(cx);
                } else if cmd && event.keystroke.modifiers.shift && key == "h" {
                    this.toggle_history(cx);
                } else if key == "escape" {
                    if this.is_history_open {
                        this.toggle_history(cx);
                    } else if this.is_creating {
                        this.cancel_new_task(window, cx);
                    }
                }
            }))
            .child(self.render_chrome(cx))
            .child(self.render_ongoing_card(cx))
            .child(self.render_inbox_list(cx))
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
                        .on_click(cx.listener(|this, _, _, cx| this.toggle_history(cx))),
                )
                .child(self.render_history_popover(cx))
            })
    }
}
