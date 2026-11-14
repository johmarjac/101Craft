//! Side and bottom panels.

use std::collections::HashSet;

use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, RichText, Sense, Stroke, Ui};

use crate::app::{App, BottomTab, RightTab};
use crate::i18n;
use crate::template::{self, Node, Prim};
use crate::tools::{self, InspKind, INSPECTOR_ROWS};

use crate::tr;

fn tab_strip<T: PartialEq + Copy, S: AsRef<str>>(ui: &mut Ui, cur: &mut T, items: &[(T, S)]) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (v, label) in items {
            let sel = *cur == *v;
            let t = if sel { RichText::new(label.as_ref()).strong() } else { RichText::new(label.as_ref()) };
            if ui.selectable_label(sel, t).clicked() {
                *cur = *v;
            }
        }
    });
    ui.separator();
}

fn section(ui: &mut Ui, title: impl AsRef<str>) {
    ui.add_space(2.0);
    ui.label(RichText::new(title.as_ref()).strong().color(ui.visuals().selection.bg_fill));
}

// ---------------------------------------------------------------- left

pub fn workspace(app: &mut App, ui: &mut Ui) {
    let lang = app.lang;
    section(ui, tr!(lang, "panel.openfiles"));
    let mut activate = None;
    for (i, d) in app.docs.iter().enumerate() {
        let r = ui.selectable_label(i == app.active, d.title());
        if r.clicked() {
            activate = Some(i);
        }
        let tip = d
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| tr!(lang, "tab.unsaved").into());
        r.on_hover_text(format!("{}\n{}", tip, tools::human_size(d.len(), lang)));
    }
    if app.docs.is_empty() {
        ui.label(RichText::new("—").weak());
    }
    if let Some(i) = activate {
        app.active = i;
        app.focus_hex = true;
    }
    ui.add_space(8.0);
    section(ui, tr!(lang, "panel.templates"));
    let current = app
        .doc()
        .and_then(|d| d.template.as_ref().map(|t| t.name.clone()));
    let mut run = None;
    for b in template::BUILTINS {
        let sel = current.as_deref() == Some(b.name);
        let name = i18n::get(lang, b.name);
        if ui
            .selectable_label(sel, format!("▸ {}", name))
            .on_hover_text(tr!(lang, "panel.extensions", exts = b.exts.join(", ")))
            .clicked()
        {
            run = Some(b);
        }
    }
    if ui
        .selectable_label(false, tr!(lang, "panel.customtemplate"))
        .clicked()
    {
        app.tpl_editor.open = true;
    }
    if let Some(b) = run {
        app.run_template(app.active, b.name, b.source);
    }
    ui.add_space(8.0);
    if let Some(d) = app.doc() {
        section(ui, tr!(lang, "panel.file"));
        egui::Grid::new("fileinfo").num_columns(2).show(ui, |ui| {
            ui.label(RichText::new(tr!(lang, "panel.size")).weak());
            ui.label(format!("{}", d.len()));
            ui.end_row();
            ui.label(RichText::new(tr!(lang, "panel.bookmarks")).weak());
            ui.label(format!("{}", d.bookmarks.len()));
            ui.end_row();
            if let Some(t) = &d.template {
                ui.label(RichText::new(tr!(lang, "panel.template")).weak());
                ui.label(i18n::get(lang, &t.name));
                ui.end_row();
            }
        });
    }
}

// ---------------------------------------------------------------- right

pub fn right_panel(app: &mut App, ui: &mut Ui) {
    let lang = app.lang;
    let mut tab = app.right_tab;
    tab_strip(
        ui,
        &mut tab,
        &[
            (RightTab::Inspector, tr!(lang, "panel.inspector")),
            (RightTab::Bookmarks, tr!(lang, "panel.bookmarks")),
            (RightTab::Checksum, tr!(lang, "panel.checksum")),
        ],
    );
    app.right_tab = tab;
    match tab {
        RightTab::Inspector => inspector(app, ui),
        RightTab::Bookmarks => bookmarks(app, ui),
        RightTab::Checksum => checksum(app, ui),
    }
}

