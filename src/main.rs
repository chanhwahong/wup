#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod i18n;
mod icons;
mod pixel_snap;
mod scrollbar;
mod theme;
mod ui_scale;
mod winget;

use gpui::{
    App, Application, Bounds, ClipboardItem, Context, DispatchPhase, Div, IntoElement, MouseButton,
    MouseMoveEvent, MouseUpEvent, Render, ScrollHandle, Stateful, Task, Window, WindowBounds,
    WindowOptions, canvas, deferred, div, point, prelude::*, px, quad, rems, rgb, size,
};
use i18n::{Language, Text};
use icons::{Icon, icon, spinner};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver},
    },
    time::Duration,
};
use theme::{Palette, Theme};
use ui_scale::UiScale;
use winget::{Event, ListMode, Operation, Package};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsPanel {
    Theme,
    Language,
    Scale,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScrollTarget {
    Packages,
    Logs,
}

#[derive(Clone, Default)]
enum Status {
    #[default]
    Ready,
    Queued,
    Running,
    Success(Operation),
    Failed(Text),
    Skipped,
}

struct Entry {
    package: Package,
    selected: bool,
    status: Status,
}

impl Entry {
    fn can_select(&self) -> bool {
        self.package.can_uninstall()
            && !matches!(
                self.status,
                Status::Success(Operation::Uninstall) | Status::Running
            )
    }
    fn can_upgrade(&self) -> bool {
        self.can_select() && self.package.can_upgrade()
    }
}

struct Wup {
    smoke_frames: Option<Arc<AtomicUsize>>,
    smoke_gap_checks: Option<Arc<AtomicUsize>>,
    ui_scale: UiScale,
    language: Language,
    theme: Theme,
    show_settings_menu: bool,
    show_about: bool,
    settings_panel: Option<SettingsPanel>,
    entries: Vec<Entry>,
    mode: ListMode,
    logs: VecDeque<Text>,
    receiver: Option<Receiver<Event>>,
    stop: Arc<AtomicBool>,
    busy: bool,
    scanning: bool,
    operation: Operation,
    batch: Vec<usize>,
    pending_uninstall: Option<Vec<(usize, Package)>>,
    scroll: ScrollHandle,
    log_scroll: ScrollHandle,
    dragging: Option<(ScrollTarget, f32)>,
    message: Text,
    error: bool,
    show_logs: bool,
    demo: bool,
    _poller: Task<()>,
}

impl Wup {
    fn new(demo: bool, cx: &mut Context<Self>) -> Self {
        let poller = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                if this.update(cx, |app, cx| app.poll(cx)).is_err() {
                    break;
                }
            }
        });
        let mut app = Self {
            smoke_frames: None,
            smoke_gap_checks: None,
            ui_scale: UiScale::load(),
            language: Language::load(),
            theme: Theme::load(),
            show_settings_menu: false,
            show_about: false,
            settings_panel: None,
            entries: Vec::new(),
            mode: ListMode::Updates,
            logs: VecDeque::new(),
            receiver: None,
            stop: Arc::new(AtomicBool::new(false)),
            busy: false,
            scanning: false,
            operation: Operation::Upgrade,
            batch: Vec::new(),
            pending_uninstall: None,
            scroll: ScrollHandle::new(),
            log_scroll: ScrollHandle::new(),
            dragging: None,
            message: Text::default(),
            error: false,
            show_logs: false,
            demo,
            _poller: poller,
        };
        app.refresh(cx);
        app
    }

    fn locked(&self) -> bool {
        self.busy || self.pending_uninstall.is_some() || self.show_about
    }

    fn close_settings(&mut self) {
        self.show_settings_menu = false;
        self.settings_panel = None;
    }

    fn render_settings_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme.palette();
        let language = self.language;
        div()
            .id("settings-menu")
            .absolute()
            .top_full()
            .right_0()
            .mt_2()
            .w(ui_len(280.))
            .p_1()
            .rounded_lg()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.panel))
            .shadow_lg()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .children(
                [
                    (
                        SettingsPanel::Theme,
                        Icon::Theme,
                        language.t("테마", "Theme"),
                        self.theme.name().to_owned(),
                    ),
                    (
                        SettingsPanel::Language,
                        Icon::Language,
                        language.t("언어", "Language"),
                        language.name().to_owned(),
                    ),
                    (
                        SettingsPanel::Scale,
                        Icon::Scale,
                        language.t("UI 배율", "UI scale"),
                        format!("{}%", self.ui_scale.percent()),
                    ),
                ]
                .into_iter()
                .enumerate()
                .map(|(index, (panel, symbol, label, value))| {
                    let active = self.settings_panel == Some(panel);
                    div()
                        .id(("settings-item", index))
                        .relative()
                        .h(ui_len(42.))
                        .px_3()
                        .flex()
                        .items_center()
                        .gap_2()
                        .rounded_md()
                        .bg(rgb(if active {
                            palette.selection
                        } else {
                            palette.panel
                        }))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(palette.hover)))
                        .child(icon(symbol, palette.text))
                        .child(label)
                        .child(div().flex_1())
                        .child(div().text_xs().text_color(rgb(palette.muted)).child(value))
                        .child(icon(Icon::ChevronLeft, palette.muted))
                        .on_mouse_move(cx.listener(move |this, _, _, cx| {
                            if this.settings_panel != Some(panel) {
                                this.settings_panel = Some(panel);
                                cx.notify();
                            }
                        }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if this.show_settings_menu {
                                this.settings_panel = Some(panel);
                                cx.notify();
                            }
                            cx.stop_propagation();
                        }))
                        .when(active, |d| {
                            // GPUI 0.2.2 forbids defer_draw inside a deferred draw.
                            // The parent popup is already deferred; paint these
                            // absolute children directly in that same layer.
                            d.child(match panel {
                                SettingsPanel::Theme => {
                                    self.render_theme_menu(cx).into_any_element()
                                }
                                SettingsPanel::Language => {
                                    self.render_language_menu(cx).into_any_element()
                                }
                                SettingsPanel::Scale => {
                                    self.render_scale_menu(cx).into_any_element()
                                }
                            })
                        })
                }),
            )
            .child(div().h(px(1.)).my_1().bg(rgb(palette.border)))
            .child(
                div()
                    .id("about-menu-item")
                    .h(ui_len(42.))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(palette.hover)))
                    .child(icon(Icon::Info, palette.text))
                    .child(language.t("앱 정보", "About"))
                    .on_mouse_move(cx.listener(|this, _, _, cx| {
                        if this.settings_panel.take().is_some() {
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.close_settings();
                        this.show_about = true;
                        cx.stop_propagation();
                        cx.notify();
                    })),
            )
    }

    fn render_scale_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme.palette();
        div()
            .id("scale-submenu")
            .absolute()
            .top_0()
            .right_full()
            .mr_2()
            .w(ui_len(280.))
            .p_1()
            .rounded_lg()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.panel))
            .shadow_lg()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .children([80, 90, 100, 110, 120].into_iter().map(|percent| {
                let active = percent == self.ui_scale.percent();
                div()
                    .id(("scale-preset", percent as usize))
                    .h(ui_len(36.))
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded_md()
                    .bg(rgb(if active {
                        palette.selection
                    } else {
                        palette.panel
                    }))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(palette.hover)))
                    .child(format!("{percent}%"))
                    .when(active, |d| d.child(icon(Icon::CheckSquare, palette.accent)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_ui_scale(UiScale::new(percent), cx);
                        this.close_settings();
                        cx.stop_propagation();
                        cx.notify();
                    }))
            }))
            .child(
                div()
                    .mt_1()
                    .pt_2()
                    .pb_1()
                    .border_t_1()
                    .border_color(rgb(palette.border))
                    .child(self.render_scale_controls(cx)),
            )
    }

    fn set_ui_scale(&mut self, scale: UiScale, cx: &mut Context<Self>) {
        if scale == self.ui_scale {
            return;
        }
        self.ui_scale = scale;
        self.dragging = None;
        if let Err(error) = scale.save() {
            self.log(Text::new(
                format!("배율은 적용했지만 설정을 저장하지 못했습니다: {error}"),
                format!("UI scale applied, but preferences could not be saved: {error}"),
            ));
            self.show_logs = true;
        }
        cx.notify();
    }

    fn render_scale_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme.palette();
        let scale = self.ui_scale;
        div()
            .flex()
            .items_center()
            .gap_2()
            .flex_shrink_0()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(palette.muted))
                    .child(self.language.t("UI 배율", "UI scale")),
            )
            .child(
                scale_button("scale-down", "−", scale.percent() > UiScale::MIN, palette).on_click(
                    cx.listener(|this, _, _, cx| this.set_ui_scale(this.ui_scale.smaller(), cx)),
                ),
            )
            .child(
                div()
                    .w(ui_len(46.))
                    .text_center()
                    .text_xs()
                    .child(format!("{}%", scale.percent())),
            )
            .child(
                scale_button("scale-up", "+", scale.percent() < UiScale::MAX, palette).on_click(
                    cx.listener(|this, _, _, cx| this.set_ui_scale(this.ui_scale.larger(), cx)),
                ),
            )
            .child(
                scale_button("scale-reset", "100%", scale != UiScale::default(), palette).on_click(
                    cx.listener(|this, _, _, cx| this.set_ui_scale(UiScale::default(), cx)),
                ),
            )
    }

    fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        self.close_settings();
        if let Err(error) = theme.save() {
            self.log(Text::new(
                format!("테마는 적용했지만 설정을 저장하지 못했습니다: {error}"),
                format!("Theme applied, but preferences could not be saved: {error}"),
            ));
            self.show_logs = true;
        }
        cx.notify();
    }

    fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        self.close_settings();
        if let Err(error) = language.save() {
            self.log(Text::new(
                format!("언어는 적용했지만 설정을 저장하지 못했습니다: {error}"),
                format!("Language applied, but preferences could not be saved: {error}"),
            ));
            self.show_logs = true;
        }
        cx.notify();
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        self.entries.clear();
        self.batch.clear();
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.dragging = None;
        self.error = false;
        if self.demo {
            self.entries = demo_packages()
                .into_iter()
                .filter(|p| self.mode == ListMode::All || p.can_upgrade())
                .map(|package| Entry {
                    package,
                    selected: false,
                    status: Status::Ready,
                })
                .collect();
            self.message = Text::new(
                "데모 화면 · 목록 전환, 선택, 스크롤, 제거 확인을 체험할 수 있습니다. 실제 작업은 실행되지 않습니다.",
                "Demo · Try list modes, selection, scrolling and uninstall confirmation. No real changes are made.",
            );
        } else {
            let (tx, rx) = mpsc::channel();
            self.receiver = Some(rx);
            self.busy = true;
            self.scanning = true;
            self.message = if self.mode == ListMode::All {
                Text::new(
                    "설치된 전체 패키지를 조회하고 있습니다…",
                    "Loading all installed packages…",
                )
            } else {
                Text::new(
                    "업데이트 가능한 패키지를 조회하고 있습니다…",
                    "Checking for available updates…",
                )
            };
            winget::scan(self.mode, tx);
        }
        cx.notify();
    }

    fn toggle_mode(&mut self, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        self.mode = if self.mode == ListMode::Updates {
            ListMode::All
        } else {
            ListMode::Updates
        };
        self.refresh(cx);
    }

    fn selected_packages(&self, operation: Operation) -> Vec<(usize, Package)> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.selected
                    && e.can_select()
                    && (operation == Operation::Uninstall || e.can_upgrade())
            })
            .map(|(i, e)| (i, e.package.clone()))
            .collect()
    }

    fn set_selection(&mut self, selected: bool, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        for entry in &mut self.entries {
            if entry.can_select() {
                entry.selected = selected;
            }
        }
        cx.notify();
    }

    fn request_uninstall(&mut self, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        let packages = self.selected_packages(Operation::Uninstall);
        if !packages.is_empty() {
            self.pending_uninstall = Some(packages);
            cx.notify();
        }
    }

    fn confirm_uninstall(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.demo {
            return;
        }
        if let Some(packages) = self.pending_uninstall.take() {
            // The confirmation refers to an immutable snapshot. Never remove a
            // package if the selected row changed after the preview was shown.
            if packages.iter().all(|(i, p)| {
                self.entries
                    .get(*i)
                    .is_some_and(|e| e.package == *p && e.can_select())
            }) {
                self.start_operation(Operation::Uninstall, packages, cx);
            } else {
                self.message = Text::new(
                    "목록이 변경되었습니다. 제거할 프로그램을 다시 선택하세요.",
                    "The list has changed. Select the programs to uninstall again.",
                );
                self.error = true;
                cx.notify();
            }
        }
    }

    fn start_operation(
        &mut self,
        operation: Operation,
        packages: Vec<(usize, Package)>,
        cx: &mut Context<Self>,
    ) {
        if self.locked() || self.demo || packages.is_empty() {
            return;
        }
        self.batch = packages.iter().map(|(i, _)| *i).collect();
        for (i, _) in &packages {
            self.entries[*i].status = Status::Queued;
        }
        self.stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        self.busy = true;
        self.scanning = false;
        self.operation = operation;
        self.error = false;
        self.show_logs = true;
        self.message = Text::new(
            format!(
                "선택한 {}개 패키지 {}를 시작합니다…",
                packages.len(),
                operation.label(Language::Korean)
            ),
            format!(
                "Starting {} for {} selected packages…",
                operation.label(Language::English),
                packages.len()
            ),
        );
        winget::execute(operation, packages, tx, self.stop.clone());
        cx.notify();
    }

    fn log(&mut self, message: impl Into<Text>) {
        if self.logs.len() >= 1000 {
            self.logs.pop_front();
        }
        self.logs.push_back(message.into().limited(4096));
    }

    fn poll(&mut self, cx: &mut Context<Self>) {
        let mut events = Vec::new();
        let mut disconnected = false;
        if let Some(rx) = &self.receiver {
            for _ in 0..256 {
                match rx.try_recv() {
                    Ok(event) => events.push(event),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }
        if events.is_empty() && !disconnected {
            return;
        }
        let mut complete = false;
        for event in events {
            match event {
                // Package queries update the list, without adding command or table output to logs.
                Event::Log(line) if !self.scanning => self.log(line),
                Event::Log(_) => {}
                Event::Scanned(result) => {
                    self.busy = false;
                    self.scanning = false;
                    complete = true;
                    match result {
                        Ok(packages) => {
                            self.message = Text::new(
                                format!(
                                    "{}개 패키지를 찾았습니다. 행을 클릭해 선택하세요.",
                                    packages.len()
                                ),
                                format!(
                                    "Found {} packages. Click a row to select it.",
                                    packages.len()
                                ),
                            );
                            self.entries = packages
                                .into_iter()
                                .map(|package| Entry {
                                    package,
                                    selected: false,
                                    status: Status::Ready,
                                })
                                .collect();
                        }
                        Err(error) => {
                            self.message = error.clone();
                            self.error = true;
                            self.show_logs = true;
                            self.log(error);
                        }
                    }
                }
                Event::Started(index) => {
                    if let Some(entry) = self.entries.get_mut(index) {
                        entry.status = Status::Running;
                        self.message = Text::new(
                            format!(
                                "{} {} 중…",
                                entry.package.name,
                                self.operation.label(Language::Korean)
                            ),
                            format!(
                                "{}: {} in progress…",
                                entry.package.name,
                                self.operation.label(Language::English)
                            ),
                        );
                    }
                }
                Event::Finished(index, result) => {
                    if let Some(entry) = self.entries.get_mut(index) {
                        let name = entry.package.name.clone();
                        match result {
                            Ok(()) => {
                                entry.status = Status::Success(self.operation);
                                entry.selected = false;
                                if self.operation == Operation::Upgrade {
                                    entry.package.installed =
                                        std::mem::take(&mut entry.package.available);
                                }
                                self.log(Text::new(
                                    format!(
                                        "[성공] {name} {}",
                                        self.operation.label(Language::Korean)
                                    ),
                                    format!(
                                        "[Success] {name}: {}",
                                        self.operation.label(Language::English)
                                    ),
                                ));
                            }
                            Err(error) => {
                                entry.status = Status::Failed(error.clone());
                                self.log(Text::new(
                                    format!("[실패] {name}: {}", error.ko),
                                    format!("[Failed] {name}: {}", error.en),
                                ));
                            }
                        }
                    }
                }
                Event::Done { cancelled } => {
                    complete = true;
                    self.busy = false;
                    for entry in &mut self.entries {
                        if matches!(entry.status, Status::Queued) {
                            entry.status = Status::Skipped;
                        }
                    }
                    let success = self
                        .batch
                        .iter()
                        .filter(|&&i| matches!(self.entries[i].status, Status::Success(_)))
                        .count();
                    let failed = self
                        .batch
                        .iter()
                        .filter(|&&i| matches!(self.entries[i].status, Status::Failed(_)))
                        .count();
                    self.message = Text::new(
                        format!(
                            "{} {} · 성공 {success}개 / 실패 {failed}개. 새로고침으로 실제 설치 상태를 확인하세요.",
                            self.operation.label(Language::Korean),
                            if cancelled { "중단" } else { "완료" }
                        ),
                        format!(
                            "{} {} · {success} succeeded / {failed} failed. Refresh to verify the installed state.",
                            self.operation.label(Language::English),
                            if cancelled { "stopped" } else { "complete" }
                        ),
                    );
                    self.error = failed > 0;
                    self.log(self.message.clone());
                }
            }
        }
        if disconnected && self.busy && !complete {
            self.busy = false;
            self.scanning = false;
            self.error = true;
            self.show_logs = true;
            self.message = Text::new(
                "작업 연결이 끊어졌습니다. 로그와 설치 상태를 확인하세요.",
                "The worker disconnected. Check the logs and installed state.",
            );
            for entry in &mut self.entries {
                if matches!(entry.status, Status::Queued | Status::Running) {
                    entry.status = Status::Skipped;
                }
            }
        }
        if complete || disconnected {
            self.receiver = None;
        }
        cx.notify();
    }

    fn scroll_handle(&self, target: ScrollTarget) -> &ScrollHandle {
        match target {
            ScrollTarget::Packages => &self.scroll,
            ScrollTarget::Logs => &self.log_scroll,
        }
    }

    fn scroll_geometry(&self, target: ScrollTarget) -> scrollbar::Geometry {
        let handle = self.scroll_handle(target);
        scrollbar::Geometry::scaled(
            handle.bounds().size.height.into(),
            handle.max_offset().height.into(),
            handle.offset().y.into(),
            self.ui_scale.factor(),
        )
    }

    fn drag_scroll(&mut self, target: ScrollTarget, y: gpui::Pixels, cx: &mut Context<Self>) {
        if let Some((active, grab)) = self.dragging {
            if active != target {
                return;
            }
            let handle = self.scroll_handle(target);
            let pointer = f32::from(y - handle.bounds().origin.y);
            let offset = self
                .scroll_geometry(target)
                .offset_for_pointer(pointer, grab);
            handle.set_offset(point(px(0.), px(offset)));
            cx.notify();
        }
    }

    fn render_scrollbar(&self, target: ScrollTarget, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme.palette();
        let handle = self.scroll_handle(target).clone();
        let weak = cx.weak_entity();
        div()
            .id(match target {
                ScrollTarget::Packages => "package-scrollbar",
                ScrollTarget::Logs => "log-scrollbar",
            })
            .w(ui_len(16.))
            .h_full()
            .flex_shrink_0()
            .bg(rgb(palette.surface))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    let geometry = this.scroll_geometry(target);
                    let y =
                        f32::from(event.position.y - this.scroll_handle(target).bounds().origin.y);
                    this.dragging = Some((
                        target,
                        if y >= geometry.thumb_top
                            && y <= geometry.thumb_top + geometry.thumb_height
                        {
                            y - geometry.thumb_top
                        } else {
                            geometry.thumb_height / 2.0
                        },
                    ));
                    this.drag_scroll(target, event.position.y, cx);
                    cx.stop_propagation();
                }),
            )
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let factor = f32::from(bounds.size.width) / 16.;
                        let geometry = scrollbar::Geometry::scaled(
                            bounds.size.height.into(),
                            handle.max_offset().height.into(),
                            handle.offset().y.into(),
                            factor,
                        );
                        let thumb = Bounds::new(
                            point(
                                bounds.origin.x + px(3. * factor),
                                bounds.origin.y + px(geometry.thumb_top),
                            ),
                            size(px(10. * factor), px(geometry.thumb_height)),
                        );
                        window.paint_quad(quad(
                            thumb,
                            px(5. * factor),
                            rgb(if handle.max_offset().height > px(0.) {
                                palette.scroll_thumb
                            } else {
                                palette.border
                            }),
                            px(0.),
                            gpui::transparent_black(),
                            Default::default(),
                        ));
                        let moved = weak.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                            if phase == DispatchPhase::Capture {
                                let _ = moved.update(cx, |this, cx| {
                                    if event.pressed_button == Some(MouseButton::Left) {
                                        this.drag_scroll(target, event.position.y, cx);
                                    } else {
                                        this.dragging = None;
                                    }
                                });
                            }
                        });
                        let released = weak.clone();
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                            if phase == DispatchPhase::Capture && event.button == MouseButton::Left
                            {
                                let _ = released.update(cx, |this, _| this.dragging = None);
                            }
                        });
                    },
                )
                .size_full(),
            )
    }

    fn render_theme_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme.palette();
        div()
            .id("theme-submenu")
            .absolute()
            .top_0()
            .right_full()
            .mr_2()
            .w(ui_len(250.))
            .p_1()
            .rounded_lg()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.panel))
            .shadow_lg()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .children(Theme::ALL.into_iter().enumerate().map(|(index, theme)| {
                let active = theme == self.theme;
                let preview = theme.palette();
                div()
                    .id(("theme-option", index))
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(ui_len(38.))
                    .px_3()
                    .rounded_md()
                    .bg(rgb(if active {
                        palette.selection
                    } else {
                        palette.panel
                    }))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(palette.hover)))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .size_4()
                                    .rounded_full()
                                    .border_2()
                                    .border_color(rgb(preview.accent))
                                    .bg(rgb(preview.background)),
                            )
                            .child(theme.name()),
                    )
                    .when(active, |d| d.child(icon(Icon::CheckSquare, palette.accent)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_theme(theme, cx);
                        cx.stop_propagation();
                    }))
            }))
    }

    fn render_language_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme.palette();
        div()
            .id("language-submenu")
            .absolute()
            .top_0()
            .right_full()
            .mr_2()
            .w(ui_len(190.))
            .p_1()
            .rounded_lg()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.panel))
            .shadow_lg()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .children(
                Language::ALL
                    .into_iter()
                    .enumerate()
                    .map(|(index, option)| {
                        let active = option == self.language;
                        div()
                            .id(("language-option", index))
                            .flex()
                            .items_center()
                            .justify_between()
                            .h(ui_len(38.))
                            .px_3()
                            .rounded_md()
                            .bg(rgb(if active {
                                palette.selection
                            } else {
                                palette.panel
                            }))
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(palette.hover)))
                            .child(option.name())
                            .when(active, |d| d.child(icon(Icon::CheckSquare, palette.accent)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.set_language(option, cx);
                                cx.stop_propagation();
                            }))
                    }),
            )
    }

    fn render_about(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme.palette();
        let language = self.language;
        let light = self.theme == Theme::GitHubLight;
        let link_color = if light { 0x16753b } else { 0x80d49d };
        let link_hover = if light { 0x0b5426 } else { 0xa4ecc1 };
        div().id("about-backdrop").absolute().inset_0().p_6()
            .flex().items_center().justify_center().occlude()
            .bg(gpui::rgba(0x000000b0))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.show_about = false;
                cx.stop_propagation();
                cx.notify();
            }))
            .child(div().id("about-dialog").w(ui_len(650.)).max_w_full()
                .flex().flex_col().gap_4().p_5().rounded_lg().shadow_lg()
                .bg(rgb(palette.panel)).border_1().border_color(rgb(palette.border)).occlude()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(div().flex().items_center().gap_3()
                    .child(div().text_lg().font_weight(gpui::FontWeight::BOLD)
                        .child(language.t("WUp 정보", "About WUp")))
                    .child(div().flex_1())
                    .child(div().text_xs().text_color(rgb(palette.muted))
                        .child(format!("v{}", env!("CARGO_PKG_VERSION"))))
                    .child(div().id("close-about").size_6().flex().items_center().justify_center()
                        .rounded_md().cursor_pointer().hover(|style| style.bg(rgb(palette.hover)))
                        .child(icon(Icon::Close, palette.muted))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.show_about = false;
                            cx.stop_propagation();
                            cx.notify();
                        }))))
                .child(div().text_base().text_color(rgb(palette.muted)).child(language.t(
                    "WUp은 가볍게 실행되는 Windows 패키지 관리자입니다. WinGet을 통해 설치된 프로그램과 업데이트 목록을 확인하고, 선택한 항목만 업데이트하거나 제거할 수 있습니다. Rust와 GPUI로 만들었습니다.",
                    "WUp is a lightweight Windows package manager. Browse installed programs and available WinGet updates, update only selected packages, and uninstall programs with confirmation. Built with Rust and GPUI.")))
                .child(div().flex().flex_wrap().items_center().gap_1().text_base()
                    .child(div().font_weight(gpui::FontWeight::BOLD)
                        .child(language.t("개발자 이메일:", "Developer email:")))
                    .child(div().id("about-email").text_color(rgb(link_color))
                        .font_weight(gpui::FontWeight::SEMIBOLD).cursor_pointer()
                        .hover(move |style| style.text_color(rgb(link_hover)))
                        .child("chanhwahong@gmail.com")
                        .on_click(|_, _, cx| { cx.open_url("mailto:chanhwahong@gmail.com"); cx.stop_propagation(); })))
                .child(div().flex().flex_wrap().items_center().gap_1().text_base()
                    .child(div().font_weight(gpui::FontWeight::BOLD).child("Git:"))
                    .child(div().id("about-github").text_color(rgb(link_color))
                        .font_weight(gpui::FontWeight::SEMIBOLD).cursor_pointer()
                        .hover(move |style| style.text_color(rgb(link_hover)))
                        .child("https://github.com/chanhwahong/wup")
                        .on_click(|_, _, cx| { cx.open_url("https://github.com/chanhwahong/wup"); cx.stop_propagation(); })))
                .child(div().flex().mt_1()
                    .child(div().id("about-coffee").flex().items_center().gap_2()
                        .px_3().py_2().rounded_xl().bg(rgb(0xff6433)).text_color(rgb(0xffffff))
                        .text_base().font_weight(gpui::FontWeight::BOLD).cursor_pointer()
                        .hover(|style| style.bg(rgb(0xef5124)))
                        .child(icon(Icon::Coffee, 0xffffff)).child("Buy me a coffee")
                        .on_click(|_, _, cx| { cx.open_url("https://ko-fi.com/chanhwahong"); cx.stop_propagation(); })))
                .child(div().id("icon-attribution").text_xs().text_color(rgb(palette.muted))
                    .cursor_pointer().child("Balloon: Twemoji (CC BY 4.0)")
                    .on_click(|_, _, cx| { cx.open_url("https://github.com/jdecked/twemoji"); cx.stop_propagation(); })))
    }

    fn render_confirmation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme.palette();
        let language = self.language;
        let packages = self
            .pending_uninstall
            .as_ref()
            .expect("confirmation requested");
        div().absolute().inset_0().flex().items_center().justify_center().bg(gpui::rgba(0x000000b0))
            .child(div().w(ui_len(650.)).max_h(ui_len(580.)).flex().flex_col().gap_4().p_6().rounded_lg().bg(rgb(palette.panel)).border_1().border_color(rgb(palette.danger))
                .child(div().text_lg().child(format!("{} {}{}", language.t("선택한", "Uninstall"), packages.len(), language.t("개 프로그램을 제거할까요?", " selected programs?"))))
                .child(div().text_sm().text_color(rgb(palette.muted)).child(language.t("아래 프로그램의 제거 프로그램을 실행합니다. 설치된 프로그램이 삭제되며, 관리자 승인이 필요할 수 있습니다.", "The uninstallers for the programs below will run. These programs will be removed. Administrator approval may be required.")))
                .child(div().id("uninstall-preview").max_h(ui_len(280.)).overflow_y_scroll().flex_shrink()
                    .children(packages.iter().map(|(_, p)| div().py_2().border_b_1().border_color(rgb(palette.border))
                        .child(p.name.clone()).child(div().text_xs().text_color(rgb(palette.muted)).child(format!("{} · {}", p.id, p.installed))))))
                .when(self.demo, |d| d.child(div().text_color(rgb(palette.accent)).child(language.t("데모 모드에서는 실제 제거를 실행할 수 없습니다.", "Uninstall is disabled in demo mode."))))
                .child(div().flex().justify_end().gap_3()
                    .child(button(palette, "cancel-uninstall", language.t("취소", "Cancel"), true, false).on_click(cx.listener(|this, _, _, cx| { this.pending_uninstall = None; cx.notify(); })))
                    .child(button(palette, "confirm-uninstall", language.t("제거 실행", "Uninstall now"), !self.demo, false).text_color(rgb(palette.danger))
                        .on_click(cx.listener(|this, _, _, cx| this.confirm_uninstall(cx))))))
    }
}

