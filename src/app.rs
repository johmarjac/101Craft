//! The main window of 101Craft: state, files, search, compare and the frame loop.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use egui::{Color32, Key, KeyboardShortcut, Modifiers, Theme, Ui};

use crate::doc::Document;
use crate::i18n::Lang;
use crate::template;
use crate::tools::{self, DiffRange, SearchKind};

use crate::tr;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BottomTab {
    Template,
    Search,
    Compare,
    Histogram,
    Output,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RightTab {
    Inspector,
    Bookmarks,
    Checksum,
}

pub struct FindState {
    pub open: bool,
    pub replace_mode: bool,
    pub text: String,
    pub replace: String,
    pub kind: SearchKind,
    pub ignore_case: bool,
    pub message: String,
    pub focus: bool,
}

pub struct CompareState {
    pub open: bool,
    pub a: usize,
    pub b: usize,
    pub result: Vec<DiffRange>,
    pub pair: Option<(usize, usize)>,
    pub split: bool,
    pub last_top: usize,
}

pub struct TemplateEditor {
    pub open: bool,
    pub name: String,
    pub source: String,
}

pub struct Shot {
    pub path: PathBuf,
    pub requested: bool,
    pub image: Arc<Mutex<Option<Arc<egui::ColorImage>>>>,
}

pub struct App {
    pub docs: Vec<Document>,
    pub active: usize,
    pub untitled: usize,
    pub bytes_per_row: usize,
    pub font_size: f32,
    pub insert: bool,
    pub little_endian: bool,
    pub dark: bool,
    pub auto_template: bool,
    pub lang: Lang,
    pub find: FindState,
    pub goto_open: bool,
    pub goto_text: String,
    pub goto_focus: bool,
    pub compare: CompareState,
    pub tpl_editor: TemplateEditor,
    pub bottom_tab: BottomTab,
    pub right_tab: RightTab,
    pub output: Vec<String>,
    pub insp_edit: Option<(usize, String)>,
    pub tpl_edit: Option<(String, String)>,
    pub checksum_selection_only: bool,
    pub checksum_results: Vec<(String, String)>,
    pub histogram: Option<(usize, u64, Box<[u64; 256]>)>,
    pub hovered_byte: Option<usize>,
    pub hex_focused: bool,
    pub focus_hex: bool,
    pub insert_dialog: Option<(String, String)>,
    pub fill_dialog: Option<String>,
    pub confirm_close: Option<usize>,
    pub confirm_quit: bool,
    pub quitting: bool,
    pub about_open: bool,
    pub keys_open: bool,
    pub status: String,
    pub frame: u64,
    pub shot: Option<Shot>,
    pub styled: bool,
}

const DEFAULT_TEMPLATE: &str = "// Custom template\nLittleEndian();\n\ntypedef struct {\n    char  magic[4];\n    uint  version;\n    uint  count;\n} HEADER;\n\nHEADER header;\n";

impl App {
    pub fn new(files: Vec<PathBuf>, screenshot: Option<PathBuf>, scene: Option<String>) -> Self {
        let mut app = App {
            docs: Vec::new(),
            active: 0,
            untitled: 0,
            bytes_per_row: 16,
            font_size: 13.0,
            insert: false,
            little_endian: true,
            dark: false,
            auto_template: true,
            lang: crate::i18n::load(),
            find: FindState {
                open: false,
                replace_mode: false,
                text: String::new(),
                replace: String::new(),
                kind: SearchKind::Text,
                ignore_case: false,
                message: String::new(),
                focus: false,
            },
            goto_open: false,
            goto_text: String::new(),
            goto_focus: false,
            compare: CompareState {
                open: false,
                a: 0,
                b: 0,
                result: Vec::new(),
                pair: None,
                split: true,
                last_top: 0,
            },
            tpl_editor: TemplateEditor {
                open: false,
                name: tr!(crate::i18n::load(), "dlg.tpled.name.default").into(),
                source: DEFAULT_TEMPLATE.into(),
            },
            bottom_tab: BottomTab::Template,
            right_tab: RightTab::Inspector,
            output: vec![tr!(crate::i18n::load(), "msg.ready").into()],
            insp_edit: None,
            tpl_edit: None,
            checksum_selection_only: false,
            checksum_results: Vec::new(),
            histogram: None,
            hovered_byte: None,
            hex_focused: false,
            focus_hex: true,
            insert_dialog: None,
            fill_dialog: None,
            confirm_close: None,
            confirm_quit: false,
            quitting: false,
            about_open: false,
            keys_open: false,
            status: String::new(),
            frame: 0,
            shot: screenshot.map(|path| Shot {
                path,
                requested: false,
                image: Arc::new(Mutex::new(None)),
            }),
            styled: false,
        };
        for f in files {
            app.open_path(f);
        }
        if app.docs.is_empty() {
            let name = tr!(app.lang, "scene.sample.bmp");
            let mut d = Document::new(name, crate::sample::sample_bmp(), None);
            d.bookmarks.push(crate::doc::Bookmark {
                name: tr!(app.lang, "msg.signature").into(),
                start: 0,
                len: 2,
                color: Color32::from_rgb(255, 214, 120),
            });
            app.docs.push(d);
            app.active = 0;
            app.auto_run_template(0);
            app.docs[0].set_cursor(0x12, false);
            app.log(tr!(
                app.lang,
                "msg.sample",
                name = tr!(app.lang, "scene.sample.bmp")
            ));
        }
        if let Some(s) = scene {
            crate::scenes::apply(&mut app, &s);
        }
        app
    }

    pub fn doc(&self) -> Option<&Document> {
        self.docs.get(self.active)
    }

    pub fn doc_mut(&mut self) -> Option<&mut Document> {
        self.docs.get_mut(self.active)
    }

    pub fn log(&mut self, s: impl Into<String>) {
        let s = s.into();
        self.status = s.clone();
        self.output.push(s);
        if self.output.len() > 2000 {
            self.output.drain(..500);
        }
    }

    // ------------------------------------------------------------ files

    pub fn open_path(&mut self, path: PathBuf) {
        if let Some(i) = self.docs.iter().position(|d| d.path.as_ref() == Some(&path)) {
            self.active = i;
            return;
        }
        match std::fs::read(&path) {
            Ok(data) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| tr!(self.lang, "msg.filefallback").into());
                let size = data.len();
                self.docs.push(Document::new(name.clone(), data, Some(path)));
                self.active = self.docs.len() - 1;
                self.log(tr!(
                    self.lang,
                    "msg.opened",
                    name = name,
                    size = tools::human_size(size, self.lang)
                ));
                if self.auto_template {
                    self.auto_run_template(self.active);
                }
                self.focus_hex = true;
            }
            Err(e) => self.log(tr!(
                self.lang,
                "msg.openfail",
                path = path.display(),
                e = e
            )),
        }
    }

    pub fn open_dialog(&mut self) {
        let title = tr!(self.lang, "msg.open").to_string();
        if let Some(files) = rfd::FileDialog::new().set_title(title).pick_files() {
            for f in files {
                self.open_path(f);
            }
        }
    }

    pub fn new_doc(&mut self) {
        self.untitled += 1;
        let name = tr!(self.lang, "msg.untitled", n = self.untitled);
        self.docs.push(Document::new(name, Vec::new(), None));
        self.active = self.docs.len() - 1;
        self.focus_hex = true;
    }

    pub fn save(&mut self, idx: usize, save_as: bool) -> bool {
        let Some(doc) = self.docs.get(idx) else { return false };
        let path = if save_as || doc.path.is_none() {
            let title = tr!(self.lang, "msg.saveas").to_string();
            match rfd::FileDialog::new()
                .set_title(title)
                .set_file_name(&doc.name)
                .save_file()
            {
                Some(p) => p,
                None => return false,
            }
        } else {
            doc.path.clone().unwrap()
        };
        match std::fs::write(&path, &doc.data) {
            Ok(()) => {
                let doc = &mut self.docs[idx];
                doc.path = Some(path.clone());
                doc.name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                doc.mark_saved();
                self.log(tr!(self.lang, "msg.saved", path = path.display()));
                true
            }
            Err(e) => {
                self.log(tr!(self.lang, "msg.savefail", e = e));
                false
            }
        }
    }

    pub fn close_doc(&mut self, idx: usize, force: bool) {
        if idx >= self.docs.len() {
            return;
        }
        if !force && self.docs[idx].modified {
            self.confirm_close = Some(idx);
            return;
        }
        if let Some((a, b)) = self.compare.pair {
            if a == idx || b == idx {
                self.end_compare();
            } else {
                let fix = |x: usize| if x > idx { x - 1 } else { x };
                self.compare.pair = Some((fix(a), fix(b)));
            }
        }
        self.docs.remove(idx);
        if self.active >= self.docs.len() {
            self.active = self.docs.len().saturating_sub(1);
        } else if self.active > idx {
            self.active -= 1;
        }
        self.histogram = None;
    }

    // ------------------------------------------------------------ templates

    pub fn auto_run_template(&mut self, idx: usize) {
        let Some(doc) = self.docs.get(idx) else { return };
        if let Some(b) = template::detect(&doc.name, &doc.data) {
            self.run_template(idx, b.name, b.source);
        }
    }

    pub fn run_template(&mut self, idx: usize, name: &str, src: &str) {
        let lang = self.lang;
        let Some(doc) = self.docs.get_mut(idx) else { return };
        let started = std::time::Instant::now();
        let mut res = template::run(name, src, &doc.data, lang);
        res.revision = doc.revision;
        if let Some(old) = &doc.template {
            if old.name == res.name {
                res.expanded = old.expanded.clone();
            }
        }
        doc.highlights = res.highlights();
        let ms = started.elapsed().as_millis();
        let mut lines = vec![tr!(
            lang,
            "msg.tpl.run",
            name = crate::i18n::get(lang, name),
            doc = doc.name,
            n = res.nodes.len(),
            ms = ms
        )];
        lines.extend(res.log.iter().cloned());
        if let Some(e) = &res.error {
            lines.push(tr!(lang, "msg.tpl.error", e = e));
        }
        doc.template = Some(res);
        for l in lines {
            self.log(l);
        }
        self.bottom_tab = BottomTab::Template;
    }

    pub fn rerun_template_if_stale(&mut self) {
        let idx = self.active;
        let lang = self.lang;
        let Some(doc) = self.docs.get_mut(idx) else { return };
        let Some(t) = &doc.template else { return };
        if t.revision != doc.revision && doc.data.len() < 16 * 1024 * 1024 {
            let (n, s, exp) = (t.name.clone(), t.source.clone(), t.expanded.clone());
            let mut res = template::run(&n, &s, &doc.data, lang);
            res.revision = doc.revision;
            res.expanded = exp;
            doc.highlights = res.highlights();
            doc.template = Some(res);
        }
    }

    pub fn run_template_file(&mut self) {
        let title = tr!(self.lang, "msg.tpl.runfile").to_string();
        let filter = tr!(self.lang, "dlg.runfilter").to_string();
        if let Some(p) = rfd::FileDialog::new()
            .set_title(title)
            .add_filter(filter, &["tpl", "bt", "txt"])
            .pick_file()
        {
            match std::fs::read_to_string(&p) {
                Ok(src) => {
                    let name = p
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    self.tpl_editor.name = name.clone();
                    self.tpl_editor.source = src.clone();
                    self.run_template(self.active, &name, &src);
                }
                Err(e) => self.log(tr!(self.lang, "msg.tpl.readfail", e = e)),
            }
        }
    }

    pub fn rerun_or_detect(&mut self) {
        let idx = self.active;
        let Some(doc) = self.docs.get(idx) else { return };
        if let Some(t) = &doc.template {
            let (n, s) = (t.name.clone(), t.source.clone());
            self.run_template(idx, &n, &s);
        } else if template::detect(&doc.name, &doc.data).is_some() {
            self.auto_run_template(idx);
        } else {
            self.log(tr!(self.lang, "msg.tpl.nomatch"));
        }
    }

    // ------------------------------------------------------------ search

    fn ci(&self) -> bool {
        self.find.ignore_case && matches!(self.find.kind, SearchKind::Text)
    }

    pub fn find_pattern(&mut self) -> Option<Vec<u8>> {
        let p = tools::pattern(&self.find.text, self.find.kind, self.little_endian);
        if p.is_none() {
            self.find.message = if self.find.text.is_empty() {
                tr!(self.lang, "msg.find.empty").into()
            } else {
                tr!(self.lang, "msg.find.badinput").into()
            };
        }
        p
    }

    pub fn find_step(&mut self, forward: bool) {
        let Some(pat) = self.find_pattern() else { return };
        let ci = self.ci();
        let Some(doc) = self.docs.get_mut(self.active) else { return };
        let from = if forward {
            match doc.selection() {
                Some((s, _)) => s + 1,
                None => doc.cursor,
            }
        } else {
            doc.selection().map(|(s, _)| s).unwrap_or(doc.cursor)
        };
        let r = if forward {
            tools::find_next(&doc.data, &pat, from, ci)
        } else {
            tools::find_prev(&doc.data, &pat, from, ci)
        };
        match r {
            Some(p) => {
                doc.select_range(p, pat.len());
                self.find.message = tr!(self.lang, "msg.find.found", p = format!("{p:X}"));
            }
            None => self.find.message = tr!(self.lang, "msg.find.notfound").into(),
        }
    }

    pub fn find_all(&mut self) {
        let Some(pat) = self.find_pattern() else { return };
        let ci = self.ci();
        let Some(doc) = self.docs.get_mut(self.active) else { return };
        doc.hits = tools::find_all(&doc.data, &pat, ci, 100_000);
        let n = doc.hits.len();
        if let Some(&(p, l)) = doc.hits.first() {
            doc.select_range(p, l);
        }
        self.find.message = tr!(self.lang, "msg.find.hits", n = n);
        self.bottom_tab = BottomTab::Search;
        let q = self.find.text.clone();
        self.log(tr!(self.lang, "msg.find.log", q = q, n = n));
    }

    pub fn replace_one(&mut self) {
        let Some(pat) = self.find_pattern() else { return };
        let rep = match tools::pattern(&self.find.replace, self.find.kind, self.little_endian) {
            Some(r) => r,
            None if self.find.replace.is_empty() => Vec::new(),
            None => {
                self.find.message = tr!(self.lang, "msg.replace.invalid").into();
                return;
            }
        };
        let ci = self.ci();
        let Some(doc) = self.docs.get_mut(self.active) else { return };
        if let Some((s, e)) = doc.selection() {
            let cur = &doc.data[s..e];
            let same = cur.len() == pat.len()
                && cur
                    .iter()
                    .zip(&pat)
                    .all(|(a, b)| if ci { a.eq_ignore_ascii_case(b) } else { a == b });
            if same {
                doc.splice(s, e - s, &rep);
                doc.cursor = s + rep.len();
                doc.anchor = None;
            }
        }
        self.find_step(true);
    }

    pub fn replace_all(&mut self) {
        let Some(pat) = self.find_pattern() else { return };
        let rep = tools::pattern(&self.find.replace, self.find.kind, self.little_endian).unwrap_or_default();
        let ci = self.ci();
        let Some(doc) = self.docs.get_mut(self.active) else { return };
        let hits = tools::find_all(&doc.data, &pat, ci, usize::MAX);
        if hits.is_empty() {
            self.find.message = tr!(self.lang, "msg.find.notfound").into();
            return;
        }
        let mut out = Vec::with_capacity(doc.data.len());
        let mut last = 0;
        for &(p, l) in &hits {
            out.extend_from_slice(&doc.data[last..p]);
            out.extend_from_slice(&rep);
            last = p + l;
        }
        out.extend_from_slice(&doc.data[last..]);
        let len = doc.data.len();
        doc.splice(0, len, &out);
        doc.cursor = 0;
        self.find.message = tr!(self.lang, "msg.replace.done", n = hits.len());
        let m = self.find.message.clone();
        self.log(m);
    }

    // ------------------------------------------------------------ compare

    pub fn open_compare_dialog(&mut self) {
        self.compare.open = true;
        self.compare.a = self.active;
        self.compare.b = if self.docs.len() > 1 {
            (self.active + 1) % self.docs.len()
        } else {
            self.active
        };
    }

    pub fn run_compare(&mut self) {
        let (a, b) = (self.compare.a, self.compare.b);
        if a >= self.docs.len() || b >= self.docs.len() || a == b {
            self.log(tr!(self.lang, "msg.cmp.need2"));
            return;
        }
        let res = tools::compare(&self.docs[a].data, &self.docs[b].data, 100_000);
        self.docs[a].diff_ranges = res.iter().map(|d| (d.start, d.start + d.len_a)).collect();
        self.docs[b].diff_ranges = res.iter().map(|d| (d.start, d.start + d.len_b)).collect();
        let n = res.len();
        let total: usize = res.iter().map(|d| d.len_a.max(d.len_b)).sum();
        let msg = tr!(
            self.lang,
            "msg.cmp.done",
            a = self.docs[a].name,
            b = self.docs[b].name,
            n = n,
            size = tools::human_size(total, self.lang)
        );
        self.compare.result = res;
        self.compare.pair = Some((a, b));
        self.compare.open = false;
        self.bottom_tab = BottomTab::Compare;
        self.log(msg);
    }

    pub fn end_compare(&mut self) {
        if let Some((a, b)) = self.compare.pair.take() {
            for i in [a, b] {
                if let Some(d) = self.docs.get_mut(i) {
                    d.diff_ranges.clear();
                }
            }
        }
        self.compare.result.clear();
    }

    // ------------------------------------------------------------ misc actions

    pub fn open_find(&mut self, replace: bool) {
        self.find.open = true;
        self.find.replace_mode = replace;
        self.find.focus = true;
    }

    pub fn open_goto(&mut self) {
        self.goto_open = true;
        self.goto_focus = true;
    }

    pub fn toggle_bookmark(&mut self) {
        let n = self.active;
        let lang = self.lang;
        if let Some(d) = self.doc_mut() {
            d.toggle_bookmark(n, lang);
        }
    }

    pub fn undo(&mut self) {
        if let Some(d) = self.doc_mut() {
            d.undo();
        }
    }

    pub fn redo(&mut self) {
        if let Some(d) = self.doc_mut() {
            d.redo();
        }
    }

    pub fn next_bookmark(&mut self, forward: bool) {
        let Some(d) = self.doc_mut() else { return };
        if d.bookmarks.is_empty() {
            return;
        }
        let cur = d.cursor;
        let target = if forward {
            d.bookmarks.iter().find(|b| b.start > cur).or(d.bookmarks.first())
        } else {
            d.bookmarks.iter().rev().find(|b| b.start < cur).or(d.bookmarks.last())
        };
        if let Some(b) = target.cloned() {
            d.set_cursor(b.start, false);
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        let cmd = Modifiers::COMMAND;
        let sc = |m: Modifiers, k: Key| ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)));
        if sc(cmd, Key::O) {
            self.open_dialog();
        }
        if sc(cmd | Modifiers::SHIFT, Key::S) {
            self.save(self.active, true);
        }
        if sc(cmd, Key::S) {
            self.save(self.active, false);
        }
        if sc(cmd, Key::N) {
            self.new_doc();
        }
        if sc(cmd, Key::W) {
            self.close_doc(self.active, false);
        }
        if sc(cmd, Key::F) {
            self.open_find(false);
        }
        if sc(cmd, Key::H) {
            self.open_find(true);
        }
        if sc(cmd, Key::G) {
            self.open_goto();
        }
        if sc(Modifiers::SHIFT, Key::F3) {
            self.find_step(false);
        }
        if sc(Modifiers::NONE, Key::F3) {
            self.find_step(true);
        }
        if sc(Modifiers::NONE, Key::F5) {
            self.rerun_or_detect();
        }
        let text_editing = ctx.egui_wants_keyboard_input() && !self.hex_focused;
        if !text_editing {
            if sc(cmd | Modifiers::SHIFT, Key::Z) || sc(cmd, Key::Y) {
                self.redo();
            }
            if sc(cmd, Key::Z) {
                self.undo();
            }
            if sc(cmd, Key::B) {
                self.toggle_bookmark();
            }
            if sc(Modifiers::SHIFT, Key::F2) {
                self.next_bookmark(false);
            }
            if sc(Modifiers::NONE, Key::F2) {
                self.next_bookmark(true);
            }
        }
        if sc(cmd, Key::Tab) && !self.docs.is_empty() {
            self.active = (self.active + 1) % self.docs.len();
        }
    }

    fn handle_drops(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        for p in dropped {
            self.open_path(p);
        }
    }

    fn style(&mut self, ctx: &egui::Context) {
        if self.styled {
            return;
        }
        self.styled = true;
        ctx.set_theme(if self.dark { Theme::Dark } else { Theme::Light });
        ctx.all_styles_mut(|s| {
            s.spacing.item_spacing = egui::vec2(6.0, 4.0);
            s.spacing.button_padding = egui::vec2(6.0, 2.0);
        });
        ctx.style_mut_of(Theme::Light, |s| {
            s.visuals.panel_fill = Color32::from_rgb(241, 244, 248);
            s.visuals.window_fill = Color32::from_rgb(250, 251, 253);
            s.visuals.faint_bg_color = Color32::from_rgb(233, 237, 243);
            s.visuals.extreme_bg_color = Color32::WHITE;
            s.visuals.selection.bg_fill = Color32::from_rgb(45, 120, 215);
        });
        ctx.style_mut_of(Theme::Dark, |s| {
            s.visuals.selection.bg_fill = Color32::from_rgb(38, 110, 200);
        });
    }

    fn screenshot_step(&mut self, ctx: &egui::Context) {
        let Some(shot) = &mut self.shot else { return };
        ctx.request_repaint();
        if self.frame >= 120 && !shot.requested {
            shot.requested = true;
            let slot = shot.image.clone();
            ctx.request_screenshot(move |img| {
                *slot.lock().unwrap() = Some(img);
            });
        }
        let img = shot.image.lock().unwrap().take();
        if let Some(img) = img {
            let [w, h] = img.size;
            let mut buf = Vec::with_capacity(w * h * 4);
            for p in &img.pixels {
                let [r, g, b, _] = p.to_array();
                buf.extend_from_slice(&[r, g, b, 255]);
            }
            if let Some(parent) = shot.path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match image::RgbaImage::from_raw(w as u32, h as u32, buf) {
                Some(im) => {
                    if let Err(e) = im.save(&shot.path) {
                        eprintln!(
                            "{}",
                            tr!(self.lang, "msg.shot.savefail", e = e)
                        );
                    }
                }
                None => eprintln!("{}", tr!(self.lang, "msg.shot.badsize")),
            }
            self.quitting = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if self.frame > 1500 {
            eprintln!("{}", tr!(self.lang, "msg.shot.timeout"));
            self.quitting = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.style(&ctx);
        self.frame += 1;

        // Ask before quitting with unsaved work.
        if ctx.input(|i| i.viewport().close_requested())
            && !self.quitting
            && self.shot.is_none()
            && self.docs.iter().any(|d| d.modified)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_quit = true;
        }

        self.handle_drops(&ctx);
        self.shortcuts(&ctx);
        self.rerun_template_if_stale();
        egui::Panel::top("menu").show(ui, |ui| {
            self.menu_bar(ui);
        });
        egui::Panel::top("toolbar").show(ui, |ui| {
            self.toolbar(ui);
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            self.status_bar(ui);
        });
        egui::Panel::right("inspector")
            .resizable(true)
            .default_size(300.0)
            .size_range(250.0..=640.0)
            .show(ui, |ui| {
                crate::panels::right_panel(self, ui);
            });
        egui::Panel::left("workspace")
            .resizable(true)
            .default_size(190.0)
            .size_range(140.0..=400.0)
            .show(ui, |ui| {
                crate::panels::workspace(self, ui);
            });
        egui::Panel::bottom("results")
            .resizable(true)
            .default_size(230.0)
            .size_range(120.0..=900.0)
            .show(ui, |ui| {
                crate::panels::bottom_panel(self, ui);
            });
        egui::CentralPanel::default().show(ui, |ui| {
            self.central(ui);
        });
        self.dialogs(&ctx);
        self.screenshot_step(&ctx);
    }
}