fn inspector(app: &mut App, ui: &mut Ui) {
    let lang = app.lang;
    let le = app.little_endian;
    let Some(d) = app.docs.get(app.active) else {
        ui.label(tr!(lang, "panel.nofile"));
        return;
    };
    let pos = d.cursor;
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr!(lang, "panel.from", pos = format!("{pos:X}"))).monospace());
        ui.separator();
        if ui.selectable_label(app.little_endian, "Little").clicked() {
            app.little_endian = true;
        }
        if ui.selectable_label(!app.little_endian, "Big").clicked() {
            app.little_endian = false;
        }
    });
    let values: Vec<(usize, InspKind, &str, String)> = INSPECTOR_ROWS
        .iter()
        .enumerate()
        .map(|(i, (k, key))| (i, *k, *key, tools::inspect(&d.data, pos, *k, le)))
        .collect();
    let mut commit: Option<(InspKind, String)> = None;
    let mut start_edit = None;
    let mut cancel = false;
    egui::ScrollArea::vertical().id_salt("insp").show(ui, |ui| {
        egui::Grid::new("inspgrid")
            .num_columns(2)
            .striped(true)
            .min_col_width(110.0)
            .show(ui, |ui| {
                for (i, k, key, val) in &values {
                    ui.label(RichText::new(tr!(lang, key)).weak());
                    match &mut app.insp_edit {
                        Some((row, text)) if *row == *i => {
                            let r = ui.add(egui::TextEdit::singleline(text).desired_width(150.0).font(FontId::monospace(12.5)));
                            r.request_focus();
                            if ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                                commit = Some((*k, text.clone()));
                            } else if ui.input(|inp| inp.key_pressed(egui::Key::Escape)) || r.lost_focus() {
                                cancel = true;
                            }
                        }
                        _ => {
                            let r = ui.add(egui::Label::new(RichText::new(val).monospace()).sense(Sense::click()));
                            let r = if k.editable() {
                                r.on_hover_text(tr!(lang, "panel.inspector.edit"))
                            } else {
                                r
                            };
                            if r.double_clicked() && k.editable() && pos < d.len() {
                                start_edit = Some((*i, val.trim_matches('\\').to_string()));
                            }
                        }
                    }
                    ui.end_row();
                }
            });
        ui.add_space(6.0);
        ui.label(RichText::new(tr!(lang, "panel.inspector.hint")).weak().small());
    });
    if let Some(s) = start_edit {
        app.insp_edit = Some(s);
    }
    if cancel && commit.is_none() {
        app.insp_edit = None;
    }
    if let Some((k, text)) = commit {
        app.insp_edit = None;
        match tools::encode(&text, k, le) {
            Some(bytes) => {
                let insert = app.insert;
                if let Some(d) = app.doc_mut() {
                    let pos = d.cursor;
                    let n = bytes.len().min(d.len().saturating_sub(pos));
                    if n == bytes.len() || insert {
                        d.splice(pos, n, &bytes);
                        d.cursor = pos;
                    }
                }
            }
            None => app.status = tr!(lang, "panel.err.type", text = text),
        }
    }
}

fn bookmarks(app: &mut App, ui: &mut Ui) {
    let lang = app.lang;
    let pal_dark = app.dark;
    let Some(d) = app.docs.get_mut(app.active) else {
        ui.label(tr!(lang, "panel.nofile"));
        return;
    };
    if ui.button(tr!(lang, "panel.bm.add")).clicked() {
        let n = app.active;
        d.toggle_bookmark(n, lang);
    }
    ui.add_space(4.0);
    if d.bookmarks.is_empty() {
        let ctrl = tr!(lang, "key.ctrl");
        ui.label(RichText::new(tr!(lang, "panel.bm.empty", ctrl = ctrl)).weak());
        return;
    }
    let mut remove = None;
    let mut jump = None;
    egui::ScrollArea::vertical().id_salt("bm").show(ui, |ui| {
        egui::Grid::new("bmgrid").num_columns(4).striped(true).show(ui, |ui| {
            ui.label(RichText::new(tr!(lang, "panel.bm.name")).weak());
            ui.label(RichText::new(tr!(lang, "panel.bm.address")).weak());
            ui.label(RichText::new(tr!(lang, "panel.bm.length")).weak());
            ui.label("");
            ui.end_row();
            for (i, b) in d.bookmarks.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                    let c = crate::hexview::Palette::new(pal_dark).tint(b.color);
                    ui.painter().rect_filled(r, CornerRadius::same(2), c);
                    ui.add_sized([120.0, 18.0], egui::TextEdit::singleline(&mut b.name));
                });
                if ui.link(RichText::new(format!("0x{:X}", b.start)).monospace()).clicked() {
                    jump = Some((b.start, b.len));
                }
                ui.label(format!("{}", b.len));
                if ui
                    .small_button("✕")
                    .on_hover_text(tr!(lang, "panel.bm.remove"))
                    .clicked()
                {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });
    });
    if let Some(i) = remove {
        d.bookmarks.remove(i);
    }
    if let Some((s, l)) = jump {
        d.select_range(s, l);
    }
}

