//! Small windows: go to address, compare, template editor, insert/fill, confirmations.

use egui::{Align2, Key, RichText};

use crate::app::App;
use crate::i18n;
use crate::template;
use crate::tools;

use crate::tr;

impl App {
    pub fn dialogs(&mut self, ctx: &egui::Context) {
        let lang = self.lang;
        let center = Align2::CENTER_CENTER;
        if self.goto_open {
            let mut open = true;
            let mut go = false;
            egui::Window::new(tr!(lang, "dlg.goto.title"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(center, [0.0, -120.0])
                .show(ctx, |ui| {
                    ui.label(tr!(lang, "dlg.goto.hint"));
                    let r = ui.add(egui::TextEdit::singleline(&mut self.goto_text).desired_width(260.0));
                    if self.goto_focus {
                        r.request_focus();
                        self.goto_focus = false;
                    }
                    if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        go = true;
                    }
                    if ui.button(tr!(lang, "dlg.goto.go")).clicked() {
                        go = true;
                    }
                });
            if go {
                let txt = self.goto_text.clone();
                if let Some(d) = self.doc_mut() {
                    match tools::parse_offset(&txt, d.cursor, false) {
                        Some(p) => {
                            let p = p.min(d.len());
                            d.set_cursor(p, false);
                            self.goto_open = false;
                            self.focus_hex = true;
                        }
                        None => self.status = tr!(lang, "dlg.goto.invalid").into(),
                    }
                }
            }
            if !open {
                self.goto_open = false;
            }
        }

        if self.compare.open {
            let mut open = true;
            let mut run = false;
            let mut open_more = false;
            egui::Window::new(tr!(lang, "dlg.cmp.title"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(center, [0.0, -80.0])
                .show(ctx, |ui| {
                    if self.docs.len() < 2 {
                        ui.label(tr!(lang, "dlg.cmp.need2"));
                        if ui.button(tr!(lang, "dlg.cmp.openmore")).clicked() {
                            open_more = true;
                        }
                        return;
                    }
                    let n = self.docs.len();
                    self.compare.a = self.compare.a.min(n - 1);
                    self.compare.b = self.compare.b.min(n - 1);
                    egui::Grid::new("cmpgrid").num_columns(2).show(ui, |ui| {
                        ui.label(tr!(lang, "dlg.cmp.filea"));
                        egui::ComboBox::from_id_salt("cmpa")
                            .width(260.0)
                            .selected_text(self.docs[self.compare.a].name.clone())
                            .show_ui(ui, |ui| {
                                for (i, d) in self.docs.iter().enumerate() {
                                    ui.selectable_value(&mut self.compare.a, i, &d.name);
                                }
                            });
                        ui.end_row();
                        ui.label(tr!(lang, "dlg.cmp.fileb"));
                        egui::ComboBox::from_id_salt("cmpb")
                            .width(260.0)
                            .selected_text(self.docs[self.compare.b].name.clone())
                            .show_ui(ui, |ui| {
                                for (i, d) in self.docs.iter().enumerate() {
                                    ui.selectable_value(&mut self.compare.b, i, &d.name);
                                }
                            });
                        ui.end_row();
                    });
                    ui.checkbox(&mut self.compare.split, tr!(lang, "dlg.cmp.side"));
                    ui.label(RichText::new(tr!(lang, "dlg.cmp.note")).weak());
                    if ui.button(tr!(lang, "dlg.cmp.run")).clicked() {
                        run = true;
                    }
                });
            if open_more {
                self.open_dialog();
            }
            if run {
                self.run_compare();
            }
            if !open {
                self.compare.open = false;
            }
        }

        if self.tpl_editor.open {
            self.template_editor(ctx);
        }

        if let Some((count, val)) = self.insert_dialog.clone() {
            let mut keep = true;
            let mut apply = false;
            let (mut c, mut v) = (count, val);
            egui::Window::new(tr!(lang, "dlg.insert.title"))
                .collapsible(false)
                .resizable(false)
                .anchor(center, [0.0, -80.0])
                .show(ctx, |ui| {
                    egui::Grid::new("ins").show(ui, |ui| {
                        ui.label(tr!(lang, "dlg.insert.count"));
                        ui.text_edit_singleline(&mut c);
                        ui.end_row();
                        ui.label(tr!(lang, "dlg.insert.value"));
                        ui.text_edit_singleline(&mut v);
                        ui.end_row();
                    });
                    ui.horizontal(|ui| {
                        if ui.button(tr!(lang, "dlg.insert.apply")).clicked() {
                            apply = true;
                        }
                        if ui.button(tr!(lang, "dlg.cancel")).clicked() {
                            keep = false;
                        }
                    });
                });
            if apply {
                let n = tools::parse_offset(&c, 0, false);
                match (n, tools::parse_hex(&v)) {
                    (Some(n), Some(p)) if n > 0 && n < 512 * 1024 * 1024 => {
                        let bytes: Vec<u8> = p.iter().cycle().take(n).copied().collect();
                        if let Some(d) = self.doc_mut() {
                            let pos = d.cursor;
                            d.splice(pos, 0, &bytes);
                        }
                        keep = false;
                    }
                    _ => self.status = tr!(lang, "dlg.invalid").into(),
                }
            }
            self.insert_dialog = if keep { Some((c, v)) } else { None };
        }

        if let Some(val) = self.fill_dialog.clone() {
            let mut keep = true;
            let mut apply = false;
            let mut v = val;
            egui::Window::new(tr!(lang, "dlg.fill.title"))
                .collapsible(false)
                .resizable(false)
                .anchor(center, [0.0, -80.0])
                .show(ctx, |ui| {
                    ui.label(tr!(lang, "dlg.fill.hint"));
                    ui.text_edit_singleline(&mut v);
                    ui.horizontal(|ui| {
                        if ui.button(tr!(lang, "dlg.fill.apply")).clicked() {
                            apply = true;
                        }
                        if ui.button(tr!(lang, "dlg.cancel")).clicked() {
                            keep = false;
                        }
                    });
                });
            if apply {
                if let Some(p) = tools::parse_hex(&v) {
                    if let Some(d) = self.doc_mut() {
                        if let Some((s, e)) = d.selection() {
                            let bytes: Vec<u8> = p.iter().cycle().take(e - s).copied().collect();
                            d.splice(s, e - s, &bytes);
                            d.anchor = Some(s);
                            d.cursor = e - 1;
                        }
                    }
                    keep = false;
                } else {
                    self.status = tr!(lang, "dlg.invalidpattern").into();
                }
            }
            self.fill_dialog = if keep { Some(v) } else { None };
        }

        if let Some(idx) = self.confirm_close {
            let name = self.docs.get(idx).map(|d| d.name.clone()).unwrap_or_default();
            let mut choice = None;
            egui::Window::new(tr!(lang, "dlg.close.title"))
                .collapsible(false)
                .resizable(false)
                .anchor(center, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(tr!(lang, "dlg.close.msg", name = name));
                    ui.horizontal(|ui| {
                        if ui.button(tr!(lang, "dlg.close.save")).clicked() {
                            choice = Some(0);
                        }
                        if ui.button(tr!(lang, "dlg.close.discard")).clicked() {
                            choice = Some(1);
                        }
                        if ui.button(tr!(lang, "dlg.cancel")).clicked() {
                            choice = Some(2);
                        }
                    });
                });
            match choice {
                Some(0) => {
                    if self.save(idx, false) {
                        self.close_doc(idx, true);
                    }
                    self.confirm_close = None;
                }
                Some(1) => {
                    self.close_doc(idx, true);
                    self.confirm_close = None;
                }
                Some(_) => self.confirm_close = None,
                None => {}
            }
        }

        if self.confirm_quit {
            let mut choice = None;
            let n = self.docs.iter().filter(|d| d.modified).count();
            egui::Window::new(tr!(lang, "dlg.quit.title"))
                .collapsible(false)
                .resizable(false)
                .anchor(center, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(tr!(lang, "dlg.quit.msg", n = n));
                    ui.horizontal(|ui| {
                        if ui.button(tr!(lang, "dlg.quit.yes")).clicked() {
                            choice = Some(true);
                        }
                        if ui.button(tr!(lang, "dlg.cancel")).clicked() {
                            choice = Some(false);
                        }
                    });
                });
            match choice {
                Some(true) => {
                    self.quitting = true;
                    self.confirm_quit = false;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Some(false) => self.confirm_quit = false,
                None => {}
            }
        }

        if self.about_open {
            egui::Window::new(tr!(lang, "dlg.about.title"))
                .open(&mut self.about_open)
                .collapsible(false)
                .resizable(false)
                .anchor(center, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.heading("101Craft");
                    ui.label(tr!(lang, "dlg.about.tag"));
                    ui.label(tr!(lang, "dlg.about.version", v = env!("CARGO_PKG_VERSION")));
                    ui.add_space(6.0);
                    ui.label(tr!(lang, "dlg.about.body"));
                });
        }
        if self.keys_open {
            let ctrl = tr!(lang, "key.ctrl");
            let shift = tr!(lang, "key.shift");
            let rows: [(String, String); 13] = [
                (
                    format!("{ctrl}+O / {ctrl}+S"),
                    tr!(lang, "keys.open_save", a = tr!(lang, "keys.open"), b = tr!(lang, "keys.save")),
                ),
                (
                    format!("{ctrl}+Z / {ctrl}+Y"),
                    tr!(lang, "keys.undo_redo", a = tr!(lang, "keys.undo"), b = tr!(lang, "keys.redo")),
                ),
                (
                    format!("{ctrl}+C / {ctrl}+X / {ctrl}+V"),
                    tr!(
                        lang,
                        "keys.clipboard",
                        a = tr!(lang, "keys.copy"),
                        b = tr!(lang, "keys.cut"),
                        c = tr!(lang, "keys.paste")
                    ),
                ),
                (format!("{ctrl}+A"), tr!(lang, "keys.selectall").into()),
                (
                    format!("{ctrl}+F / {ctrl}+H"),
                    tr!(lang, "keys.find_replace", a = tr!(lang, "keys.find"), b = tr!(lang, "keys.replace")),
                ),
                (
                    format!("F3 / {shift}+F3"),
                    tr!(lang, "keys.next_prev", a = tr!(lang, "keys.next"), b = tr!(lang, "keys.prev")),
                ),
                (format!("{ctrl}+G"), tr!(lang, "keys.goto").into()),
                (
                    format!("{ctrl}+B / F2"),
                    tr!(
                        lang,
                        "keys.bookmark_next",
                        a = tr!(lang, "keys.bookmark"),
                        b = tr!(lang, "keys.nextbm")
                    ),
                ),
                ("F5".into(), tr!(lang, "keys.run").into()),
                (tr!(lang, "sc.insert").into(), tr!(lang, "keys.insert").into()),
                ("Tab".into(), tr!(lang, "keys.tab").into()),
                (
                    format!("{shift}+{}", tr!(lang, "keys.arrows")),
                    tr!(lang, "keys.shiftarrows").into(),
                ),
                (format!("{ctrl}+Tab"), tr!(lang, "keys.nextfile").into()),
            ];
            egui::Window::new(tr!(lang, "dlg.keys.title"))
                .open(&mut self.keys_open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    egui::Grid::new("keys").striped(true).show(ui, |ui| {
                        for (k, v) in rows {
                            ui.label(RichText::new(k).monospace());
                            ui.label(v);
                            ui.end_row();
                        }
                    });
                });
        }
    }

