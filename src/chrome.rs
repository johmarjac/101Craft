//! Menu bar, toolbar, document tabs, find bar, status bar and the central view.

use egui::{Align, Color32, Key, Layout, RichText, Theme, Ui};

use crate::app::{App, BottomTab, RightTab};
use crate::doc::Column;
use crate::hexview::{self, Palette, ViewOptions};
use crate::i18n::Lang;
use crate::template;
use crate::tools::{self, SearchKind};

use crate::tr;

impl App {
    pub fn menu_bar(&mut self, ui: &mut Ui) {
        let lang = self.lang;
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(tr!(lang, "menu.file"), |ui| {
                if ui
                    .add(egui::Button::new(tr!(lang, "menu.new")).shortcut_text(tr!(lang, "sc.new")))
                    .clicked()
                {
                    self.new_doc();
                    ui.close();
                }
                if ui
                    .add(egui::Button::new(tr!(lang, "menu.open")).shortcut_text(tr!(lang, "sc.open")))
                    .clicked()
                {
                    ui.close();
                    self.open_dialog();
                }
                ui.separator();
                let has = !self.docs.is_empty();
                if ui
                    .add_enabled(has, egui::Button::new(tr!(lang, "menu.save")).shortcut_text(tr!(lang, "sc.save")))
                    .clicked()
                {
                    ui.close();
                    self.save(self.active, false);
                }
                if ui
                    .add_enabled(
                        has,
                        egui::Button::new(tr!(lang, "menu.saveas")).shortcut_text(tr!(lang, "sc.saveas")),
                    )
                    .clicked()
                {
                    ui.close();
                    self.save(self.active, true);
                }
                if ui
                    .add_enabled(has, egui::Button::new(tr!(lang, "menu.close")).shortcut_text(tr!(lang, "sc.close")))
                    .clicked()
                {
                    ui.close();
                    self.close_doc(self.active, false);
                }
                ui.separator();
                if ui.button(tr!(lang, "menu.quit")).clicked() {
                    ui.close();
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button(tr!(lang, "menu.edit"), |ui| {
                let (cu, cr) = self
                    .doc()
                    .map(|d| (d.can_undo(), d.can_redo()))
                    .unwrap_or((false, false));
                if ui
                    .add_enabled(cu, egui::Button::new(tr!(lang, "menu.undo")).shortcut_text(tr!(lang, "sc.undo")))
                    .clicked()
                {
                    self.undo();
                    ui.close();
                }
                if ui
                    .add_enabled(cr, egui::Button::new(tr!(lang, "menu.redo")).shortcut_text(tr!(lang, "sc.redo")))
                    .clicked()
                {
                    self.redo();
                    ui.close();
                }
                ui.separator();
                if ui
                    .add(egui::Button::new(tr!(lang, "menu.copyhex")).shortcut_text(tr!(lang, "sc.copy")))
                    .clicked()
                {
                    if let Some(d) = self.doc() {
                        let b = if d.selected_bytes().is_empty() {
                            d.data.get(d.cursor..d.cursor + 1).unwrap_or(&[])
                        } else {
                            d.selected_bytes()
                        };
                        ui.ctx().copy_text(tools::hex_spaced(b));
                    }
                    ui.close();
                }
                if ui.button(tr!(lang, "menu.copytext")).clicked() {
                    if let Some(d) = self.doc() {
                        ui.ctx()
                            .copy_text(d.selected_bytes().iter().map(|x| *x as char).collect());
                    }
                    ui.close();
                }
                if ui.button(tr!(lang, "menu.copyc")).clicked() {
                    if let Some(d) = self.doc() {
                        let b = d.selected_bytes();
                        let body = b
                            .chunks(12)
                            .map(|c| c.iter().map(|x| format!("0x{x:02X}")).collect::<Vec<_>>().join(", "))
                            .collect::<Vec<_>>()
                            .join(",\n    ");
                        ui.ctx()
                            .copy_text(format!("unsigned char data[{}] = {{\n    {body}\n}};", b.len()));
                    }
                    ui.close();
                }
                ui.label(RichText::new(tr!(lang, "menu.pastehint")).weak().small());
                ui.separator();
                if ui
                    .add(egui::Button::new(tr!(lang, "menu.selectall")).shortcut_text(tr!(lang, "sc.selectall")))
                    .clicked()
                {
                    if let Some(d) = self.doc_mut() {
                        let n = d.len();
                        if n > 0 {
                            d.anchor = Some(0);
                            d.cursor = n - 1;
                        }
                    }
                    ui.close();
                }
                if ui
                    .add(egui::Button::new(tr!(lang, "menu.deleteselection")).shortcut_text(tr!(lang, "sc.delete")))
                    .clicked()
                {
                    if let Some(d) = self.doc_mut() {
                        d.delete_forward();
                    }
                    ui.close();
                }
                if ui.button(tr!(lang, "menu.insertbytes")).clicked() {
                    self.insert_dialog = Some(("16".into(), "00".into()));
                    ui.close();
                }
                if ui.button(tr!(lang, "menu.fillselection")).clicked() {
                    self.fill_dialog = Some("00".into());
                    ui.close();
                }
                ui.separator();
                let label = if self.insert {
                    tr!(lang, "menu.mode.overwrite")
                } else {
                    tr!(lang, "menu.mode.insert")
                };
                if ui
                    .add(egui::Button::new(label).shortcut_text(tr!(lang, "sc.insert")))
                    .clicked()
                {
                    self.insert = !self.insert;
                    ui.close();
                }
            });
            ui.menu_button(tr!(lang, "menu.search"), |ui| {
                if ui
                    .add(egui::Button::new(tr!(lang, "menu.find")).shortcut_text(tr!(lang, "sc.find")))
                    .clicked()
                {
                    self.open_find(false);
                    ui.close();
                }
                if ui
                    .add(egui::Button::new(tr!(lang, "menu.replace")).shortcut_text(tr!(lang, "sc.replace")))
                    .clicked()
                {
                    self.open_find(true);
                    ui.close();
                }
                if ui
                    .add(egui::Button::new(tr!(lang, "menu.findnext")).shortcut_text(tr!(lang, "sc.findnext")))
                    .clicked()
                {
                    self.find_step(true);
                    ui.close();
                }
                if ui
                    .add(egui::Button::new(tr!(lang, "menu.findprev")).shortcut_text(tr!(lang, "sc.findprev")))
                    .clicked()
                {
                    self.find_step(false);
                    ui.close();
                }
                ui.separator();
                if ui
                    .add(egui::Button::new(tr!(lang, "menu.goto")).shortcut_text(tr!(lang, "sc.goto")))
                    .clicked()
                {
                    self.open_goto();
                    ui.close();
                }
                ui.separator();
                if ui
                    .add(egui::Button::new(tr!(lang, "menu.bookmark.toggle")).shortcut_text(tr!(lang, "sc.bookmark")))
                    .clicked()
                {
                    self.toggle_bookmark();
                    ui.close();
                }
                if ui
                    .add(egui::Button::new(tr!(lang, "menu.bookmark.next")).shortcut_text(tr!(lang, "sc.nextbm")))
                    .clicked()
                {
                    self.next_bookmark(true);
                    ui.close();
                }
                if ui
                    .add(egui::Button::new(tr!(lang, "menu.bookmark.prev")).shortcut_text(tr!(lang, "sc.prevbm")))
                    .clicked()
                {
                    self.next_bookmark(false);
                    ui.close();
                }
            });
            ui.menu_button(tr!(lang, "menu.view"), |ui| {
                if let Some(d) = self.doc_mut() {
                    let mut t = d.text_mode;
                    if ui.checkbox(&mut t, tr!(lang, "menu.textmode")).clicked() {
                        d.text_mode = t;
                        d.text_dirty_from_data = true;
                        ui.close();
                    }
                }
                ui.separator();
                ui.label(tr!(lang, "menu.bytesperrow"));
                for n in [8usize, 16, 24, 32] {
                    ui.radio_value(&mut self.bytes_per_row, n, format!("{n}"));
                }
                ui.separator();
                ui.label(tr!(lang, "menu.byteorder"));
                ui.radio_value(&mut self.little_endian, true, tr!(lang, "menu.le"));
                ui.radio_value(&mut self.little_endian, false, tr!(lang, "menu.be"));
                ui.separator();
                ui.label(tr!(lang, "menu.fontsize"));
                ui.add(egui::Slider::new(&mut self.font_size, 10.0..=20.0).step_by(1.0));
                ui.separator();
                if ui.checkbox(&mut self.dark, tr!(lang, "menu.dark")).clicked() {
                    ui.ctx().set_theme(if self.dark { Theme::Dark } else { Theme::Light });
                }
            });
            ui.menu_button(tr!(lang, "menu.templates"), |ui| {
                ui.checkbox(&mut self.auto_template, tr!(lang, "menu.autoapply"));
                if ui
                    .add(egui::Button::new(tr!(lang, "menu.rerun")).shortcut_text("F5"))
                    .clicked()
                {
                    self.rerun_or_detect();
                    ui.close();
                }
                ui.separator();
                for b in template::BUILTINS {
                    let label = tr!(lang, "menu.apply", name = crate::i18n::get(lang, b.name));
                    if ui.button(label).clicked() {
                        self.run_template(self.active, b.name, b.source);
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button(tr!(lang, "menu.editor")).clicked() {
                    self.tpl_editor.open = true;
                    ui.close();
                }
                if ui.button(tr!(lang, "menu.runfile")).clicked() {
                    ui.close();
                    self.run_template_file();
                }
                if ui.button(tr!(lang, "menu.clearresult")).clicked() {
                    if let Some(d) = self.doc_mut() {
                        d.template = None;
                        d.highlights.clear();
                    }
                    ui.close();
                }
            });
            ui.menu_button(tr!(lang, "menu.tools"), |ui| {
                if ui.button(tr!(lang, "menu.checksum")).clicked() {
                    self.right_tab = RightTab::Checksum;
                    ui.close();
                }
                if ui.button(tr!(lang, "menu.compare")).clicked() {
                    self.open_compare_dialog();
                    ui.close();
                }
                if ui.button(tr!(lang, "menu.histogram")).clicked() {
                    self.bottom_tab = BottomTab::Histogram;
                    ui.close();
                }
                if self.compare.pair.is_some() && ui.button(tr!(lang, "menu.endcompare")).clicked() {
                    self.end_compare();
                    ui.close();
                }
            });
            ui.menu_button(tr!(lang, "menu.help"), |ui| {
                if ui.button(tr!(lang, "menu.keys")).clicked() {
                    self.keys_open = true;
                    ui.close();
                }
                if ui.button(tr!(lang, "menu.about")).clicked() {
                    self.about_open = true;
                    ui.close();
                }
            });
        });
    }

    pub fn toolbar(&mut self, ui: &mut Ui) {
        let lang = self.lang;
        fn tool(ui: &mut Ui, icon: &str, tip: impl Into<egui::WidgetText>) -> egui::Response {
            ui.add(
                egui::Button::new(RichText::new(icon).size(15.0))
                    .frame(false)
                    .min_size(egui::vec2(28.0, 26.0)),
            )
            .on_hover_text(tip)
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            if tool(ui, "📄", tr!(lang, "tb.new", sc = tr!(lang, "sc.new"))).clicked() {
                self.new_doc();
            }
            if tool(ui, "📂", tr!(lang, "tb.open", sc = tr!(lang, "sc.open"))).clicked() {
                self.open_dialog();
            }
            if tool(ui, "💾", tr!(lang, "tb.save", sc = tr!(lang, "sc.save"))).clicked() {
                self.save(self.active, false);
            }
            ui.separator();
            if tool(ui, "⟲", tr!(lang, "tb.undo", sc = tr!(lang, "sc.undo"))).clicked() {
                self.undo();
            }
            if tool(ui, "⟳", tr!(lang, "tb.redo", sc = tr!(lang, "sc.redo"))).clicked() {
                self.redo();
            }
            ui.separator();
            if tool(ui, "🔍", tr!(lang, "tb.find", sc = tr!(lang, "sc.find"))).clicked() {
                self.open_find(false);
            }
            if tool(ui, "⇄", tr!(lang, "tb.replace", sc = tr!(lang, "sc.replace"))).clicked() {
                self.open_find(true);
            }
            if tool(ui, "📍", tr!(lang, "tb.goto", sc = tr!(lang, "sc.goto"))).clicked() {
                self.open_goto();
            }
            if tool(ui, "🔖", tr!(lang, "tb.bookmark", sc = tr!(lang, "sc.bookmark"))).clicked() {
                self.toggle_bookmark();
            }
            ui.separator();
            if tool(ui, "▶", tr!(lang, "tb.template")).clicked() {
                self.rerun_or_detect();
            }
            if tool(ui, "✏", tr!(lang, "tb.tpleditor")).clicked() {
                self.tpl_editor.open = true;
            }
            if tool(ui, "#", tr!(lang, "tb.checksum")).clicked() {
                self.right_tab = RightTab::Checksum;
            }
            if tool(ui, "⚖", tr!(lang, "tb.compare")).clicked() {
                self.open_compare_dialog();
            }
            if tool(ui, "📊", tr!(lang, "tb.histogram")).clicked() {
                self.bottom_tab = BottomTab::Histogram;
            }
            ui.separator();
            ui.label(RichText::new(tr!(lang, "tb.bytesperrow")).weak());
            egui::ComboBox::from_id_salt("bpr")
                .width(48.0)
                .selected_text(format!("{}", self.bytes_per_row))
                .show_ui(ui, |ui| {
                    for n in [8usize, 16, 24, 32] {
                        ui.selectable_value(&mut self.bytes_per_row, n, format!("{n}"));
                    }
                });
            ui.label(RichText::new(tr!(lang, "tb.endian")).weak());
            egui::ComboBox::from_id_salt("endian")
                .width(62.0)
                .selected_text(if self.little_endian { "Little" } else { "Big" })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.little_endian, true, "Little");
                    ui.selectable_value(&mut self.little_endian, false, "Big");
                });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let mode = tr!(
                    lang,
                    if self.insert { "tb.mode.insert" } else { "tb.mode.overwrite" }
                );
                if ui
                    .add(egui::Button::new(RichText::new(mode).monospace().strong()).min_size(egui::vec2(46.0, 22.0)))
                    .on_hover_text(tr!(lang, "tb.modetip", sc = tr!(lang, "sc.insert")))
                    .clicked()
                {
                    self.insert = !self.insert;
                }
                ui.separator();
                egui::ComboBox::from_id_salt("lang")
                    .width(130.0)
                    .selected_text(self.lang.name())
                    .show_ui(ui, |ui| {
                        for &l in Lang::ALL {
                            if ui.selectable_label(self.lang == l, l.name()).clicked() {
                                self.lang = l;
                                crate::i18n::save(l);
                            }
                        }
                    });
                ui.separator();
                ui.label(RichText::new("101Craft").strong().color(Color32::from_rgb(45, 120, 215)));
            });
        });
    }

    fn doc_tabs(&mut self, ui: &mut Ui) {
        let lang = self.lang;
        let mut close = None;
        let mut activate = None;
        egui::ScrollArea::horizontal()
            .id_salt("tabs")
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (i, d) in self.docs.iter().enumerate() {
                        let sel = i == self.active;
                        let v = ui.visuals();
                        let frame = egui::Frame::new()
                            .fill(if sel { v.extreme_bg_color } else { v.faint_bg_color })
                            .stroke(egui::Stroke::new(
                                1.0,
                                if sel { v.selection.bg_fill } else { v.widgets.noninteractive.bg_stroke.color },
                            ))
                            .corner_radius(egui::CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 })
                            .inner_margin(egui::Margin::symmetric(8, 3));
                        frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let icon = if d.template.is_some() { "◆ " } else { "◇ " };
                                let t = RichText::new(format!("{icon}{}", d.title()));
                                let t = if sel { t.strong() } else { t };
                                let tip = d
                                    .path
                                    .as_ref()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_else(|| tr!(lang, "tab.unsaved").into());
                                if ui
                                    .add(egui::Label::new(t).sense(egui::Sense::click()))
                                    .on_hover_text(tip)
                                    .clicked()
                                {
                                    activate = Some(i);
                                }
                                if ui
                                    .add(egui::Button::new(RichText::new("×").size(13.0)).frame(false))
                                    .on_hover_text(tr!(lang, "tab.close"))
                                    .clicked()
                                {
                                    close = Some(i);
                                }
                            });
                        });
                    }
                });
            });
        if let Some(i) = activate {
            self.active = i;
            self.focus_hex = true;
        }
        if let Some(i) = close {
            self.close_doc(i, false);
        }
    }

    fn find_bar(&mut self, ui: &mut Ui) {
        let lang = self.lang;
        let mut close = false;
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(egui::Margin::symmetric(6, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr!(lang, "find.label"));
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.find.text)
                            .desired_width(200.0)
                            .hint_text(tr!(lang, "find.hint")),
                    );
                    if self.find.focus {
                        r.request_focus();
                        self.find.focus = false;
                    }
                    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    egui::ComboBox::from_id_salt("findkind")
                        .selected_text(self.find.kind.label(lang))
                        .show_ui(ui, |ui| {
                            for k in [SearchKind::Text, SearchKind::TextUtf16, SearchKind::Hex, SearchKind::Int32] {
                                ui.selectable_value(&mut self.find.kind, k, k.label(lang));
                            }
                        });
                    if self.find.kind == SearchKind::Text {
                        ui.checkbox(&mut self.find.ignore_case, tr!(lang, "find.ignorecase"));
                    }
                    if ui
                        .button("◀")
                        .on_hover_text(tr!(lang, "find.prevtip", sc = tr!(lang, "sc.findprev")))
                        .clicked()
                    {
                        self.find_step(false);
                    }
                    if ui
                        .button(tr!(lang, "find.next"))
                        .on_hover_text(tr!(lang, "find.nexttip"))
                        .clicked()
                        || enter
                    {
                        self.find_step(true);
                        if enter {
                            r.request_focus();
                        }
                    }
                    if ui.button(tr!(lang, "find.all")).clicked() {
                        self.find_all();
                    }
                    ui.label(RichText::new(&self.find.message).weak());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new("✕").frame(false))
                            .on_hover_text(tr!(lang, "find.close"))
                            .clicked()
                        {
                            close = true;
                        }
                        if ui
                            .selectable_label(self.find.replace_mode, tr!(lang, "find.replace.toggle"))
                            .clicked()
                        {
                            self.find.replace_mode = !self.find.replace_mode;
                        }
                    });
                });
                if self.find.replace_mode {
                    ui.horizontal(|ui| {
                        ui.label(tr!(lang, "find.replace.label"));
                        ui.add(egui::TextEdit::singleline(&mut self.find.replace).desired_width(200.0));
                        if ui.button(tr!(lang, "find.replace.one")).clicked() {
                            self.replace_one();
                        }
                        if ui.button(tr!(lang, "find.replace.all")).clicked() {
                            self.replace_all();
                        }
                    });
                }
            });
        if close {
            self.find.open = false;
            if let Some(d) = self.doc_mut() {
                d.hits.clear();
            }
            self.focus_hex = true;
        }
    }

    pub fn status_bar(&mut self, ui: &mut Ui) {
        let lang = self.lang;
        ui.horizontal(|ui| {
            let Some(d) = self.docs.get(self.active) else {
                ui.label(tr!(lang, "status.nofile"));
                return;
            };
            let c = d.cursor;
            ui.label(RichText::new(format!("{} 0x{c:X} ({c})", tr!(lang, "status.pos"))).monospace());
            ui.separator();
            if let Some(b) = d.data.get(c) {
                let ch = if (0x20..0x7f).contains(b) {
                    format!(" '{}'", *b as char)
                } else {
                    String::new()
                };
                ui.label(
                    RichText::new(format!("{} 0x{b:02X} = {b}{ch}", tr!(lang, "status.value"))).monospace(),
                );
                ui.separator();
            }
            if let Some((s, e)) = d.selection() {
                ui.label(
                    RichText::new(tr!(
                        lang,
                        "status.selection",
                        s = format!("{s:X}"),
                        e = format!("{:X}", e - 1),
                        n = e - s
                    ))
                    .monospace(),
                );
                ui.separator();
            }
            ui.label(format!("{} {}", tr!(lang, "status.size"), tools::human_size(d.len(), lang)));
            if let Some(t) = &d.template {
                if let Some(last) = t.path_at(c).last() {
                    ui.separator();
                    ui.label(
                        RichText::new(format!(
                            "{} {}",
                            tr!(lang, "status.field"),
                            last.trim_start_matches('/').replace('/', ".")
                        ))
                        .monospace(),
                    );
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(tr!(
                        lang,
                        if self.insert { "tb.mode.insert" } else { "tb.mode.overwrite" }
                    ))
                    .monospace()
                    .strong(),
                );
                ui.separator();
                ui.label(if self.little_endian { "LE" } else { "BE" });
                ui.separator();
                ui.label(tr!(
                    lang,
                    match d.column {
                        Column::Hex => "status.column.hex",
                        Column::Text => "status.column.text",
                    }
                ));
                if d.modified {
                    ui.separator();
                    ui.label(RichText::new(tr!(lang, "status.modified")).color(Color32::from_rgb(220, 120, 0)));
                }
            });
        });
    }

    pub fn central(&mut self, ui: &mut Ui) {
        let lang = self.lang;
        if self.docs.is_empty() {
            self.hex_focused = false;
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() / 3.0);
                ui.heading("101Craft");
                ui.label(tr!(lang, "central.empty"));
                ui.add_space(8.0);
                if ui.button(tr!(lang, "central.open")).clicked() {
                    self.open_dialog();
                }
                if ui.button(tr!(lang, "central.new")).clicked() {
                    self.new_doc();
                }
            });
            return;
        }
        self.doc_tabs(ui);
        if self.find.open {
            self.find_bar(ui);
        }
        let pal = Palette::new(self.dark);
        let opts = ViewOptions {
            bytes_per_row: self.bytes_per_row,
            font_size: self.font_size,
        };
        // Side-by-side compare view.
        if let (Some((a, b)), true) = (self.compare.pair, self.compare.split) {
            if a < self.docs.len() && b < self.docs.len() && (self.active == a || self.active == b) {
                self.compare_view(ui, a, b, &opts, &pal);
                return;
            }
        }
        let idx = self.active;
        if self.docs[idx].text_mode {
            self.text_view(ui, idx);
            self.hex_focused = false;
            return;
        }
        let id = egui::Id::unique("hexview");
        if self.focus_hex {
            ui.memory_mut(|m| m.request_focus(id));
            self.focus_hex = false;
        }
        let r = hexview::hex_view(ui, &mut self.docs[idx], id, &opts, self.insert, &pal);
        self.hex_focused = r.focused;
        self.hovered_byte = r.hovered_byte;
        if r.toggle_insert {
            self.insert = !self.insert;
        }
    }

    fn compare_view(&mut self, ui: &mut Ui, a: usize, b: usize, opts: &ViewOptions, pal: &Palette) {
        let lang = self.lang;
        let w = ui.available_width();
        let h = ui.available_height();
        let mut res = Vec::new();
        ui.horizontal(|ui| {
            for (k, idx) in [a, b].into_iter().enumerate() {
                ui.allocate_ui(egui::vec2(w / 2.0 - 4.0, h), |ui| {
                    ui.vertical(|ui| {
                        let tag = tr!(lang, if k == 0 { "compare.a" } else { "compare.b" });
                        ui.label(RichText::new(format!("{} {}", tag, self.docs[idx].name)).strong());
                        let id = egui::Id::unique(("cmpview", k));
                        let r = hexview::hex_view(ui, &mut self.docs[idx], id, opts, self.insert, pal);
                        res.push((idx, r));
                    });
                });
            }
        });
        // Keep both sides scrolled together.
        let (ta, tb) = (self.docs[a].top_row, self.docs[b].top_row);
        if ta != self.compare.last_top {
            self.docs[b].top_row = ta;
            self.compare.last_top = ta;
        } else if tb != self.compare.last_top {
            self.docs[a].top_row = tb;
            self.compare.last_top = tb;
        }
        self.hex_focused = false;
        for (idx, r) in res {
            if r.focused {
                self.active = idx;
                self.hex_focused = true;
            }
            if r.toggle_insert {
                self.insert = !self.insert;
            }
        }
    }

    fn text_view(&mut self, ui: &mut Ui, idx: usize) {
        let lang = self.lang;
        let font = egui::FontId::monospace(self.font_size);
        let d = &mut self.docs[idx];
        if d.len() > 8 * 1024 * 1024 {
            ui.label(tr!(lang, "text.toobig"));
            return;
        }
        let valid = std::str::from_utf8(&d.data).is_ok();
        if d.text_dirty_from_data {
            d.text_buf = String::from_utf8_lossy(&d.data).into_owned();
            d.text_dirty_from_data = false;
        }
        if !valid {
            ui.label(
                RichText::new(tr!(lang, "text.invalid")).color(Color32::from_rgb(200, 110, 0)),
            );
        }
        egui::ScrollArea::both()
            .id_salt("textview")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut buf = d.text_buf.clone();
                let r = ui.add_enabled(
                    valid,
                    egui::TextEdit::multiline(&mut buf)
                        .font(font)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(40),
                );
                if r.changed() && valid {
                    let len = d.data.len();
                    let new = buf.clone().into_bytes();
                    d.splice(0, len, &new);
                    d.text_buf = buf;
                    d.text_dirty_from_data = false;
                }
            });
    }
}