/// Convert a baseline 100%-size in pixels into zoom-aware layout units.
fn balloon_image() -> Arc<gpui::Image> {
    static IMAGE: std::sync::OnceLock<Arc<gpui::Image>> = std::sync::OnceLock::new();
    IMAGE
        .get_or_init(|| {
            Arc::new(gpui::Image::from_bytes(
                gpui::ImageFormat::Png,
                include_bytes!("../assets/balloon.png").to_vec(),
            ))
        })
        .clone()
}

fn ui_len(base_pixels: f32) -> gpui::Rems {
    rems(base_pixels / 16.)
}

fn scale_button(
    id: &'static str,
    label: &'static str,
    enabled: bool,
    palette: Palette,
) -> Stateful<Div> {
    div()
        .id(id)
        .px_2()
        .py_1()
        .min_w(ui_len(28.))
        .text_center()
        .text_sm()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.surface))
        .text_color(rgb(if enabled {
            palette.text
        } else {
            palette.disabled_text
        }))
        .when(enabled, |d| {
            d.cursor_pointer().hover(|s| s.bg(rgb(palette.hover)))
        })
        .child(label)
}

fn button(
    palette: Palette,
    id: &'static str,
    label: impl Into<String>,
    enabled: bool,
    primary: bool,
) -> Stateful<Div> {
    let label = label.into();
    let symbol = match id {
        "refresh" => Icon::Refresh,
        "about-email" => Icon::Mail,
        "about-github" => Icon::Link,
        "about-coffee" => Icon::Coffee,
        "theme" => Icon::Theme,
        "language" => Icon::Language,
        "select-toggle" => {
            if matches!(label.as_str(), "전체 해제" | "Deselect all") {
                Icon::CheckSquare
            } else {
                Icon::Square
            }
        }
        "toggle-all" => Icon::Packages,
        "uninstall" | "confirm-uninstall" => Icon::Trash,
        "upgrade" => Icon::Upgrade,
        "logs" => Icon::Log,
        "copy-logs" => Icon::Copy,
        "stop" => Icon::Stop,
        _ => Icon::Close,
    };
    let icon_color = if !enabled {
        palette.disabled_text
    } else if matches!(id, "uninstall" | "confirm-uninstall") {
        palette.danger
    } else if primary {
        palette.primary_text
    } else {
        palette.text
    };
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .flex_shrink_0()
        .px_4()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(if primary {
            palette.accent
        } else {
            palette.border
        }))
        .bg(rgb(if !enabled {
            palette.surface
        } else if primary {
            palette.primary
        } else {
            palette.surface
        }))
        .text_color(rgb(if !enabled {
            palette.disabled_text
        } else if primary {
            palette.primary_text
        } else {
            palette.text
        }))
        .text_sm()
        .when(enabled, |d| {
            d.cursor_pointer().hover(|s| {
                s.bg(rgb(if primary {
                    palette.primary_hover
                } else {
                    palette.hover
                }))
            })
        })
        .child(icon(symbol, icon_color))
        .child(label)
        .when(matches!(id, "language" | "theme"), |d| {
            d.child(icon(Icon::ChevronDown, icon_color))
        })
}

