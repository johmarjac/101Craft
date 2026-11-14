//! A document: the bytes of one file plus cursor, selection, undo history and markers.

use std::path::PathBuf;

use egui::Color32;

use crate::template::TemplateResult;

/// One reversible change: at `pos`, `old` was replaced by `new`.
#[derive(Clone, Debug)]
pub struct Splice {
    pub pos: usize,
    pub old: Vec<u8>,
    pub new: Vec<u8>,
    /// Cursor position before the change (restored on undo).
    pub cursor_before: usize,
}

#[derive(Clone, Debug)]
pub struct Bookmark {
    pub name: String,
    pub start: usize,
    pub len: usize,
    pub color: Color32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Column {
    Hex,
    Text,
}

pub struct Document {
    pub name: String,
    pub path: Option<PathBuf>,
    pub data: Vec<u8>,
    pub modified: bool,
    undo: Vec<Splice>,
    redo: Vec<Splice>,
    pub cursor: usize,
    pub anchor: Option<usize>,
    pub nibble_low: bool,
    pub column: Column,
    pub top_row: usize,
    pub scroll_to_cursor: bool,
    pub bookmarks: Vec<Bookmark>,
    pub template: Option<TemplateResult>,
    /// Sorted, non-overlapping coloured ranges (start, end exclusive, colour) from a template.
    pub highlights: Vec<(usize, usize, Color32)>,
    /// Ranges that differ from the compared file.
    pub diff_ranges: Vec<(usize, usize)>,
    /// Search hits (start, len).
    pub hits: Vec<(usize, usize)>,
    pub text_mode: bool,
    pub text_buf: String,
    pub text_dirty_from_data: bool,
    /// Bumped on every change, so caches can tell when to recompute.
    pub revision: u64,
}

impl Document {
    pub fn new(name: impl Into<String>, data: Vec<u8>, path: Option<PathBuf>) -> Self {
        Self {
            name: name.into(),
            path,
            data,
            modified: false,
            undo: Vec::new(),
            redo: Vec::new(),
            cursor: 0,
            anchor: None,
            nibble_low: false,
            column: Column::Hex,
            top_row: 0,
            scroll_to_cursor: false,
            bookmarks: Vec::new(),
            template: None,
            highlights: Vec::new(),
            diff_ranges: Vec::new(),
            hits: Vec::new(),
            text_mode: false,
            text_buf: String::new(),
            text_dirty_from_data: true,
            revision: 1,
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn title(&self) -> String {
        if self.modified {
            format!("{} *", self.name)
        } else {
            self.name.clone()
        }
    }

    /// Selection as (start, end exclusive), if any.
    pub fn selection(&self) -> Option<(usize, usize)> {
        let a = self.anchor?;
        let len = self.data.len();
        if len == 0 {
            return None;
        }
        let c = self.cursor.min(len - 1);
        let a = a.min(len - 1);
        Some((a.min(c), a.max(c) + 1))
    }

    /// The selection, or the single byte under the cursor.
    pub fn selection_or_cursor(&self) -> Option<(usize, usize)> {
        if let Some(s) = self.selection() {
            return Some(s);
        }
        if self.cursor < self.data.len() {
            Some((self.cursor, self.cursor + 1))
        } else {
            None
        }
    }

    pub fn selected_bytes(&self) -> &[u8] {
        match self.selection() {
            Some((s, e)) => &self.data[s..e],
            None => &[],
        }
    }

    pub fn set_cursor(&mut self, pos: usize, extend: bool) {
        let pos = pos.min(self.data.len());
        if extend {
            if self.anchor.is_none() {
                self.anchor = Some(self.cursor);
            }
        } else {
            self.anchor = None;
        }
        self.cursor = pos;
        self.nibble_low = false;
        self.scroll_to_cursor = true;
    }

    pub fn select_range(&mut self, start: usize, len: usize) {
        if self.data.is_empty() {
            return;
        }
        let start = start.min(self.data.len() - 1);
        let end = (start + len.max(1) - 1).min(self.data.len() - 1);
        self.anchor = Some(start);
        self.cursor = end;
        self.nibble_low = false;
        self.scroll_to_cursor = true;
    }

    fn apply(&mut self, s: &Splice) {
        let end = s.pos + s.old.len();
        self.data.splice(s.pos..end, s.new.iter().copied());
    }

    fn changed(&mut self) {
        self.modified = true;
        self.revision += 1;
        self.text_dirty_from_data = true;
        self.hits.clear();
        if self.cursor > self.data.len() {
            self.cursor = self.data.len();
        }
    }

    /// Replace `old_len` bytes at `pos` with `new`, recording it for undo.
    pub fn splice(&mut self, pos: usize, old_len: usize, new: &[u8]) {
        let pos = pos.min(self.data.len());
        let old_len = old_len.min(self.data.len() - pos);
        let old = self.data[pos..pos + old_len].to_vec();
        if old == new {
            return;
        }
        let s = Splice {
            pos,
            old,
            new: new.to_vec(),
            cursor_before: self.cursor,
        };
        self.apply(&s);
        // Merge single-byte overwrites of the same byte (typing two hex digits).
        let merged = if let Some(last) = self.undo.last_mut() {
            if last.pos == s.pos && last.new.len() == 1 && s.old.len() == 1 && s.new.len() == 1 && self.nibble_low {
                last.new = s.new.clone();
                true
            } else {
                false
            }
        } else {
            false
        };
        if !merged {
            self.undo.push(s);
        }
        self.redo.clear();
        self.changed();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) {
        if let Some(s) = self.undo.pop() {
            let inv = Splice {
                pos: s.pos,
                old: s.new.clone(),
                new: s.old.clone(),
                cursor_before: s.cursor_before,
            };
            self.apply(&inv);
            self.cursor = s.cursor_before.min(self.data.len());
            self.anchor = None;
            self.nibble_low = false;
            self.redo.push(s);
            self.changed();
            self.scroll_to_cursor = true;
            if self.undo.is_empty() {
                self.modified = false;
            }
        }
    }

    pub fn redo(&mut self) {
        if let Some(s) = self.redo.pop() {
            self.apply(&s);
            self.cursor = (s.pos + s.new.len()).min(self.data.len());
            self.anchor = None;
            self.nibble_low = false;
            self.undo.push(s);
            self.changed();
            self.scroll_to_cursor = true;
        }
    }

    pub fn mark_saved(&mut self) {
        self.modified = false;
    }

    /// Delete the selection (or the byte at the cursor).
    pub fn delete_forward(&mut self) {
        if let Some((s, e)) = self.selection() {
            self.splice(s, e - s, &[]);
            self.cursor = s;
            self.anchor = None;
        } else if self.cursor < self.data.len() {
            self.splice(self.cursor, 1, &[]);
        }
        self.nibble_low = false;
    }

    pub fn delete_backward(&mut self) {
        if self.selection().is_some() {
            self.delete_forward();
        } else if self.cursor > 0 {
            let p = self.cursor - 1;
            self.splice(p, 1, &[]);
            self.cursor = p;
        }
        self.nibble_low = false;
    }

    /// Write bytes at the cursor, inserting or overwriting.
    pub fn write_bytes(&mut self, bytes: &[u8], insert: bool) {
        if let Some((s, e)) = self.selection() {
            // Typing over a selection replaces it.
            self.splice(s, e - s, bytes);
            self.cursor = (s + bytes.len()).min(self.data.len());
            self.anchor = None;
            return;
        }
        let pos = self.cursor;
        if insert {
            self.splice(pos, 0, bytes);
        } else {
            let old_len = bytes.len().min(self.data.len().saturating_sub(pos));
            self.splice(pos, old_len, bytes);
        }
        self.cursor = (pos + bytes.len()).min(self.data.len());
    }

    /// Hex digit typed in the hex column.
    pub fn type_nibble(&mut self, v: u8, insert: bool) {
        if self.selection().is_some() {
            let (s, e) = self.selection().unwrap();
            self.splice(s, e - s, &[v << 4]);
            self.cursor = s;
            self.anchor = None;
            self.nibble_low = true;
            return;
        }
        let pos = self.cursor;
        if !self.nibble_low {
            if insert || pos >= self.data.len() {
                self.splice(pos, 0, &[v << 4]);
            } else {
                let b = (self.data[pos] & 0x0F) | (v << 4);
                self.splice(pos, 1, &[b]);
            }
            self.nibble_low = true;
        } else {
            if pos < self.data.len() {
                let b = (self.data[pos] & 0xF0) | v;
                self.splice(pos, 1, &[b]);
            }
            self.nibble_low = false;
            self.cursor = (pos + 1).min(self.data.len());
        }
        self.scroll_to_cursor = true;
    }

    pub fn toggle_bookmark(&mut self, palette_idx: usize, lang: crate::i18n::Lang) {
        let (s, e) = match self.selection_or_cursor() {
            Some(r) => r,
            None => return,
        };
        if let Some(i) = self
            .bookmarks
            .iter()
            .position(|b| b.start == s && b.len == e - s)
        {
            self.bookmarks.remove(i);
            return;
        }
        let colors = [
            Color32::from_rgb(255, 214, 120),
            Color32::from_rgb(150, 220, 255),
            Color32::from_rgb(190, 240, 160),
            Color32::from_rgb(255, 170, 200),
            Color32::from_rgb(210, 190, 255),
        ];
        let n = self.bookmarks.len() + 1;
        self.bookmarks.push(Bookmark {
            name: crate::tr!(lang, "msg.bookmark", n = n),
            start: s,
            len: e - s,
            color: colors[(palette_idx + n) % colors.len()],
        });
        self.bookmarks.sort_by_key(|b| b.start);
    }

    pub fn bookmark_at(&self, pos: usize) -> Option<&Bookmark> {
        self.bookmarks
            .iter()
            .find(|b| pos >= b.start && pos < b.start + b.len)
    }

    pub fn highlight_at(&self, pos: usize) -> Option<Color32> {
        let i = self.highlights.partition_point(|h| h.1 <= pos);
        let h = self.highlights.get(i)?;
        if pos >= h.0 && pos < h.1 {
            Some(h.2)
        } else {
            None
        }
    }

    pub fn in_ranges(ranges: &[(usize, usize)], pos: usize) -> bool {
        let i = ranges.partition_point(|r| r.1 <= pos);
        matches!(ranges.get(i), Some(r) if pos >= r.0 && pos < r.1)
    }

    pub fn is_hit(&self, pos: usize) -> bool {
        let i = self.hits.partition_point(|h| h.0 + h.1 <= pos);
        matches!(self.hits.get(i), Some(h) if pos >= h.0 && pos < h.0 + h.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_and_undo() {
        let mut d = Document::new("t", vec![0x00, 0x11, 0x22], None);
        d.type_nibble(0xA, false);
        d.type_nibble(0xB, false);
        assert_eq!(d.data, vec![0xAB, 0x11, 0x22]);
        assert_eq!(d.cursor, 1);
        d.type_nibble(0xC, true);
        d.type_nibble(0xD, true);
        assert_eq!(d.data, vec![0xAB, 0xCD, 0x11, 0x22]);
        d.undo();
        assert_eq!(d.data, vec![0xAB, 0x11, 0x22]);
        d.undo();
        assert_eq!(d.data, vec![0x00, 0x11, 0x22]);
        assert!(!d.modified);
        d.redo();
        assert_eq!(d.data, vec![0xAB, 0x11, 0x22]);
        d.set_cursor(1, false);
        d.set_cursor(2, true);
        d.delete_forward();
        assert_eq!(d.data, vec![0xAB]);
        d.write_bytes(b"hi", true);
        assert_eq!(d.data, vec![0xAB, b'h', b'i']);
    }
}