fn checksum(app: &mut App, ui: &mut Ui) {
    let lang = app.lang;
    let has_sel = app.doc().map(|d| d.selection().is_some()).unwrap_or(false);
    ui.horizontal(|ui| {
        ui.radio_value(&mut app.checksum_selection_only, false, tr!(lang, "panel.cs.whole"));
        ui.add_enabled_ui(has_sel, |ui| {
            ui.radio_value(&mut app.checksum_selection_only, true, tr!(lang, "panel.cs.selection"));
        });
    });
    if ui.button(tr!(lang, "panel.cs.compute")).clicked() {
        if let Some(d) = app.doc() {
            let data: &[u8] = if app.checksum_selection_only && has_sel {
                d.selected_bytes()
            } else {
                &d.data
            };
            let n = data.len();
            app.checksum_results = tools::CHECKSUMS
                .iter()
                .map(|(id, key)| (tr!(lang, key).to_string(), tools::checksum(id, data)))
                .collect();
            app.log(tr!(lang, "panel.cs.done", size = tools::human_size(n, lang)));
        }
    }
    ui.add_space(4.0);
    let mut copy = None;
    egui::ScrollArea::vertical().id_salt("cs").show(ui, |ui| {
        for (name, val) in &app.checksum_results {
            ui.horizontal(|ui| {
                ui.label(RichText::new(name.as_str()).weak());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button("📋")
                        .on_hover_text(tr!(lang, "panel.cs.copy"))
                        .clicked()
                    {
                        copy = Some(val.clone());
                    }
                });
            });
            ui.add(egui::Label::new(RichText::new(val).monospace()).wrap());
            ui.add_space(2.0);
        }
    });
    if let Some(c) = copy {
        ui.ctx().copy_text(c);
        app.status = tr!(lang, "panel.cs.copied").into();
    }
}

// ---------------------------------------------------------------- bottom

pub fn bottom_panel(app: &mut App, ui: &mut Ui) {
    let lang = app.lang;
    let mut tab = app.bottom_tab;
    let hits = app.doc().map(|d| d.hits.len()).unwrap_or(0);
    let search_label = if hits > 0 {
        tr!(lang, "panel.bottom.search.n", n = hits)
    } else {
        tr!(lang, "panel.bottom.search").into()
    };
    let cmp_label = if app.compare.pair.is_some() {
        tr!(lang, "panel.bottom.compare.n", n = app.compare.result.len())
    } else {
        tr!(lang, "panel.bottom.compare").into()
    };
    tab_strip(
        ui,
        &mut tab,
        &[
            (BottomTab::Template, tr!(lang, "panel.bottom.template")),
            (BottomTab::Search, search_label),
            (BottomTab::Compare, cmp_label),
            (BottomTab::Histogram, tr!(lang, "panel.bottom.histogram")),
            (BottomTab::Output, tr!(lang, "panel.bottom.output")),
        ],
    );
    app.bottom_tab = tab;
    match tab {
        BottomTab::Template => template_results(app, ui),
        BottomTab::Search => search_results(app, ui),
        BottomTab::Compare => compare_results(app, ui),
        BottomTab::Histogram => histogram(app, ui),
        BottomTab::Output => output(app, ui),
    }
}

struct Row<'a> {
    depth: usize,
    node: &'a Node,
    id: String,
}