    fn template_editor(&mut self, ctx: &egui::Context) {
        let lang = self.lang;
        let mut open = true;
        let mut run = false;
        let mut save = false;
        let mut load = false;
        egui::Window::new(tr!(lang, "dlg.tpled.title"))
            .open(&mut open)
            .default_size([560.0, 460.0])
            .default_pos([380.0, 110.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr!(lang, "dlg.tpled.name"));
                    ui.text_edit_singleline(&mut self.tpl_editor.name);
                    egui::ComboBox::from_id_salt("tplload")
                        .selected_text(tr!(lang, "dlg.tpled.loadexample"))
                        .show_ui(ui, |ui| {
                            for b in template::BUILTINS {
                                if ui.selectable_label(false, i18n::get(lang, b.name)).clicked() {
                                    self.tpl_editor.name = b.name.to_string();
                                    self.tpl_editor.source = b.source.to_string();
                                }
                            }
                        });
                });
                ui.horizontal(|ui| {
                    if ui.button(tr!(lang, "dlg.tpled.run")).clicked() {
                        run = true;
                    }
                    if ui.button(tr!(lang, "dlg.tpled.load")).clicked() {
                        load = true;
                    }
                    if ui.button(tr!(lang, "dlg.tpled.save")).clicked() {
                        save = true;
                    }
                });
                ui.label(RichText::new(tr!(lang, "dlg.tpled.hint")).weak().small());
                egui::ScrollArea::vertical().id_salt("tplsrc").show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.tpl_editor.source)
                            .code_editor()
                            .desired_width(f32::INFINITY)
                            .desired_rows(24),
                    );
                });
            });
        if run {
            let (n, s) = (self.tpl_editor.name.clone(), self.tpl_editor.source.clone());
            self.run_template(self.active, &n, &s);
        }
        if load {
            self.run_template_file();
        }
        if save {
            let filter = tr!(lang, "dlg.tpled.filter").to_string();
            if let Some(p) = rfd::FileDialog::new()
                .set_file_name(format!("{}.tpl", self.tpl_editor.name))
                .add_filter(filter, &["tpl", "bt"])
                .save_file()
            {
                match std::fs::write(&p, &self.tpl_editor.source) {
                    Ok(()) => self.log(tr!(lang, "dlg.tpled.saved", path = p.display())),
                    Err(e) => self.log(tr!(lang, "dlg.tpled.savefail", e = e)),
                }
            }
        }
        if !open {
            self.tpl_editor.open = false;
        }
    }
}
