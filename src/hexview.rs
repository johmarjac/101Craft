//! The hex editor view: address column, hex bytes, text column and a scrollbar.

use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, Event, EventFilter, FontId, Id, Key, Rect, Sense,
    Stroke, StrokeKind, Ui,
};

use crate::doc::{Column, Document};

pub struct Palette {
    pub bg: Color32,
    pub bg_alt: Color32,
    pub header_bg: Color32,
    pub header_fg: Color32,
    pub addr_fg: Color32,
    pub byte_fg: Color32,
    pub zero_fg: Color32,
    pub sel_bg: Color32,
    pub sel_fg: Color32,
    pub cursor: Color32,
    pub hit_bg: Color32,
    pub diff_bg: Color32,
    pub diff_fg: Color32,
    pub line: Color32,
    pub dark: bool,
}

impl Palette {
    pub fn new(dark: bool) -> Self {
        if dark {
            Palette {
                bg: Color32::from_rgb(30, 32, 38),
                bg_alt: Color32::from_rgb(35, 37, 44),
                header_bg: Color32::from_rgb(42, 45, 54),
                header_fg: Color32::from_rgb(150, 170, 200),
                addr_fg: Color32::from_rgb(110, 160, 220),
                byte_fg: Color32::from_rgb(220, 222, 228),
                zero_fg: Color32::from_rgb(128, 134, 148),
                sel_bg: Color32::from_rgb(38, 110, 200),
                sel_fg: Color32::WHITE,
                cursor: Color32::from_rgb(255, 160, 40),
                hit_bg: Color32::from_rgb(120, 105, 20),
                diff_bg: Color32::from_rgb(120, 40, 45),
                diff_fg: Color32::from_rgb(255, 200, 200),
                line: Color32::from_rgb(58, 62, 72),
                dark,
            }
        } else {
            Palette {
                bg: Color32::from_rgb(255, 255, 255),
                bg_alt: Color32::from_rgb(247, 249, 252),
                header_bg: Color32::from_rgb(234, 239, 246),
                header_fg: Color32::from_rgb(70, 90, 125),
                addr_fg: Color32::from_rgb(30, 90, 170),
                byte_fg: Color32::from_rgb(25, 28, 35),
                zero_fg: Color32::from_rgb(160, 165, 175),
                sel_bg: Color32::from_rgb(45, 120, 215),
                sel_fg: Color32::WHITE,
                cursor: Color32::from_rgb(230, 110, 0),
                hit_bg: Color32::from_rgb(255, 236, 120),
                diff_bg: Color32::from_rgb(255, 205, 205),
                diff_fg: Color32::from_rgb(160, 0, 0),
                line: Color32::from_rgb(215, 222, 232),
                dark,
            }
        }
    }

    /// Template and bookmark colours are pastel; dim them on a dark background.
    pub fn tint(&self, c: Color32) -> Color32 {
        if self.dark {
            Color32::from_rgb(
                (c.r() as u16 * 28 / 100 + 14) as u8,
                (c.g() as u16 * 28 / 100 + 14) as u8,
                (c.b() as u16 * 28 / 100 + 18) as u8,
            )
        } else {
            c
        }
    }
}

pub struct ViewOptions {
    pub bytes_per_row: usize,
    pub font_size: f32,
    /// "Allow Delete in Overwrite Mode" (options default 1 at 0x1403d6d33).
    pub allow_delete_overwrite: bool,
}

#[derive(Default)]
pub struct ViewResult {
    pub focused: bool,
    pub toggle_insert: bool,
    pub hovered_byte: Option<usize>,
    pub message: Option<crate::doc::EditMsg>,
}

const SCROLLBAR_W: f32 = 14.0;