fn flatten<'a>(nodes: &'a [Node], prefix: &str, depth: usize, expanded: &HashSet<String>, out: &mut Vec<Row<'a>>) {
    for n in nodes {
        let id = format!("{prefix}/{}", n.name);
        let open = expanded.contains(&id);
        out.push(Row { depth, node: n, id: id.clone() });
        if open && !n.children.is_empty() {
            flatten(&n.children, &id, depth + 1, expanded, out);
        }
    }
}

fn insp_kind(p: Prim) -> InspKind {
    match p {
        Prim::I8 | Prim::Char => InspKind::I8,
        Prim::U8 => InspKind::U8,
        Prim::I16 => InspKind::I16,
        Prim::U16 => InspKind::U16,
        Prim::I32 => InspKind::I32,
        Prim::U32 => InspKind::U32,
        Prim::I64 => InspKind::I64,
        Prim::U64 => InspKind::U64,
        Prim::F32 => InspKind::F32,
        Prim::F64 => InspKind::F64,
    }
}

fn template_results(app: &mut App, ui: &mut Ui) {
    let lang = app.lang;
    let dark = app.dark;
    let Some(d) = app.docs.get(app.active) else {
        ui.label(tr!(lang, "panel.nofile"));
        return;
    };
    let Some(t) = &d.template else {
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr!(lang, "panel.tpl.empty")).weak());
        });
        let mut run = None;
        ui.horizontal_wrapped(|ui| {
            for b in template::BUILTINS {
                if ui.button(i18n::get(lang, b.name)).clicked() {
                    run = Some(b);
                }
            }
        });
        if let Some(b) = run {
            app.run_template(app.active, b.name, b.source);
        }
        return;
    };
    let cursor_path = t.path_at(d.cursor);
    let deepest = cursor_path.last().cloned();
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr!(lang, "panel.tpl.name", name = i18n::get(lang, &t.name))).strong());
        if let Some(e) = &t.error {
            ui.label(RichText::new(format!("⚠ {e}")).color(Color32::from_rgb(210, 90, 40)));
        } else {
            ui.label(RichText::new(tr!(lang, "panel.tpl.ok")).color(Color32::from_rgb(40, 150, 70)));
        }
    });
    let mut rows = Vec::new();
    flatten(&t.nodes, "", 0, &t.expanded, &mut rows);

    let font = FontId::proportional(13.0);
    let mono = FontId::monospace(12.5);
    let rh = 20.0;
    let scale = (ui.available_width() / 1000.0).clamp(0.75, 1.6);
    let cols = [0.0f32, 270.0, 520.0, 610.0, 680.0, 860.0].map(|c| c * scale);
    let titles = [
        tr!(lang, "panel.tpl.col.name"),
        tr!(lang, "panel.tpl.col.value"),
        tr!(lang, "panel.tpl.col.start"),
        tr!(lang, "panel.tpl.col.size"),
        tr!(lang, "panel.tpl.col.type"),
        tr!(lang, "panel.tpl.col.comment"),
    ];
    let (hdr, _) = ui.allocate_exact_size(vec2(ui.available_width(), rh), Sense::hover());
    let vis = ui.visuals().clone();
    ui.painter().rect_filled(hdr, CornerRadius::ZERO, vis.faint_bg_color);
    for (x, title) in cols.iter().zip(titles) {
        ui.painter().text(pos2(hdr.min.x + x + 6.0, hdr.center().y), Align2::LEFT_CENTER, title, font.clone(), vis.weak_text_color());
    }

    let mut toggle: Option<String> = None;
    let mut select: Option<(usize, usize)> = None;
    let mut start_edit: Option<(String, String)> = None;
    let mut commit: Option<(String, String)> = None;
    let mut cancel = false;
    let editing = app.tpl_edit.clone();
    let mut edit_text = editing.as_ref().map(|e| e.1.clone()).unwrap_or_default();
    let mut edit_target: Option<(usize, Prim, bool)> = None;

    egui::ScrollArea::vertical()
        .id_salt("tplrows")
        .auto_shrink([false, false])
        .show_rows(ui, rh, rows.len(), |ui, range| {
            for i in range {
                let row = &rows[i];
                let n = row.node;
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), rh), Sense::click());
                let p = ui.painter().clone();
                let is_deep = deepest.as_deref() == Some(row.id.as_str());
                let on_path = cursor_path.contains(&row.id);
                if is_deep {
                    p.rect_filled(rect, CornerRadius::ZERO, vis.selection.bg_fill.gamma_multiply(0.35));
                } else if on_path {
                    p.rect_filled(rect, CornerRadius::ZERO, vis.selection.bg_fill.gamma_multiply(0.12));
                } else if resp.hovered() {
                    p.rect_filled(rect, CornerRadius::ZERO, vis.faint_bg_color);
                } else if i % 2 == 1 {
                    p.rect_filled(rect, CornerRadius::ZERO, vis.faint_bg_color.gamma_multiply(0.5));
                }
                let x0 = rect.min.x + 6.0 + row.depth as f32 * 16.0;
                let cy = rect.center().y;
                let has_kids = !n.children.is_empty();
                let open = t.expanded.contains(&row.id);
                if has_kids {
                    p.text(pos2(x0, cy), Align2::LEFT_CENTER, if open { "▼" } else { "▶" }, FontId::proportional(10.0), vis.weak_text_color());
                }
                // colour swatch like the hex view
                if let Some(c) = d.highlight_at(n.offset).filter(|_| n.size > 0 && (n.children.is_empty() || n.elem.is_some())) {
                    let sw = Rect::from_center_size(pos2(x0 + 18.0, cy), vec2(8.0, 8.0));
                    p.rect_filled(sw, CornerRadius::same(2), crate::hexview::Palette::new(dark).tint(c));
                }
                let text_col = vis.text_color();
                p.text(pos2(x0 + 26.0, cy), Align2::LEFT_CENTER, &n.name, font.clone(), if n.is_struct { vis.strong_text_color() } else { text_col });
                let is_editing = editing.as_ref().map(|e| e.0 == row.id).unwrap_or(false);
                let vrect = Rect::from_min_max(pos2(rect.min.x + cols[1] + 2.0, rect.min.y + 1.0), pos2(rect.min.x + cols[2] - 6.0, rect.max.y - 1.0));
                if is_editing {
                    if let Some(pr) = n.prim {
                        edit_target = Some((n.offset, pr, n.le));
                    }
                    let r = ui.put(vrect, egui::TextEdit::singleline(&mut edit_text).font(mono.clone()));
                    r.request_focus();
                    if ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                        commit = Some((row.id.clone(), edit_text.clone()));
                    } else if ui.input(|inp| inp.key_pressed(egui::Key::Escape)) || r.lost_focus() {
                        cancel = true;
                    }
                } else {
                    let shown: String = n.display.chars().take(60).collect();
                    p.text(pos2(rect.min.x + cols[1] + 6.0, cy), Align2::LEFT_CENTER, shown, mono.clone(), text_col);
                }
                p.text(pos2(rect.min.x + cols[2] + 6.0, cy), Align2::LEFT_CENTER, format!("{:X}h", n.offset), mono.clone(), vis.weak_text_color());
                p.text(pos2(rect.min.x + cols[3] + 6.0, cy), Align2::LEFT_CENTER, format!("{}", n.size), mono.clone(), vis.weak_text_color());
                p.text(pos2(rect.min.x + cols[4] + 6.0, cy), Align2::LEFT_CENTER, &n.type_name, font.clone(), vis.weak_text_color());
                if let Some(c) = &n.comment {
                    p.text(pos2(rect.min.x + cols[5] + 6.0, cy), Align2::LEFT_CENTER, c, font.clone(), vis.weak_text_color());
                }
                p.hline(rect.x_range(), rect.max.y - 0.5, Stroke::new(0.5, vis.widgets.noninteractive.bg_stroke.color.gamma_multiply(0.5)));

                if resp.clicked() {
                    let click_x = resp.interact_pointer_pos().map(|p| p.x).unwrap_or(0.0);
                    if has_kids && click_x < x0 + 14.0 {
                        toggle = Some(row.id.clone());
                    } else {
                        select = Some((n.offset, n.size));
                    }
                }
                if resp.double_clicked() {
                    let click_x = resp.interact_pointer_pos().map(|p| p.x).unwrap_or(0.0);
                    if n.prim.is_some() && click_x >= rect.min.x + cols[1] && click_x < rect.min.x + cols[2] {
                        let v = match &n.value {
                            Some(template::Val::Int(i)) => i.to_string(),
                            Some(template::Val::Float(f)) => f.to_string(),
                            _ => n.display.clone(),
                        };
                        start_edit = Some((row.id.clone(), v));
                    } else if has_kids {
                        toggle = Some(row.id.clone());
                    }
                }
                let edit_hint = if n.prim.is_some() {
                    format!("\n{}", tr!(lang, "panel.tpl.edit"))
                } else {
                    String::new()
                };
                resp.on_hover_text(format!(
                    "{} {}\nStart 0x{:X}, {} bytes{}",
                    n.type_name, n.name, n.offset, n.size, edit_hint
                ));
            }
        });

    // apply actions
    if let Some(id) = toggle {
        if let Some(t) = app.docs[app.active].template.as_mut() {
            if !t.expanded.remove(&id) {
                t.expanded.insert(id);
            }
        }
    }
    if let Some((s, l)) = select {
        app.docs[app.active].select_range(s, l);
    }
    if let Some(e) = start_edit {
        app.tpl_edit = Some(e);
    } else if let Some((id, text)) = commit {
        app.tpl_edit = None;
        if let Some((off, prim, le)) = edit_target {
            let k = insp_kind(prim);
            let enc = tools::encode(&text, k, le).or_else(|| {
                // allow unsigned input for signed chars etc.
                let alt = match k {
                    InspKind::I8 => InspKind::U8,
                    InspKind::I16 => InspKind::U16,
                    InspKind::I32 => InspKind::U32,
                    InspKind::I64 => InspKind::U64,
                    o => o,
                };
                tools::encode(&text, alt, le)
            });
            match enc {
                Some(bytes) => {
                    let d = &mut app.docs[app.active];
                    d.splice(off, bytes.len(), &bytes);
                    let path = id.trim_start_matches('/').replace('/', ".");
                    app.log(tr!(lang, "panel.tpl.set", path = path, value = text));
                }
                None => app.status = tr!(lang, "panel.err.type", text = text),
            }
        }
    } else if cancel {
        app.tpl_edit = None;
    } else if let Some((id, _)) = editing {
        app.tpl_edit = Some((id, edit_text));
    }
}