fn metric(palette: Palette, label: &'static str, value: usize, color: u32) -> Div {
    div()
        .flex_1()
        .p_3()
        .rounded_lg()
        .bg(rgb(palette.panel))
        .border_1()
        .border_color(rgb(palette.border))
        .child(div().text_xs().text_color(rgb(palette.muted)).child(label))
        .child(
            div()
                .mt_1()
                .text_2xl()
                .text_color(rgb(color))
                .child(value.to_string()),
        )
}

fn cell(text: String, width: f32, color: u32) -> Div {
    div()
        .w(ui_len(width))
        .flex_shrink_0()
        .text_sm()
        .text_color(rgb(color))
        .truncate()
        .child(text)
}

impl Render for Wup {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(frames) = &self.smoke_frames {
            frames.fetch_add(1, Ordering::Relaxed);
        }
        window.set_rem_size(px(16. * self.ui_scale.factor()));
        let palette = self.theme.palette();
        let language = self.language;
        let selected = self.selected_packages(Operation::Uninstall).len();
        let updatable_selected = self.selected_packages(Operation::Upgrade).len();
        let updatable = self.entries.iter().filter(|e| e.can_upgrade()).count();
        let unlocked = !self.locked();
        let selectable = self.entries.iter().filter(|e| e.can_select()).count();
        let all_selected = selectable > 0 && selected == selectable;
        let row_metrics =
            pixel_snap::row_metrics(64.0, self.ui_scale.factor(), window.scale_factor());
        let mut list = div()
            .id("packages")
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll);
        if self.entries.is_empty() {
            list = list.child(
                div()
                    .h(ui_len(160.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(palette.muted))
                    .child(if self.scanning {
                        language.t("패키지 조회 중…", "Loading packages…")
                    } else if self.error {
                        language.t(
                            "목록 조회에 실패했습니다. 실행 로그를 확인해 주세요.",
                            "Could not load packages. Check the execution log.",
                        )
                    } else if self.mode == ListMode::All {
                        language.t("설치된 패키지가 없습니다.", "No installed packages found.")
                    } else {
                        language.t(
                            "업데이트 가능한 패키지가 없습니다.",
                            "No updates available.",
                        )
                    }),
            );
        }
        for (index, entry) in self.entries.iter().enumerate() {
            let (label, color) = match &entry.status {
                Status::Ready if !entry.package.can_uninstall() => (
                    language
                        .t("ID 잘림 · 작업 불가", "Truncated ID · unavailable")
                        .into(),
                    palette.danger,
                ),
                Status::Ready if entry.can_upgrade() => (
                    language.t("업데이트 가능", "Update available").into(),
                    palette.accent,
                ),
                Status::Ready => (language.t("설치됨", "Installed").into(), palette.muted),
                Status::Queued => (language.t("대기 중", "Queued").into(), palette.accent),
                Status::Running => (String::new(), palette.accent),
                Status::Success(Operation::Upgrade) => (
                    language.t("업데이트 완료", "Updated").into(),
                    palette.success,
                ),
                Status::Success(Operation::Uninstall) => (
                    language.t("제거 완료", "Uninstalled").into(),
                    palette.success,
                ),
                Status::Failed(error) => (
                    format!(
                        "{} · {}",
                        language.t("실패", "Failed"),
                        error.value(language)
                    ),
                    palette.danger,
                ),
                Status::Skipped => (language.t("중단됨", "Stopped").into(), palette.muted),
            };
            let row = div()
                .id(("package", index))
                .relative()
                .flex()
                .items_center()
                .gap_3()
                .px_4()
                .h(px(row_metrics.content_height))
                .bg(rgb(if entry.selected {
                    palette.selection
                } else {
                    palette.panel
                }))
                .when(unlocked && entry.can_select(), |d| {
                    d.cursor_pointer().hover(|s| s.bg(rgb(palette.hover)))
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    if !this.locked() && this.dragging.is_none() {
                        if let Some(entry) = this.entries.get_mut(index) {
                            if entry.can_select() {
                                entry.selected = !entry.selected;
                            }
                        }
                        cx.notify();
                    }
                }))
                .child(
                    div()
                        .w_6()
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .child(icon(
                            if entry.selected {
                                Icon::CheckSquare
                            } else {
                                Icon::Square
                            },
                            if entry.selected {
                                palette.accent
                            } else {
                                palette.muted
                            },
                        )),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(div().truncate().child(entry.package.name.clone()))
                        .child(
                            div()
                                .mt_1()
                                .text_xs()
                                .text_color(rgb(palette.muted))
                                .truncate()
                                .child(entry.package.id.clone()),
                        ),
                )
                .child(cell(entry.package.installed.clone(), 115., palette.text))
                .child(cell(
                    if entry.package.available.is_empty() {
                        "—".into()
                    } else {
                        entry.package.available.clone()
                    },
                    115.,
                    palette.success,
                ))
                .child(cell(
                    if entry.package.source.is_empty() {
                        language.t("로컬", "Local").into()
                    } else {
                        entry.package.source.clone()
                    },
                    75.,
                    palette.muted,
                ))
                .child(if matches!(entry.status, Status::Running) {
                    div()
                        .w(ui_len(150.))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .child(spinner(("package-progress", index), color))
                        .into_any_element()
                } else {
                    cell(label, 150., color).into_any_element()
                });
            // Reserve separator space in the parent background. It cannot be
            // covered by the row content or culled as a zero-width border.
            let frame = div()
                .bg(rgb(palette.border))
                .pb(px(row_metrics.separator_height))
                .child(row)
                .when(self.smoke_gap_checks.is_some(), |frame| {
                    let measured = std::rc::Rc::new(std::cell::Cell::new(0.0_f32));
                    let record = measured.clone();
                    let checks = self.smoke_gap_checks.as_ref().unwrap().clone();
                    frame
                        .on_children_prepainted(move |bounds, _, _| {
                            if let Some(row) = bounds.first() {
                                record.set(row.size.height.into());
                            }
                        })
                        .child(
                            canvas(
                                |_, _, _| (),
                                move |bounds, _, window, _| {
                                    let gap = f32::from(bounds.size.height) - measured.get();
                                    assert!(
                                        gap * window.scale_factor() >= 0.99,
                                        "actual row separator collapsed: {gap}"
                                    );
                                    checks.fetch_add(1, Ordering::Relaxed);
                                },
                            )
                            .absolute()
                            .inset_0(),
                        )
                });
            list = list.child(frame);
        }
        let mut content = div().size_full().flex().flex_col().gap_4().p_6()
            .child(div().flex().flex_wrap().items_center().gap_3()
                 .child(div().size_10().flex().items_center().justify_center()
                    .child(gpui::img(balloon_image()).size_8()))
                .child(div().flex_1().child(div().text_xl().font_weight(gpui::FontWeight::BOLD).child("WUp"))
                    .child(div().text_xs().text_color(rgb(palette.muted)).child(language.t("Windows 패키지 · 업데이트 및 제거", "Windows packages · Update and uninstall"))))
                .child(button(palette, "refresh", language.t("새로고침", "Refresh"), unlocked, false).on_click(cx.listener(|this, _, _, cx| this.refresh(cx))))
                .child(div().relative().flex_shrink_0()
                    .child(div().id("settings-button").size_9().flex().items_center().justify_center().rounded_md().border_1()
                        .border_color(rgb(if self.show_settings_menu { palette.accent } else { palette.border }))
                        .bg(rgb(palette.surface))
                        .when(self.pending_uninstall.is_none(), |d| d.cursor_pointer().hover(|s| s.bg(rgb(palette.hover))))
                        .child(icon(Icon::More, palette.text))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.pending_uninstall.is_none() {
                                this.show_settings_menu = !this.show_settings_menu;
                                this.settings_panel = None;
                                cx.notify();
                            }
                        })))
                    .when(self.show_settings_menu && self.pending_uninstall.is_none(), |d| {
                        d.child(deferred(self.render_settings_menu(cx)).with_priority(2))
                    })))
            .child(div().flex().gap_3().child(metric(palette, language.t("표시한 패키지", "Packages shown"), self.entries.len(), palette.text))
                .child(metric(palette, language.t("업데이트 가능", "Update available"), updatable, palette.accent)).child(metric(palette, language.t("선택한 패키지", "Selected packages"), selected, palette.success)))
            .child(div().flex().flex_wrap().items_center().gap_2()
                .child(button(palette, "select-toggle", if all_selected { language.t("전체 해제", "Deselect all") } else { language.t("전체 선택", "Select all") }, unlocked && selectable > 0, false)
                    .on_click(cx.listener(|this, _, _, cx| {
                        let all_selected = this.entries.iter().filter(|e| e.can_select()).all(|e| e.selected);
                        this.set_selection(!all_selected, cx);
                    })))
                .child(button(palette, "toggle-all", language.t("전체 설치 패키지 보기", "Show all installed packages"), unlocked, false)
                    .when(self.mode == ListMode::All, |d| d.border_color(rgb(palette.accent)).text_color(rgb(palette.accent)))
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_mode(cx))))
                .child(div().flex_1())
                .when(self.busy && !self.scanning, |d| d.child(button(palette, "stop", language.t("완료 후 중단", "Stop after current"), !self.stop.load(Ordering::Relaxed), false)
                    .on_click(cx.listener(|this, _, _, cx| { this.stop.store(true, Ordering::Relaxed); this.message = Text::new("현재 작업이 끝나면 남은 작업을 중단합니다.", "Remaining tasks will stop after the current operation finishes."); cx.notify(); }))))
                .child(button(palette, "uninstall", format!("{} ({selected})", language.t("제거", "Uninstall")), unlocked && selected > 0, false)
                    .when(unlocked && selected > 0, |d| d.text_color(rgb(palette.danger)).border_color(rgb(palette.danger)))
                    .on_click(cx.listener(|this, _, _, cx| this.request_uninstall(cx))))
                .child(button(palette, "upgrade", format!("{} ({updatable_selected})", language.t("업데이트", "Update")), unlocked && updatable_selected > 0 && !self.demo, true)
                    .on_click(cx.listener(|this, _, _, cx| { let packages = this.selected_packages(Operation::Upgrade); this.start_operation(Operation::Upgrade, packages, cx); }))))
            .child(div().flex_1().min_h_0().flex().flex_col().rounded_lg().border_1().border_color(rgb(palette.border)).overflow_hidden()
                .child(div().flex().gap_3().px_4().py_3().pr(ui_len(32.)).bg(rgb(palette.surface)).text_xs().text_color(rgb(palette.muted))
                    .child(div().w_6().flex_shrink_0()).child(div().flex_1().child(language.t("프로그램 / 패키지 ID", "Program / Package ID")))
                    .child(cell(language.t("현재 버전", "Installed").into(), 115., palette.muted)).child(cell(language.t("새 버전", "Available").into(), 115., palette.muted))
                    .child(cell(language.t("원본", "Source").into(), 75., palette.muted)).child(cell(language.t("상태", "Status").into(), 150., palette.muted)))
                .child(div().flex_1().min_h_0().flex().child(list).child(self.render_scrollbar(ScrollTarget::Packages, cx))))
            .child(div().text_sm().text_color(rgb(if self.error { palette.danger } else { palette.muted })).child(self.message.value(language).to_owned()))
            .child(div().text_xs().text_color(rgb(palette.muted)).child(language.t("업데이트 실행 시 패키지 약관에 동의합니다. 프로그램 제거는 확인 후 실행합니다. 관리자 승인이 필요할 수 있습니다.", "Updating accepts package agreements. Uninstall requires confirmation. Administrator approval may be required.")))
            .child(div().flex().flex_wrap().items_center().gap_2()
                .child(button(palette, "logs", if self.show_logs { language.t("실행 로그 접기", "Hide logs") } else { language.t("실행 로그 보기", "Show logs") }, true, false)
                    .on_click(cx.listener(|this, _, _, cx| { this.show_logs = !this.show_logs; cx.notify(); })))
                .when(self.show_logs, |d| {
                    d.child(button(palette, "copy-logs", language.t("로그 복사", "Copy logs"), true, false)
                        .on_click(cx.listener(|this, _, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(this.logs.iter().map(|line| line.value(this.language)).collect::<Vec<_>>().join("\n")));
                        })))
                })
                .child(div().flex_1()));

        if self.show_logs {
            content = content.child(
                div()
                    .h(ui_len(145.))
                    .flex_shrink_0()
                    .flex()
                    .rounded_md()
                    .overflow_hidden()
                    .bg(rgb(palette.log_background))
                    .child(
                        div()
                            .id("log-output")
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .overflow_y_scroll()
                            .track_scroll(&self.log_scroll)
                            .p_3()
                            .rounded_md()
                            .bg(rgb(palette.log_background))
                            .font_family("Consolas")
                            .text_xs()
                            .text_color(rgb(palette.muted))
                            .child(div().mb_2().text_color(rgb(palette.accent)).child(
                                language.t("최근 로그부터 표시 · 복사 시 시간순 전체 로그 (최대 1,000줄)", "Newest first · Copy includes all logs in chronological order (up to 1,000 lines)"),
                            ))
                            .children(
                                self.logs
                                    .iter()
                                    .rev()
                                    .take(120)
                                    .map(|line| div().mb_1().child(line.value(language).to_owned())),
                            ),
                    )
                    .child(self.render_scrollbar(ScrollTarget::Logs, cx)),
            );
        }
        div()
            .relative()
            .size_full()
            .bg(rgb(palette.background))
            .font_family("Malgun Gothic")
            .text_color(rgb(palette.text))
            .text_sm()
            .child(content)
            .when(
                self.show_settings_menu && self.pending_uninstall.is_none(),
                |d| {
                    d.child(
                        deferred(
                            div()
                                .id("settings-menu-backdrop")
                                .absolute()
                                .inset_0()
                                .occlude()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _, cx| {
                                        this.close_settings();
                                        cx.stop_propagation();
                                        cx.notify();
                                    }),
                                )
                                .on_mouse_down(
                                    MouseButton::Right,
                                    cx.listener(|this, _, _, cx| {
                                        this.close_settings();
                                        cx.stop_propagation();
                                        cx.notify();
                                    }),
                                ),
                        )
                        .with_priority(1),
                    )
                },
            )
            .when(self.pending_uninstall.is_some(), |d| {
                d.child(self.render_confirmation(cx))
            })
            .when(self.show_about, |d| {
                d.child(deferred(self.render_about(cx)).with_priority(3))
            })
    }
}