/// Draw the hex view filling the available space, and handle its input.
pub fn hex_view(ui: &mut Ui, doc: &mut Document, id: Id, opts: &ViewOptions, insert: bool, pal: &Palette) -> ViewResult {
    let mut out = ViewResult::default();
    let bpr = opts.bytes_per_row.max(1);
    let font = FontId::monospace(opts.font_size);
    let cw = ui
        .painter()
        .layout_no_wrap("0".into(), font.clone(), Color32::WHITE)
        .size()
        .x;
    let rh = (opts.font_size * 1.5).round();

    let (outer, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    let rect = Rect::from_min_max(outer.min, pos2(outer.max.x - SCROLLBAR_W, outer.max.y));
    let sb_rect = Rect::from_min_max(pos2(rect.max.x, outer.min.y), outer.max);
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let painter = ui.painter_at(outer);

    let len = doc.len();
    let addr_digits = format!("{:X}", len.max(1)).len().max(8);
    let x_addr = rect.min.x + cw;
    let x_hex = x_addr + (addr_digits as f32 + 2.0) * cw;
    let group_gap = cw * 0.8;
    let byte_x = |i: usize| x_hex + i as f32 * 3.0 * cw + (i / 4) as f32 * group_gap;
    let hex_w = byte_x(bpr) - x_hex;
    let x_txt = x_hex + hex_w + 1.5 * cw;
    let header_h = rh + 4.0;
    let body_top = rect.min.y + header_h;

    let rows = len / bpr + 1;
    let visible = (((rect.max.y - body_top) / rh).floor() as usize).max(1);
    let max_top = rows.saturating_sub(visible);

    // --- focus
    if resp.clicked() || resp.drag_started() {
        resp.request_focus();
    }
    let focused = resp.has_focus();
    out.focused = focused;
    if focused {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                id,
                EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: false,
                },
            )
        });
    }

    // --- mouse wheel
    if resp.hovered() || ui.rect_contains_pointer(sb_rect) {
        let dy = ui.input(|i| i.smooth_scroll_delta.y);
        if dy != 0.0 {
            let acc_id = id.with("scroll_acc");
            let mut acc: f32 = ui.data(|d| d.get_temp(acc_id).unwrap_or(0.0));
            acc += dy;
            let step = rh;
            let mut top = doc.top_row as i64;
            while acc >= step {
                top -= 1;
                acc -= step;
            }
            while acc <= -step {
                top += 1;
                acc += step;
            }
            doc.top_row = top.clamp(0, max_top as i64) as usize;
            ui.data_mut(|d| d.insert_temp(acc_id, acc));
        }
    }

    // --- hit testing
    let hit = |p: egui::Pos2, top: usize| -> Option<(usize, Column)> {
        let row_f = ((p.y - body_top) / rh).floor();
        let row = (top as i64 + row_f as i64).max(0) as usize;
        let col_hex = if p.x < x_txt - cw * 0.75 {
            let mut best = 0;
            for i in 0..bpr {
                if p.x >= byte_x(i) - cw * 0.5 {
                    best = i;
                }
            }
            Some((best, Column::Hex))
        } else {
            let i = ((p.x - x_txt) / cw).floor().max(0.0) as usize;
            Some((i.min(bpr - 1), Column::Text))
        };
        col_hex.map(|(c, col)| ((row * bpr + c).min(len), col))
    };

    let shift = ui.input(|i| i.modifiers.shift);
    let drag_id = id.with("drag_origin");
    if let Some(p) = resp.interact_pointer_pos() {
        if ui.input(|i| i.pointer.primary_pressed()) && rect.contains(p) {
            if let Some((pos, col)) = hit(p, doc.top_row) {
                doc.column = col;
                if shift {
                    doc.set_cursor(pos, true);
                } else {
                    doc.set_cursor(pos, false);
                }
                ui.data_mut(|d| d.insert_temp(drag_id, doc.anchor.unwrap_or(pos)));
            }
        } else if resp.dragged() {
            if p.y < body_top && doc.top_row > 0 {
                doc.top_row -= 1;
            } else if p.y > rect.max.y && doc.top_row < max_top {
                doc.top_row += 1;
            }
            let pc = pos2(p.x, p.y.clamp(body_top + 1.0, rect.max.y - 1.0));
            if let Some((pos, _)) = hit(pc, doc.top_row) {
                let origin: usize = ui.data(|d| d.get_temp(drag_id).unwrap_or(pos));
                if pos != origin || doc.anchor.is_some() {
                    doc.anchor = Some(origin);
                    doc.cursor = pos.min(len.saturating_sub(1));
                    doc.nibble_low = false;
                }
            }
        }
    }
    if let Some(p) = resp.hover_pos() {
        if p.y >= body_top {
            if let Some((pos, _)) = hit(p, doc.top_row) {
                if pos < len {
                    out.hovered_byte = Some(pos);
                }
            }
        }
    }

    // --- keyboard
    if focused {
        let events = ui.input(|i| i.events.clone());
        for ev in events {
            handle_event(ui, doc, ev, bpr, visible, insert, opts.allow_delete_overwrite, &mut out);
        }
    }

    // Keys may have inserted or deleted bytes: everything below must use the new length,
    // or painting reads past the end of the data.
    let len = doc.len();
    let rows = len / bpr + 1;
    let max_top = rows.saturating_sub(visible);
    doc.cursor = doc.cursor.min(len);
    if let Some(a) = doc.anchor {
        if len == 0 {
            doc.anchor = None;
        } else {
            doc.anchor = Some(a.min(len - 1));
        }
    }

    // --- keep cursor visible
    if doc.scroll_to_cursor {
        let crow = doc.cursor / bpr;
        if crow < doc.top_row {
            doc.top_row = crow;
        } else if crow >= doc.top_row + visible {
            doc.top_row = crow + 1 - visible;
        }
        doc.scroll_to_cursor = false;
    }
    doc.top_row = doc.top_row.min(max_top);

    // --- scrollbar
    let sb_resp = ui.interact(sb_rect, id.with("scrollbar"), Sense::click_and_drag());
    let track = sb_rect.shrink2(vec2(3.0, 3.0));
    let thumb_h = if rows <= visible {
        track.height()
    } else {
        (track.height() * visible as f32 / rows as f32).max(24.0)
    };
    let frac = if max_top == 0 { 0.0 } else { doc.top_row as f32 / max_top as f32 };
    let thumb_y = track.min.y + (track.height() - thumb_h) * frac;
    if let Some(p) = sb_resp.interact_pointer_pos() {
        if (sb_resp.dragged() || sb_resp.clicked()) && max_top > 0 {
            let f = ((p.y - track.min.y - thumb_h / 2.0) / (track.height() - thumb_h)).clamp(0.0, 1.0);
            doc.top_row = (f * max_top as f32).round() as usize;
        }
    }

    // --- paint
    painter.rect_filled(rect, CornerRadius::ZERO, pal.bg);
    let header = Rect::from_min_max(rect.min, pos2(rect.max.x, body_top));
    painter.rect_filled(header, CornerRadius::ZERO, pal.header_bg);
    painter.hline(rect.x_range(), body_top - 0.5, Stroke::new(1.0, pal.line));
    let hy = rect.min.y + 2.0;
    painter.text(pos2(x_addr, hy), Align2::LEFT_TOP, "Adresse", font.clone(), pal.header_fg);
    let cur_col = doc.cursor % bpr;
    for i in 0..bpr {
        let c = if i == cur_col { pal.cursor } else { pal.header_fg };
        painter.text(pos2(byte_x(i), hy), Align2::LEFT_TOP, format!("{i:02X}"), font.clone(), c);
    }
    let mut hdr_txt = String::new();
    for i in 0..bpr {
        hdr_txt.push(std::char::from_digit((i % 16) as u32, 16).unwrap().to_ascii_uppercase());
    }
    painter.text(pos2(x_txt, hy), Align2::LEFT_TOP, hdr_txt, font.clone(), pal.header_fg);
    // separators
    let sep_x1 = x_hex - cw;
    let sep_x2 = x_txt - 0.75 * cw;
    painter.vline(sep_x1, rect.y_range(), Stroke::new(1.0, pal.line));
    painter.vline(sep_x2, rect.y_range(), Stroke::new(1.0, pal.line));

    let sel = doc.selection();
    let has_tpl = !doc.highlights.is_empty();
    for vr in 0..=visible {
        let row = doc.top_row + vr;
        if row >= rows {
            break;
        }
        let y = body_top + vr as f32 * rh;
        if y > rect.max.y {
            break;
        }
        let row_rect = Rect::from_min_max(pos2(rect.min.x, y), pos2(rect.max.x, y + rh));
        if row % 2 == 1 {
            painter.rect_filled(row_rect, CornerRadius::ZERO, pal.bg_alt);
        }
        let start = row * bpr;
        let ty = y + (rh - opts.font_size) / 2.0 - 1.0;
        let addr_col = if doc.cursor / bpr == row { pal.cursor } else { pal.addr_fg };
        painter.text(
            pos2(x_addr, ty),
            Align2::LEFT_TOP,
            format!("{:0w$X}h", start, w = addr_digits),
            font.clone(),
            addr_col,
        );
        for i in 0..bpr {
            let pos = start + i;
            if pos > len {
                break;
            }
            let bx = byte_x(i);
            let hex_cell = Rect::from_min_size(pos2(bx - cw * 0.35, y + 1.0), vec2(cw * 2.7, rh - 2.0));
            let txt_cell = Rect::from_min_size(pos2(x_txt + i as f32 * cw, y + 1.0), vec2(cw, rh - 2.0));
            if pos == len {
                // End-of-file position: only the cursor can be here.
                if doc.cursor == len {
                    draw_cursor(&painter, doc, hex_cell, txt_cell, pal);
                }
                break;
            }
            let Some(&b) = doc.data.get(pos) else { break };
            let selected = matches!(sel, Some((s, e)) if pos >= s && pos < e);
            let mut fg = if b == 0 { pal.zero_fg } else { pal.byte_fg };
            let mut bg = None;
            if selected {
                bg = Some(pal.sel_bg);
                fg = pal.sel_fg;
            } else if doc.is_hit(pos) {
                bg = Some(pal.hit_bg);
            } else if Document::in_ranges(&doc.diff_ranges, pos) {
                bg = Some(pal.diff_bg);
                fg = pal.diff_fg;
            } else if let Some(bm) = doc.bookmark_at(pos) {
                bg = Some(pal.tint(bm.color));
            } else if has_tpl {
                bg = doc.highlight_at(pos).map(|c| pal.tint(c));
            }
            if let Some(bg) = bg {
                // Join neighbouring cells of the same colour into one band.
                let join = if i + 1 < bpr && pos + 1 < len { cw * 0.3 + if (i + 1) % 4 == 0 { group_gap } else { 0.0 } } else { 0.0 };
                let band = Rect::from_min_max(hex_cell.min, pos2(hex_cell.max.x + join, hex_cell.max.y));
                painter.rect_filled(band, CornerRadius::ZERO, bg);
                painter.rect_filled(txt_cell, CornerRadius::ZERO, bg);
            }
            painter.text(pos2(bx, ty), Align2::LEFT_TOP, format!("{b:02X}"), font.clone(), fg);
            let ch = if (0x20..0x7f).contains(&b) { b as char } else { '.' };
            let tfg = if (0x20..0x7f).contains(&b) || selected { fg } else { pal.zero_fg };
            painter.text(pos2(txt_cell.min.x, ty), Align2::LEFT_TOP, ch.to_string(), font.clone(), tfg);
            if pos == doc.cursor {
                draw_cursor(&painter, doc, hex_cell, txt_cell, pal);
            }
        }
    }
    if focused {
        painter.rect_stroke(rect.shrink(0.5), CornerRadius::ZERO, Stroke::new(1.0, pal.sel_bg.gamma_multiply(0.6)), StrokeKind::Inside);
    }

    // scrollbar paint
    painter.rect_filled(sb_rect, CornerRadius::ZERO, pal.header_bg);
    let thumb = Rect::from_min_size(pos2(track.min.x, thumb_y), vec2(track.width(), thumb_h));
    let tc = if sb_resp.dragged() || sb_resp.hovered() { pal.addr_fg } else { pal.zero_fg };
    painter.rect_filled(thumb, CornerRadius::same(4), tc.gamma_multiply(0.8));

    out
}