fn search_results(app: &mut App, ui: &mut Ui) {
    let lang = app.lang;
    let Some(d) = app.docs.get(app.active) else {
        return;
    };
    if d.hits.is_empty() {
        let ctrl = tr!(lang, "key.ctrl");
        ui.label(RichText::new(tr!(lang, "panel.search.empty", ctrl = ctrl)).weak());
        return;
    }
    let mut select = None;
    let rh = 18.0;
    let vis = ui.visuals().clone();
    egui::ScrollArea::vertical()
        .id_salt("hits")
        .auto_shrink([false, false])
        .show_rows(ui, rh, d.hits.len(), |ui, range| {
            for i in range {
                let (p, l) = d.hits[i];
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), rh), Sense::click());
                let painter = ui.painter();
                let sel = d.selection().map(|(s, _)| s == p).unwrap_or(false);
                if sel {
                    painter.rect_filled(rect, CornerRadius::ZERO, vis.selection.bg_fill.gamma_multiply(0.3));
                } else if resp.hovered() {
                    painter.rect_filled(rect, CornerRadius::ZERO, vis.faint_bg_color);
                }
                let mono = FontId::monospace(12.5);
                let cy = rect.center().y;
                painter.text(pos2(rect.min.x + 6.0, cy), Align2::LEFT_CENTER, format!("{}", i + 1), mono.clone(), vis.weak_text_color());
                painter.text(pos2(rect.min.x + 70.0, cy), Align2::LEFT_CENTER, format!("0x{p:08X}"), mono.clone(), vis.text_color());
                let len_txt = tr!(lang, "panel.search.bytes", n = l);
                painter.text(pos2(rect.min.x + 180.0, cy), Align2::LEFT_CENTER, len_txt, mono.clone(), vis.weak_text_color());
                let ctx_start = p.saturating_sub(4);
                let ctx_end = (p + l + 8).min(d.len());
                let preview: String = d.data[ctx_start..ctx_end].iter().map(|b| if (0x20..0x7f).contains(b) { *b as char } else { '.' }).collect();
                painter.text(pos2(rect.min.x + 270.0, cy), Align2::LEFT_CENTER, tools::hex_spaced(&d.data[p..(p + l).min(d.len())].iter().take(16).copied().collect::<Vec<_>>()), mono.clone(), vis.text_color());
                painter.text(pos2(rect.min.x + 680.0, cy), Align2::LEFT_CENTER, preview, mono, vis.weak_text_color());
                if resp.clicked() {
                    select = Some((p, l));
                }
            }
        });
    if let Some((p, l)) = select {
        app.docs[app.active].select_range(p, l);
    }
}