fn demo_packages() -> Vec<Package> {
    (0..45)
        .map(|i| Package {
            name: format!(
                "{} {:02}",
                if i % 3 == 0 {
                    "로컬 설치 프로그램"
                } else {
                    "Demo Application"
                },
                i + 1
            ),
            id: if i % 3 == 0 {
                format!("ARP\\Machine\\X64\\Local App {}", i + 1)
            } else {
                format!("Example.App{}", i + 1)
            },
            installed: "1.0.0".into(),
            available: if i % 3 == 0 {
                String::new()
            } else {
                "2.0.0".into()
            },
            source: if i % 3 == 0 {
                String::new()
            } else {
                "winget".into()
            },
        })
        .collect()
}

/// Render every cascading menu at supported zoom extremes in a real GPUI
/// window. Demo data only; this never runs WinGet or saves user preferences.
fn smoke_test_menus(view: &gpui::Entity<Wup>, cx: &mut App) {
    let frames = Arc::new(AtomicUsize::new(0));
    let gap_checks = Arc::new(AtomicUsize::new(0));
    view.update(cx, |app, _| {
        // Exercise the live progress ring without running WinGet.
        if let Some(entry) = app.entries.first_mut() {
            entry.status = Status::Running;
        }
        app.smoke_frames = Some(frames.clone());
        app.smoke_gap_checks = Some(gap_checks.clone());
    });
    let weak = view.downgrade();
    cx.spawn(async move |cx| {
        for percent in [80, 95, 100, 120] {
            for panel in [
                None,
                Some(SettingsPanel::Theme),
                Some(SettingsPanel::Language),
                Some(SettingsPanel::Scale),
            ] {
                let before = frames.load(Ordering::Relaxed);
                weak.update(cx, |app, cx| {
                    app.ui_scale = UiScale::new(percent);
                    if percent == 95 { app.scroll.set_offset(point(px(0.), px(-17.35))); }
                    app.show_settings_menu = true;
                    app.settings_panel = panel;
                    cx.notify();
                })
                .expect("smoke-test window must remain alive");
                for _ in 0..60 {
                    cx.background_executor()
                        .timer(Duration::from_millis(50))
                        .await;
                    if frames.load(Ordering::Relaxed) > before {
                        break;
                    }
                }
                assert!(
                    frames.load(Ordering::Relaxed) > before,
                    "menu smoke test did not render a frame"
                );
            }
        }
        // Exercise About in both languages, light/dark palettes and zoom limits.
        for percent in [80, 95, 100, 120] {
            for language in Language::ALL {
                let before = frames.load(Ordering::Relaxed);
                weak.update(cx, |app, cx| {
                    app.close_settings();
                    app.ui_scale = UiScale::new(percent);
                    app.language = language;
                    app.theme = if language == Language::English { Theme::GitHubLight } else { Theme::TokyoNight };
                    app.show_about = true;
                    cx.notify();
                }).expect("About smoke-test window must remain alive");
                for _ in 0..60 {
                    cx.background_executor().timer(Duration::from_millis(50)).await;
                    if frames.load(Ordering::Relaxed) > before { break; }
                }
                assert!(frames.load(Ordering::Relaxed) > before, "About dialog did not render");
            }
        }
        weak.update(cx, |app, cx| {
            app.show_about = false;
            assert!(!app.locked(), "closing About must restore interaction");
            cx.notify();
        }).expect("About smoke-test window must remain alive");
        assert!(gap_checks.load(Ordering::Relaxed) > 0, "no actual row gaps were checked");
        println!("Rendering smoke test passed: main/theme/language/scale at 80%, 95%, 100%, 120%, including fractional scrolling and About in Korean/English with light/dark themes; {} actual row gaps checked.", gap_checks.load(Ordering::Relaxed));
        let _ = cx.update(|cx| cx.quit());
    })
    .detach();
}