fn draw_cursor(painter: &egui::Painter, doc: &Document, hex_cell: Rect, txt_cell: Rect, pal: &Palette) {
    let (active, passive) = match doc.column {
        Column::Hex => (hex_cell, txt_cell),
        Column::Text => (txt_cell, hex_cell),
    };
    if doc.nibble_low && doc.column == Column::Hex {
        let half = Rect::from_min_max(pos2(active.center().x, active.min.y), active.max);
        painter.rect_stroke(half, CornerRadius::same(2), Stroke::new(2.0, pal.cursor), StrokeKind::Inside);
    } else {
        painter.rect_stroke(active, CornerRadius::same(2), Stroke::new(2.0, pal.cursor), StrokeKind::Inside);
    }
    painter.rect_stroke(passive, CornerRadius::same(2), Stroke::new(1.0, pal.cursor.gamma_multiply(0.6)), StrokeKind::Inside);
}

fn handle_event(
    ui: &mut Ui,
    doc: &mut Document,
    ev: Event,
    bpr: usize,
    visible: usize,
    insert: bool,
    allow_delete_overwrite: bool,
    out: &mut ViewResult,
) {
    use crate::doc::EditMsg;
    // 0x1401428b0: in overwrite mode deleting is refused unless the option allows it.
    let may_delete = insert || allow_delete_overwrite;
    let len = doc.len();
    match ev {
        Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => {
            let ext = modifiers.shift;
            let cur = doc.cursor;
            let max_pos = len;
            match key {
                Key::ArrowLeft => doc.set_cursor(cur.saturating_sub(1), ext),
                Key::ArrowRight => doc.set_cursor((cur + 1).min(max_pos), ext),
                Key::ArrowUp => {
                    if cur >= bpr {
                        doc.set_cursor(cur - bpr, ext)
                    }
                }
                Key::ArrowDown => doc.set_cursor((cur + bpr).min(max_pos), ext),
                Key::PageUp => {
                    let d = bpr * visible.saturating_sub(1).max(1);
                    doc.top_row = doc.top_row.saturating_sub(visible.saturating_sub(1).max(1));
                    doc.set_cursor(cur.saturating_sub(d), ext);
                }
                Key::PageDown => {
                    let d = bpr * visible.saturating_sub(1).max(1);
                    doc.top_row += visible.saturating_sub(1).max(1);
                    doc.set_cursor((cur + d).min(max_pos), ext);
                }
                Key::Home => {
                    if modifiers.command {
                        doc.set_cursor(0, ext)
                    } else {
                        doc.set_cursor(cur - cur % bpr, ext)
                    }
                }
                Key::End => {
                    if modifiers.command {
                        doc.set_cursor(max_pos, ext)
                    } else {
                        doc.set_cursor((cur - cur % bpr + bpr - 1).min(max_pos), ext)
                    }
                }
                Key::Tab => {
                    doc.column = match doc.column {
                        Column::Hex => Column::Text,
                        Column::Text => Column::Hex,
                    };
                    doc.nibble_low = false;
                }
                Key::Insert => out.toggle_insert = true,
                Key::Delete | Key::Backspace if !may_delete => {
                    out.message = Some(EditMsg::DeleteRefused);
                }
                Key::Delete => {
                    let n = doc.delete_forward();
                    if n > 0 {
                        out.message = Some(EditMsg::Deleted(n));
                    }
                }
                // Backspace and Shift+Backspace both mean "Delete Previous" (0x1404149c0).
                Key::Backspace => {
                    let n = doc.delete_backward();
                    if n > 0 {
                        out.message = Some(EditMsg::Deleted(n));
                    }
                }
                Key::Escape => {
                    doc.anchor = None;
                    doc.nibble_low = false;
                }
                Key::A if modifiers.command => {
                    if len > 0 {
                        doc.anchor = Some(0);
                        doc.cursor = len - 1;
                    }
                }
                _ => {}
            }
        }
        Event::Text(t) => {
            // Ctrl/Alt combos must not type.
            if ui.input(|i| i.modifiers.command || i.modifiers.alt) {
                return;
            }
            match doc.column {
                Column::Hex => {
                    for c in t.chars() {
                        if let Some(v) = c.to_digit(16) {
                            doc.type_nibble(v as u8, insert);
                        }
                    }
                }
                Column::Text => {
                    let mut bytes = Vec::new();
                    for c in t.chars() {
                        if (c as u32) < 256 {
                            bytes.push(c as u32 as u8);
                        } else {
                            let mut buf = [0u8; 4];
                            bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                        }
                    }
                    doc.write_bytes(&bytes, insert);
                    doc.scroll_to_cursor = true;
                }
            }
        }
        Event::Copy => {
            if let Some(text) = copy_text(doc) {
                ui.ctx().copy_text(text);
            }
        }
        // 0x140142730: Cut needs a selection, then copies and deletes it.
        Event::Cut => {
            if doc.selection().is_none() {
                out.message = Some(EditMsg::NoSelection);
            } else if !may_delete {
                out.message = Some(EditMsg::DeleteRefused);
            } else if let Some(text) = copy_text(doc) {
                ui.ctx().copy_text(text);
                let n = doc.delete_forward();
                out.message = Some(EditMsg::Cut(n));
            }
        }
        Event::Paste(s) => {
            let bytes = match doc.column {
                Column::Hex => crate::tools::parse_hex(&s).unwrap_or_else(|| s.as_bytes().to_vec()),
                Column::Text => s.as_bytes().to_vec(),
            };
            if !bytes.is_empty() {
                doc.write_bytes(&bytes, insert);
                doc.scroll_to_cursor = true;
            }
        }
        _ => {}
    }
}

pub fn copy_text(doc: &Document) -> Option<String> {
    let bytes = doc.selected_bytes();
    let bytes = if bytes.is_empty() {
        doc.data.get(doc.cursor..doc.cursor + 1)?
    } else {
        bytes
    };
    Some(match doc.column {
        Column::Hex => crate::tools::hex_spaced(bytes),
        Column::Text => bytes.iter().map(|b| *b as char).collect(),
    })
}