fn compare_results(app: &mut App, ui: &mut Ui) {
    let lang = app.lang;
    let Some((a, b)) = app.compare.pair else {
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr!(lang, "panel.cmp.empty")).weak());
            if ui.button(tr!(lang, "panel.cmp.open")).clicked() {
                app.open_compare_dialog();
            }
        });
        return;
    };
    if a >= app.docs.len() || b >= app.docs.len() {
        app.end_compare();
        return;
    }
    let mut end = false;
    ui.horizontal(|ui| {
        let n = app.compare.result.len();
        if n == 0 {
            let msg = tr!(
                lang,
                "panel.cmp.identical",
                a = app.docs[a].name,
                b = app.docs[b].name
            );
            ui.label(RichText::new(msg).color(Color32::from_rgb(40, 150, 70)));
        } else {
            let msg = tr!(
                lang,
                "panel.cmp.count",
                n = n,
                a = app.docs[a].name,
                b = app.docs[b].name
            );
            ui.label(msg);
        }
        ui.checkbox(&mut app.compare.split, tr!(lang, "panel.cmp.side"));
        if ui.button(tr!(lang, "panel.cmp.end")).clicked() {
            end = true;
        }
    });
    if end {
        app.end_compare();
        return;
    }
    let mut select = None;
    let rh = 18.0;
    let vis = ui.visuals().clone();
    let res = &app.compare.result;
    egui::ScrollArea::vertical()
        .id_salt("cmprows")
        .auto_shrink([false, false])
        .show_rows(ui, rh, res.len(), |ui, range| {
            for i in range {
                let r = &res[i];
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), rh), Sense::click());
                let painter = ui.painter();
                if resp.hovered() {
                    painter.rect_filled(rect, CornerRadius::ZERO, vis.faint_bg_color);
                }
                let mono = FontId::monospace(12.5);
                let cy = rect.center().y;
                let da = &app.docs[a].data;
                let db = &app.docs[b].data;
                let ba: Vec<u8> = da.get(r.start..r.start + r.len_a).unwrap_or(&[]).iter().take(12).copied().collect();
                let bb: Vec<u8> = db.get(r.start..r.start + r.len_b).unwrap_or(&[]).iter().take(12).copied().collect();
                painter.text(pos2(rect.min.x + 6.0, cy), Align2::LEFT_CENTER, format!("0x{:08X}", r.start), mono.clone(), vis.text_color());
                let kind = if r.len_a == r.len_b {
                    tr!(lang, "panel.cmp.changed", n = r.len_a)
                } else if r.len_a == 0 {
                    tr!(lang, "panel.cmp.onlyb", n = r.len_b)
                } else {
                    tr!(lang, "panel.cmp.onlya", n = r.len_a)
                };
                painter.text(pos2(rect.min.x + 120.0, cy), Align2::LEFT_CENTER, kind, mono.clone(), vis.weak_text_color());
                painter.text(pos2(rect.min.x + 260.0, cy), Align2::LEFT_CENTER, format!("A: {}", tools::hex_spaced(&ba)), mono.clone(), Color32::from_rgb(200, 60, 60));
                painter.text(pos2(rect.min.x + 640.0, cy), Align2::LEFT_CENTER, format!("B: {}", tools::hex_spaced(&bb)), mono, Color32::from_rgb(40, 130, 60));
                if resp.clicked() {
                    select = Some((r.start, r.len_a.max(1), r.len_b.max(1)));
                }
            }
        });
    if let Some((s, la, lb)) = select {
        app.docs[a].select_range(s, la);
        app.docs[b].select_range(s, lb);
    }
}