fn main() {
    let smoke = std::env::args().any(|arg| arg == "--smoke-test-menus");
    let demo = smoke || std::env::args().any(|arg| arg == "--demo");
    Application::new().run(move |cx: &mut App| {
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let bounds = Bounds::centered(None, size(px(1220.), px(900.)), cx);
        let result = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(1080.), px(820.))),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some(format!("WUp v{} | WinGet Package Manager", env!("CARGO_PKG_VERSION")).into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| Wup::new(demo, cx));
                if smoke { smoke_test_menus(&view, cx); }
                let weak = view.downgrade();
                window.on_window_should_close(cx, move |_, cx| {
                    weak.update(cx, |this, cx| {
                        if this.busy {
                            if !this.scanning {
                                this.stop.store(true, Ordering::Relaxed);
                            }
                            this.message = Text::new("현재 작업이 끝난 뒤 창을 닫아 주세요. 남은 작업은 중단합니다.", "Close the window after the current operation finishes. Remaining tasks will stop.");
                            cx.notify();
                            false
                        } else {
                            true
                        }
                    })
                    .unwrap_or(true)
                });
                view
            },
        );
        if let Err(error) = result {
            eprintln!("{}: {error}", Language::load().t("WUp 창을 열지 못했습니다", "Could not open the WUp window"));
            cx.quit();
        }
        cx.activate(true);
    });
}