fn histogram(app: &mut App, ui: &mut Ui) {
    let lang = app.lang;
    let idx = app.active;
    let Some(d) = app.docs.get(idx) else { return };
    let stale = match &app.histogram {
        Some((i, rev, _)) => *i != idx || *rev != d.revision,
        None => true,
    };
    if stale {
        let mut counts = Box::new([0u64; 256]);
        for &b in &d.data {
            counts[b as usize] += 1;
        }
        app.histogram = Some((idx, d.revision, counts));
    }
    let counts = &app.histogram.as_ref().unwrap().2;
    let total: u64 = counts.iter().sum();
    let entropy: f64 = if total == 0 {
        0.0
    } else {
        counts
            .iter()
            .filter(|c| **c > 0)
            .map(|&c| {
                let p = c as f64 / total as f64;
                -p * p.log2()
            })
            .sum()
    };
    let distinct = counts.iter().filter(|c| **c > 0).count();
    ui.horizontal(|ui| {
        ui.label(tr!(lang, "panel.hist.count", n = total, d = distinct));
        ui.separator();
        ui.label(tr!(lang, "panel.hist.entropy", e = format!("{entropy:.3}")))
            .on_hover_text(tr!(lang, "panel.hist.hint"));
    });
    let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    let vis = ui.visuals().clone();
    let p = ui.painter_at(rect);
    p.rect_filled(rect, CornerRadius::ZERO, vis.extreme_bg_color);
    let max = *counts.iter().max().unwrap_or(&1) as f32;
    let inner = rect.shrink2(vec2(30.0, 18.0));
    let bw = inner.width() / 256.0;
    let hover = resp.hover_pos().map(|hp| (((hp.x - inner.min.x) / bw).floor() as i64).clamp(0, 255) as usize);
    for (i, &c) in counts.iter().enumerate() {
        if c == 0 {
            continue;
        }
        let h = (c as f32 / max.max(1.0)) * inner.height();
        let r = Rect::from_min_max(pos2(inner.min.x + i as f32 * bw, inner.max.y - h.max(1.0)), pos2(inner.min.x + (i + 1) as f32 * bw - 0.5, inner.max.y));
        let col = if Some(i) == hover {
            Color32::from_rgb(230, 110, 0)
        } else if (0x20..0x7f).contains(&i) {
            Color32::from_rgb(70, 140, 220)
        } else {
            Color32::from_rgb(130, 150, 180)
        };
        p.rect_filled(r, CornerRadius::ZERO, col);
    }
    for mark in [0usize, 0x20, 0x40, 0x60, 0x80, 0xA0, 0xC0, 0xE0, 0xFF] {
        let x = inner.min.x + mark as f32 * bw;
        p.text(pos2(x, inner.max.y + 2.0), Align2::LEFT_TOP, format!("{mark:02X}"), FontId::monospace(10.0), vis.weak_text_color());
    }
    p.hline(inner.x_range(), inner.max.y, Stroke::new(1.0, vis.weak_text_color()));
    if let Some(i) = hover {
        if resp.hovered() {
            let c = counts[i];
            let ch = if (0x20..0x7f).contains(&i) { format!(" '{}'", i as u8 as char) } else { String::new() };
            let pct = if total > 0 { c as f64 * 100.0 / total as f64 } else { 0.0 };
            let txt = tr!(
                lang,
                "panel.hist.byte",
                b = format!("{i:02X}"),
                ch = ch,
                c = c,
                p = format!("{pct:.2}")
            );
            p.text(pos2(inner.min.x + 4.0, rect.min.y + 2.0), Align2::LEFT_TOP, txt, FontId::monospace(12.0), vis.strong_text_color());
        }
    }
}

fn output(app: &mut App, ui: &mut Ui) {
    let lang = app.lang;
    ui.horizontal(|ui| {
        if ui.small_button(tr!(lang, "panel.out.clear")).clicked() {
            app.output.clear();
        }
    });
    egui::ScrollArea::vertical()
        .id_salt("out")
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for l in &app.output {
                ui.label(RichText::new(l).monospace().size(12.0));
            }
        });
}
