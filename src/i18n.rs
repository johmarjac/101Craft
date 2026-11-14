//! UI strings in two languages. English is the default; German falls back to English.
//!
//! Keys are dotted strings (`"menu.file"`). `get` returns the key itself for unknown
//! keys, so free-form text such as file names can be passed straight through.

use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Lang {
    #[default]
    En,
    De,
}

impl Lang {
    /// Name shown in the switcher (endonyms, never translated).
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::De => "Deutsch",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::De => "de",
        }
    }

    pub fn from_code(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "de" | "deutsch" | "german" => Lang::De,
            _ => Lang::En,
        }
    }
}

/// Look up a UI string. Unknown keys return the key; missing German falls back to English.
pub fn get(lang: Lang, key: &str) -> String {
    let de = if lang == Lang::De { de(key) } else { None };
    match de.or_else(|| en(key)) {
        Some(s) => s.to_owned(),
        None => key.to_owned(),
    }
}

/// Fetch a string, then substitute `{name}` placeholders.
///
/// ```
/// let s = tr!(Lang::En, "msg.opened", name = "a.bin", size = "1 KB");
/// ```
#[macro_export]
macro_rules! tr {
    ($lang:expr, $key:expr) => {
        $crate::i18n::get($lang, $key)
    };
    ($lang:expr, $key:expr, $($name:ident = $val:expr),+ $(,)?) => {{
        let mut s: String = $crate::i18n::get($lang, $key).to_string();
        $(
            s = s.replace(concat!("{", stringify!($name), "}"), &$val.to_string());
        )+
        s
    }};
}

// ---------------------------------------------------------------- persistence

/// The chosen language survives a restart, so the switcher is not a one-shot.
pub fn load() -> Lang {
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| Lang::from_code(&s))
        .unwrap_or_default()
}

pub fn save(lang: Lang) {
    let Some(p) = config_path() else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(p, lang.code());
}

fn config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let p = std::env::var_os("APPDATA")
        .map(|d| PathBuf::from(d).join("101Craft").join("language"));
    #[cfg(target_os = "macos")]
    let p = std::env::var_os("HOME").map(|h| {
        PathBuf::from(h)
            .join("Library")
            .join("Application Support")
            .join("101Craft")
            .join("language")
    });
    #[cfg(all(unix, not(target_os = "macos")))]
    let p = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .map(|b| b.join("101Craft").join("language"));
    #[cfg(not(any(windows, target_os = "macos", unix)))]
    let p = None;
    p
}

// ---------------------------------------------------------------- English

fn en(key: &str) -> Option<&'static str> {
    Some(match key {
        // modifiers and shortcuts
        "key.ctrl" => "Ctrl",
        "key.shift" => "Shift",
        "sc.new" => "{ctrl}+N",
        "sc.open" => "{ctrl}+O",
        "sc.save" => "{ctrl}+S",
        "sc.saveas" => "{ctrl}+{shift}+S",
        "sc.close" => "{ctrl}+W",
        "sc.undo" => "{ctrl}+Z",
        "sc.redo" => "{ctrl}+Y",
        "sc.copy" => "{ctrl}+C",
        "sc.selectall" => "{ctrl}+A",
        "sc.delete" => "Del",
        "sc.find" => "{ctrl}+F",
        "sc.replace" => "{ctrl}+H",
        "sc.findnext" => "F3",
        "sc.findprev" => "{shift}+F3",
        "sc.goto" => "{ctrl}+G",
        "sc.bookmark" => "{ctrl}+B",
        "sc.nextbm" => "F2",
        "sc.prevbm" => "{shift}+F2",
        "sc.insert" => "Insert",
        "sc.nextfile" => "{ctrl}+Tab",

        // menus
        "menu.file" => "File",
        "menu.new" => "New",
        "menu.open" => "Open…",
        "menu.save" => "Save",
        "menu.saveas" => "Save as…",
        "menu.close" => "Close",
        "menu.quit" => "Quit",
        "menu.edit" => "Edit",
        "menu.undo" => "Undo",
        "menu.redo" => "Redo",
        "menu.copyhex" => "Copy as hex",
        "menu.copytext" => "Copy as text",
        "menu.copyc" => "Copy as C array",
        "menu.pastehint" => "Paste: Ctrl+V in the hex view",
        "menu.selectall" => "Select all",
        "menu.deleteselection" => "Delete selection",
        "menu.insertbytes" => "Insert bytes…",
        "menu.fillselection" => "Fill selection…",
        "menu.mode.insert" => "Switch to insert mode",
        "menu.mode.overwrite" => "Switch to overwrite mode",
        "menu.search" => "Search",
        "menu.find" => "Find…",
        "menu.replace" => "Replace…",
        "menu.findnext" => "Find next",
        "menu.findprev" => "Find backwards",
        "menu.goto" => "Go to address…",
        "menu.bookmark.toggle" => "Toggle bookmark",
        "menu.bookmark.next" => "Next bookmark",
        "menu.bookmark.prev" => "Previous bookmark",
        "menu.view" => "View",
        "menu.textmode" => "Edit as text",
        "menu.bytesperrow" => "Bytes per row",
        "menu.byteorder" => "Byte order",
        "menu.le" => "Little endian (Intel)",
        "menu.be" => "Big endian (Motorola)",
        "menu.fontsize" => "Font size",
        "menu.dark" => "Dark theme",
        "menu.templates" => "Templates",
        "menu.autoapply" => "Apply automatically on open",
        "menu.rerun" => "Re-run template",
        "menu.apply" => "Apply {name}",
        "menu.editor" => "Template editor…",
        "menu.runfile" => "Run template from file…",
        "menu.clearresult" => "Remove template result",
        "menu.tools" => "Tools",
        "menu.checksum" => "Checksum / Hash…",
        "menu.compare" => "Compare files…",
        "menu.histogram" => "Histogram",
        "menu.endcompare" => "End comparison",
        "menu.help" => "Help",
        "menu.keys" => "Keyboard shortcuts",
        "menu.about" => "About 101Craft",
        "menu.language" => "Language",

        // toolbar
        "tb.new" => "New file ({sc})",
        "tb.open" => "Open ({sc})",
        "tb.save" => "Save ({sc})",
        "tb.undo" => "Undo ({sc})",
        "tb.redo" => "Redo ({sc})",
        "tb.find" => "Find ({sc})",
        "tb.replace" => "Replace ({sc})",
        "tb.goto" => "Go to address ({sc})",
        "tb.bookmark" => "Set bookmark ({sc})",
        "tb.template" => "Run template (F5)",
        "tb.tpleditor" => "Template editor",
        "tb.checksum" => "Checksum / Hash",
        "tb.compare" => "Compare files",
        "tb.histogram" => "Histogram",
        "tb.bytesperrow" => "Bytes/row",
        "tb.endian" => "Endian",
        "tb.mode.insert" => "INS",
        "tb.mode.overwrite" => "OVR",
        "tb.modetip" => "Toggle insert/overwrite mode ({sc})",
        "tab.unsaved" => "Not saved",
        "tab.close" => "Close",

        // find bar
        "find.label" => "Find:",
        "find.hint" => "Text or hex, e.g. 42 4D",
        "find.ignorecase" => "Ignore case",
        "find.prevtip" => "Backwards ({sc})",
        "find.nexttip" => "Find next (F3)",
        "find.all" => "Find all",
        "find.next" => "Next ▶",
        "find.close" => "Close",
        "find.replace.toggle" => "Replace…",
        "find.replace.label" => "Replace:",
        "find.replace.one" => "Replace",
        "find.replace.all" => "Replace all",

        // status bar
        "status.nofile" => "No file open",
        "status.pos" => "Pos:",
        "status.value" => "Value:",
        "status.selection" => "Selection: 0x{s}–0x{e} ({n} bytes)",
        "status.size" => "Size:",
        "status.field" => "Field:",
        "status.modified" => "modified",
        "status.column.hex" => "Hex",
        "status.column.text" => "Text",

        // central view
        "central.empty" => "No file open. Drop a file here or open one.",
        "central.open" => "📂 Open file…",
        "central.new" => "📄 New empty file",
        "compare.a" => "A:",
        "compare.b" => "B:",
        "text.toobig" => "The file is too large for the text view (over 8 MB).",
        "text.invalid" => "The file contains bytes that are not valid UTF-8 – the text view is read-only.",

        // left panel
        "panel.openfiles" => "Open files",
        "panel.templates" => "Templates",
        "panel.customtemplate" => "✏ Custom template…",
        "panel.file" => "File",
        "panel.size" => "Size",
        "panel.bookmarks" => "Bookmarks",
        "panel.template" => "Template",
        "panel.extensions" => "Extensions: {exts}",

        // right panel
        "panel.nofile" => "No file",
        "panel.inspector" => "Inspector",
        "panel.checksum" => "Checksum",
        "panel.from" => "From 0x{pos}",
        "panel.inspector.edit" => "Double-click to edit",
        "panel.inspector.hint" => "Values from the cursor position. Double-click a value to change it.",
        "panel.bm.add" => "🔖 Bookmark at cursor/selection",
        "panel.bm.empty" => "No bookmarks yet. Select bytes and press {ctrl}+B.",
        "panel.bm.name" => "Name",
        "panel.bm.address" => "Address",
        "panel.bm.length" => "Length",
        "panel.bm.remove" => "Remove",
        "panel.cs.whole" => "Whole file",
        "panel.cs.selection" => "Selection",
        "panel.cs.compute" => "# Compute checksums",
        "panel.cs.done" => "Checksums computed over {size}.",
        "panel.cs.copy" => "Copy",
        "panel.cs.copied" => "Copied to clipboard",

        // bottom panel
        "panel.bottom.template" => "Template result",
        "panel.bottom.search" => "Search results",
        "panel.bottom.search.n" => "Search results ({n})",
        "panel.bottom.compare" => "Compare",
        "panel.bottom.compare.n" => "Compare ({n})",
        "panel.bottom.histogram" => "Histogram",
        "panel.bottom.output" => "Output",
        "panel.tpl.empty" => "No template active. A template splits the file into named fields.",
        "panel.tpl.name" => "Template: {name}",
        "panel.tpl.ok" => "✔ no errors",
        "panel.tpl.col.name" => "Name",
        "panel.tpl.col.value" => "Value",
        "panel.tpl.col.start" => "Start",
        "panel.tpl.col.size" => "Size",
        "panel.tpl.col.type" => "Type",
        "panel.tpl.col.comment" => "Comment",
        "panel.tpl.edit" => "Double-click the value to change it",
        "panel.tpl.set" => "Field {path} set to {value}.",
        "panel.err.type" => "'{text}' does not fit this type.",
        "panel.search.empty" => "No search results. {ctrl}+F opens the search, “Find all” lists all hits here.",
        "panel.search.bytes" => "{n} bytes",
        "panel.cmp.empty" => "No comparison active.",
        "panel.cmp.open" => "⚖ Compare files…",
        "panel.cmp.identical" => "{a} and {b} are identical.",
        "panel.cmp.count" => "{n} differences between {a} (A) and {b} (B)",
        "panel.cmp.side" => "Side by side",
        "panel.cmp.end" => "End comparison",
        "panel.cmp.changed" => "{n} changed",
        "panel.cmp.onlyb" => "{n} only in B",
        "panel.cmp.onlya" => "{n} only in A",
        "panel.hist.count" => "{n} bytes, {d} distinct values",
        "panel.hist.entropy" => "Entropy: {e} bits/byte",
        "panel.hist.hint" => "Close to 8 = random/compressed/encrypted, low = lots of repetition",
        "panel.hist.byte" => "Byte 0x{b}{ch}: {c}× ({p} %)",
        "panel.out.clear" => "Clear",

        // dialogs
        "dlg.goto.title" => "Go to address",
        "dlg.goto.hint" => "0x1A0 or 1A0h (hex), 416 (decimal), +16 / -16 (relative)",
        "dlg.goto.go" => "Jump",
        "dlg.goto.invalid" => "Invalid address",
        "dlg.cmp.title" => "Compare files",
        "dlg.cmp.need2" => "Please open two files first.",
        "dlg.cmp.openmore" => "📂 Open another file…",
        "dlg.cmp.filea" => "File A:",
        "dlg.cmp.fileb" => "File B:",
        "dlg.cmp.side" => "Show side by side",
        "dlg.cmp.note" => "Byte-wise comparison at the same position.",
        "dlg.cmp.run" => "Compare",
        "dlg.insert.title" => "Insert bytes",
        "dlg.insert.count" => "Count:",
        "dlg.insert.value" => "Value (hex):",
        "dlg.insert.apply" => "Insert",
        "dlg.fill.title" => "Fill selection",
        "dlg.fill.hint" => "Pattern (hex), repeated across the selection:",
        "dlg.fill.apply" => "Fill",
        "dlg.cancel" => "Cancel",
        "dlg.invalid" => "Invalid input",
        "dlg.invalidpattern" => "Invalid pattern",
        "dlg.close.title" => "Unsaved changes",
        "dlg.close.msg" => "“{name}” has been changed. Save changes?",
        "dlg.close.save" => "Save",
        "dlg.close.discard" => "Don’t save",
        "dlg.quit.title" => "Quit 101Craft",
        "dlg.quit.msg" => "{n} file(s) have unsaved changes. Quit anyway?",
        "dlg.quit.yes" => "Quit without saving",
        "dlg.about.title" => "About 101Craft",
        "dlg.about.tag" => "Hex and binary editor with templates.",
        "dlg.about.version" => "Version {v}",
        "dlg.about.body" => {
            "View and edit bytes, split file formats with templates, search, compare and compute checksums."
        }
        "dlg.keys.title" => "Keyboard shortcuts",
        "keys.open" => "Open",
        "keys.save" => "Save",
        "keys.undo" => "Undo",
        "keys.redo" => "Redo",
        "keys.copy" => "Copy",
        "keys.cut" => "Cut",
        "keys.paste" => "Paste",
        "keys.find" => "Find",
        "keys.replace" => "Replace",
        "keys.next" => "Find next",
        "keys.prev" => "Find backwards",
        "keys.bookmark" => "Set bookmark",
        "keys.nextbm" => "Next bookmark",
        "keys.arrows" => "arrows",
        "keys.open_save" => "{a} / {b}",
        "keys.undo_redo" => "{a} / {b}",
        "keys.clipboard" => "{a} / {b} / {c}",
        "keys.selectall" => "Select all",
        "keys.find_replace" => "{a} / {b}",
        "keys.next_prev" => "{a} / {b}",
        "keys.goto" => "Go to address",
        "keys.bookmark_next" => "{a} / {b}",
        "keys.run" => "Run template",
        "keys.insert" => "Insert / overwrite",
        "keys.tab" => "Switch between hex and text column",
        "keys.shiftarrows" => "Extend selection",
        "keys.nextfile" => "Next file",
        "dlg.tpled.title" => "Template editor",
        "dlg.tpled.name" => "Name:",
        "dlg.tpled.name.default" => "New template",
        "dlg.tpled.loadexample" => "Load example…",
        "dlg.tpled.run" => "▶ Run on current file",
        "dlg.tpled.load" => "📂 Load…",
        "dlg.tpled.save" => "💾 Save…",
        "dlg.tpled.hint" => {
            "C-like language: struct, typedef, enum, if/while/for, local variables, FTell(), FSeek(), ReadUInt(), Printf() …"
        }
        "dlg.tpled.saved" => "Template saved: {path}",
        "dlg.tpled.savefail" => "Save failed: {e}",
        "dlg.tpled.filter" => "Templates",
        "dlg.runfilter" => "Templates",

        // app messages
        "msg.ready" => "101Craft ready.",
        "msg.sample" => "Sample file “{name}” opened – use File › Open to load your own files.",
        "msg.filefallback" => "File",
        "msg.untitled" => "Untitled {n}",
        "msg.opened" => "Opened: {name} ({size})",
        "msg.openfail" => "Could not open {path}: {e}",
        "msg.open" => "Open file",
        "msg.saveas" => "Save as",
        "msg.saved" => "Saved: {path}",
        "msg.savefail" => "Save failed: {e}",
        "msg.tpl.run" => "Template “{name}” run on {doc} ({n} fields, {ms} ms).",
        "msg.tpl.error" => "Template error: {e}",
        "msg.tpl.runfile" => "Run template",
        "msg.tpl.readfail" => "Could not read template: {e}",
        "msg.tpl.nomatch" => "No matching template found – pick one in the Templates menu.",
        "msg.find.empty" => "Please enter a search term.",
        "msg.find.badinput" => "Invalid input for this search type.",
        "msg.find.found" => "Found at 0x{p}",
        "msg.find.notfound" => "Not found.",
        "msg.find.hits" => "{n} hits",
        "msg.find.log" => "Search for “{q}”: {n} hits",
        "msg.replace.invalid" => "Invalid replacement.",
        "msg.replace.done" => "{n} occurrences replaced",
        "msg.cmp.need2" => "Please choose two different files to compare.",
        "msg.cmp.done" => "Comparison {a} ↔ {b}: {n} differences, {size} differing",
        "msg.shot.savefail" => "Screenshot could not be saved: {e}",
        "msg.shot.badsize" => "Screenshot: invalid image size",
        "msg.shot.timeout" => "Screenshot did not arrive – aborting.",
        "msg.bookmark" => "Bookmark {n}",
        "msg.signature" => "Signature “BM”",

        // inspector
        "insp.i8" => "int8 (signed byte)",
        "insp.u8" => "uint8 (byte)",
        "insp.i16" => "int16 (short)",
        "insp.u16" => "uint16 (ushort)",
        "insp.i32" => "int32 (int)",
        "insp.u32" => "uint32 (uint)",
        "insp.i64" => "int64",
        "insp.u64" => "uint64",
        "insp.f32" => "float",
        "insp.f64" => "double",
        "insp.bin" => "Binary",
        "insp.char" => "Character (ASCII)",
        "insp.utf16" => "Character (UTF-16)",
        "insp.dosdate" => "DOS date",
        "insp.dostime" => "DOS time",
        "insp.unixtime" => "Unix time (time_t)",
        "insp.filetime" => "Windows FILETIME",

        // search kinds
        "search.text" => "Text (ASCII/UTF-8)",
        "search.utf16" => "Text (UTF-16)",
        "search.hex" => "Hex bytes",
        "search.int32" => "Integer (32-bit)",

        // checksums
        "checksum.crc32" => "CRC-32",
        "checksum.adler32" => "Adler-32",
        "checksum.sum8" => "Sum (8-bit)",
        "checksum.sum16" => "Sum (16-bit)",
        "checksum.sum32" => "Sum (32-bit)",
        "checksum.xor8" => "XOR (8-bit)",
        "checksum.md5" => "MD5",
        "checksum.sha1" => "SHA-1",
        "checksum.sha256" => "SHA-256",

        "unit.bytes" => "{n} bytes",

        // template engine
        "err.line.word" => "Line",
        "err.line" => "Line {line}: {m}",
        "err.abort.word" => "Aborted",
        "err.toomanysteps" => "Aborted: too many steps (infinite loop?)",
        "err.toomanyfields" => "Aborted: too many fields",
        "err.eof_read" => "End of file reached while reading at 0x{at}",
        "err.eof" => "End of file reached",
        "err.eof_in" => "End of file reached in “{name}” at element {i}",
        "err.eof_need" => "End of file reached: “{name}” needs {total} bytes from 0x{pos}",
        "err.unknownvar" => "Unknown variable “{n}” (declare it with “local”)",
        "err.unknownvar2" => "Unknown variable “{n}”",
        "err.unknownname" => "Unknown name “{n}”",
        "err.fieldnotfound" => "Field not found",
        "err.indexout" => "Index {idx} outside of {name}",
        "err.negsize" => "Line {line}: negative field size for “{name}”",
        "err.unknowntype" => "Unknown type “{ty}”",
        "err.fieldbig" => "Field too large",
        "err.elems" => "[{n} elements]",
        "err.optext" => "Operator {op} not valid for text",
        "err.div0" => "Division by zero",
        "err.unknownop" => "Unknown operator {op}",
        "err.syntax" => "Syntax error: {e}",
        "err.fnarg.sizeof" => "sizeof: missing argument",
        "err.unknown.sizeof" => "sizeof: unknown",
        "err.fnarg.startof" => "startof: missing argument",
        "err.unknown.startof" => "startof: unknown",
        "err.fnarg.exists" => "exists: missing argument",
        "err.unknownfunc" => "Unknown function “{name}”",
        "err.warn" => "Warning: {s}",
        "err.incr" => "Line {line}: ++/-- only on variables",
        "err.dowhile" => "Line {line}: “while” expected",
        "err.unexpected_end" => "Unexpected end: “}” missing",
        "err.bracket_missing" => "“]” missing",
        "err.gt_missing" => "“>” missing",
        "err.badnum" => "Line {line}: invalid number",
        "err.badchar" => "Line {line}: unexpected character “{c}”",
        "err.expected" => "Line {line}: expected “{p}”, found {t}",
        "err.expected_name" => "Line {line}: expected a name, found {t}",
        "err.unexpected" => "Line {line}: unexpected {t}",
        "err.eof.tok" => "end of file",

        // built-in template names
        "tpl.bmp" => "BMP image",
        "tpl.png" => "PNG image",
        "tpl.gif" => "GIF image",
        "tpl.wav" => "WAV audio",
        "tpl.zip" => "ZIP archive",
        "tpl.exe" => "EXE/DLL (PE)",

        // demo content
        "scene.sample.bmp" => "sample.bmp",
        "scene.sample.newbmp" => "sample-new.bmp",
        "scene.sample.png" => "image.png",
        "scene.bm.pixels" => "Pixel data",
        "scene.bm.width" => "Image width",
        "scene.tpl.bmp" => "BMP image",
        "scene.png.text" => "Created with 101Craft",

        // misc
        "main.unknownscene" => "Unknown scene “{s}”. Available: {list}",
        _ => return None,
    })
}

// ------------------------------------------------------------------ German

fn de(key: &str) -> Option<&'static str> {
    Some(match key {
        "key.ctrl" => "Strg",
        "key.shift" => "Umschalt",
        "sc.new" => "{ctrl}+N",
        "sc.open" => "{ctrl}+O",
        "sc.save" => "{ctrl}+S",
        "sc.saveas" => "{ctrl}+{shift}+S",
        "sc.close" => "{ctrl}+W",
        "sc.undo" => "{ctrl}+Z",
        "sc.redo" => "{ctrl}+Y",
        "sc.copy" => "{ctrl}+C",
        "sc.selectall" => "{ctrl}+A",
        "sc.delete" => "Entf",
        "sc.find" => "{ctrl}+F",
        "sc.replace" => "{ctrl}+H",
        "sc.findnext" => "F3",
        "sc.findprev" => "{shift}+F3",
        "sc.goto" => "{ctrl}+G",
        "sc.bookmark" => "{ctrl}+B",
        "sc.nextbm" => "F2",
        "sc.prevbm" => "{shift}+F2",
        "sc.insert" => "Einfg",
        "sc.nextfile" => "{ctrl}+Tab",

        "menu.file" => "Datei",
        "menu.new" => "Neu",
        "menu.open" => "Öffnen…",
        "menu.save" => "Speichern",
        "menu.saveas" => "Speichern unter…",
        "menu.close" => "Schließen",
        "menu.quit" => "Beenden",
        "menu.edit" => "Bearbeiten",
        "menu.undo" => "Rückgängig",
        "menu.redo" => "Wiederholen",
        "menu.copyhex" => "Als Hex kopieren",
        "menu.copytext" => "Als Text kopieren",
        "menu.copyc" => "Als C-Array kopieren",
        "menu.pastehint" => "Einfügen: Strg+V in der Hex-Ansicht",
        "menu.selectall" => "Alles auswählen",
        "menu.deleteselection" => "Auswahl löschen",
        "menu.insertbytes" => "Bytes einfügen…",
        "menu.fillselection" => "Auswahl füllen…",
        "menu.mode.insert" => "Zum Einfügemodus wechseln",
        "menu.mode.overwrite" => "Zum Überschreibmodus wechseln",
        "menu.search" => "Suchen",
        "menu.find" => "Suchen…",
        "menu.replace" => "Ersetzen…",
        "menu.findnext" => "Weitersuchen",
        "menu.findprev" => "Rückwärts suchen",
        "menu.goto" => "Gehe zu Adresse…",
        "menu.bookmark.toggle" => "Lesezeichen setzen/entfernen",
        "menu.bookmark.next" => "Nächstes Lesezeichen",
        "menu.bookmark.prev" => "Vorheriges Lesezeichen",
        "menu.view" => "Ansicht",
        "menu.textmode" => "Als Text bearbeiten",
        "menu.bytesperrow" => "Bytes pro Zeile",
        "menu.byteorder" => "Byte-Reihenfolge",
        "menu.le" => "Little Endian (Intel)",
        "menu.be" => "Big Endian (Motorola)",
        "menu.fontsize" => "Schriftgröße",
        "menu.dark" => "Dunkles Design",
        "menu.templates" => "Vorlagen",
        "menu.autoapply" => "Beim Öffnen automatisch anwenden",
        "menu.rerun" => "Vorlage erneut ausführen",
        "menu.apply" => "{name} anwenden",
        "menu.editor" => "Vorlagen-Editor…",
        "menu.runfile" => "Vorlage aus Datei ausführen…",
        "menu.clearresult" => "Vorlagenergebnis entfernen",
        "menu.tools" => "Werkzeuge",
        "menu.checksum" => "Prüfsumme / Hash…",
        "menu.compare" => "Dateien vergleichen…",
        "menu.histogram" => "Histogramm",
        "menu.endcompare" => "Vergleich beenden",
        "menu.help" => "Hilfe",
        "menu.keys" => "Tastenkürzel",
        "menu.about" => "Über 101Craft",
        "menu.language" => "Sprache",

        "tb.new" => "Neue Datei ({sc})",
        "tb.open" => "Öffnen ({sc})",
        "tb.save" => "Speichern ({sc})",
        "tb.undo" => "Rückgängig ({sc})",
        "tb.redo" => "Wiederholen ({sc})",
        "tb.find" => "Suchen ({sc})",
        "tb.replace" => "Ersetzen ({sc})",
        "tb.goto" => "Gehe zu Adresse ({sc})",
        "tb.bookmark" => "Lesezeichen setzen ({sc})",
        "tb.template" => "Vorlage ausführen (F5)",
        "tb.tpleditor" => "Vorlagen-Editor",
        "tb.checksum" => "Prüfsumme / Hash",
        "tb.compare" => "Dateien vergleichen",
        "tb.histogram" => "Histogramm",
        "tb.bytesperrow" => "Bytes/Zeile",
        "tb.endian" => "Endian",
        "tb.mode.insert" => "EINF",
        "tb.mode.overwrite" => "ÜBR",
        "tb.modetip" => "Einfüge-/Überschreibmodus umschalten ({sc})",
        "tab.unsaved" => "Nicht gespeichert",
        "tab.close" => "Schließen",

        "find.label" => "Suchen:",
        "find.hint" => "Text oder Hex, z. B. 42 4D",
        "find.ignorecase" => "Groß/klein egal",
        "find.prevtip" => "Rückwärts ({sc})",
        "find.nexttip" => "Weitersuchen (F3)",
        "find.all" => "Alle finden",
        "find.next" => "Weiter ▶",
        "find.close" => "Schließen",
        "find.replace.toggle" => "Ersetzen…",
        "find.replace.label" => "Ersetzen:",
        "find.replace.one" => "Ersetzen",
        "find.replace.all" => "Alle ersetzen",

        "status.nofile" => "Keine Datei geöffnet",
        "status.pos" => "Pos:",
        "status.value" => "Wert:",
        "status.selection" => "Auswahl: 0x{s}–0x{e} ({n} Bytes)",
        "status.size" => "Größe:",
        "status.field" => "Feld:",
        "status.modified" => "geändert",
        "status.column.hex" => "Hex",
        "status.column.text" => "Text",

        "central.empty" => "Keine Datei geöffnet. Datei hierher ziehen oder öffnen.",
        "central.open" => "📂 Datei öffnen…",
        "central.new" => "📄 Neue leere Datei",
        "compare.a" => "A:",
        "compare.b" => "B:",
        "text.toobig" => "Die Datei ist für die Textansicht zu groß (über 8 MB).",
        "text.invalid" => {
            "Die Datei enthält Bytes, die kein gültiger UTF-8-Text sind – die Textansicht ist schreibgeschützt."
        }

        "panel.openfiles" => "Geöffnete Dateien",
        "panel.templates" => "Vorlagen",
        "panel.customtemplate" => "✏ Eigene Vorlage…",
        "panel.file" => "Datei",
        "panel.size" => "Größe",
        "panel.bookmarks" => "Lesezeichen",
        "panel.template" => "Vorlage",
        "panel.extensions" => "Endungen: {exts}",

        "panel.nofile" => "Keine Datei",
        "panel.inspector" => "Inspektor",
        "panel.checksum" => "Prüfsumme",
        "panel.from" => "Ab 0x{pos}",
        "panel.inspector.edit" => "Doppelklick zum Bearbeiten",
        "panel.inspector.hint" => {
            "Werte ab der Cursorposition. Doppelklick auf einen Wert, um ihn zu ändern."
        }
        "panel.bm.add" => "🔖 Lesezeichen an Cursor/Auswahl",
        "panel.bm.empty" => "Noch keine Lesezeichen. Bytes markieren und {ctrl}+B drücken.",
        "panel.bm.name" => "Name",
        "panel.bm.address" => "Adresse",
        "panel.bm.length" => "Länge",
        "panel.bm.remove" => "Entfernen",
        "panel.cs.whole" => "Ganze Datei",
        "panel.cs.selection" => "Auswahl",
        "panel.cs.compute" => "# Prüfsummen berechnen",
        "panel.cs.done" => "Prüfsummen über {size} berechnet.",
        "panel.cs.copy" => "Kopieren",
        "panel.cs.copied" => "In die Zwischenablage kopiert",

        "panel.bottom.template" => "Vorlagenergebnis",
        "panel.bottom.search" => "Suchergebnisse",
        "panel.bottom.search.n" => "Suchergebnisse ({n})",
        "panel.bottom.compare" => "Vergleich",
        "panel.bottom.compare.n" => "Vergleich ({n})",
        "panel.bottom.histogram" => "Histogramm",
        "panel.bottom.output" => "Ausgabe",
        "panel.tpl.empty" => "Keine Vorlage aktiv. Eine Vorlage zerlegt die Datei in benannte Felder.",
        "panel.tpl.name" => "Vorlage: {name}",
        "panel.tpl.ok" => "✔ ohne Fehler",
        "panel.tpl.col.name" => "Name",
        "panel.tpl.col.value" => "Wert",
        "panel.tpl.col.start" => "Start",
        "panel.tpl.col.size" => "Größe",
        "panel.tpl.col.type" => "Typ",
        "panel.tpl.col.comment" => "Kommentar",
        "panel.tpl.edit" => "Doppelklick auf den Wert zum Ändern",
        "panel.tpl.set" => "Feld {path} auf {value} gesetzt.",
        "panel.err.type" => "“{text}” passt nicht in diesen Typ.",
        "panel.search.empty" => {
            "Keine Suchergebnisse. {ctrl}+F öffnet die Suche, „Alle finden“ listet alle Treffer hier."
        }
        "panel.search.bytes" => "{n} Bytes",
        "panel.cmp.empty" => "Kein Vergleich aktiv.",
        "panel.cmp.open" => "⚖ Dateien vergleichen…",
        "panel.cmp.identical" => "{a} und {b} sind identisch.",
        "panel.cmp.count" => "{n} Unterschiede zwischen {a} (A) und {b} (B)",
        "panel.cmp.side" => "Nebeneinander",
        "panel.cmp.end" => "Vergleich beenden",
        "panel.cmp.changed" => "{n} geändert",
        "panel.cmp.onlyb" => "{n} nur in B",
        "panel.cmp.onlya" => "{n} nur in A",
        "panel.hist.count" => "{n} Bytes, {d} verschiedene Werte",
        "panel.hist.entropy" => "Entropie: {e} Bit/Byte",
        "panel.hist.hint" => "Nahe 8 = zufällig/komprimiert/verschlüsselt, niedrig = viel Wiederholung",
        "panel.hist.byte" => "Byte 0x{b}{ch}: {c}× ({p} %)",
        "panel.out.clear" => "Leeren",

        "dlg.goto.title" => "Gehe zu Adresse",
        "dlg.goto.hint" => "0x1A0 oder 1A0h (hex), 416 (dezimal), +16 / -16 (relativ)",
        "dlg.goto.go" => "Springen",
        "dlg.goto.invalid" => "Ungültige Adresse",
        "dlg.cmp.title" => "Dateien vergleichen",
        "dlg.cmp.need2" => "Bitte zuerst zwei Dateien öffnen.",
        "dlg.cmp.openmore" => "📂 Weitere Datei öffnen…",
        "dlg.cmp.filea" => "Datei A:",
        "dlg.cmp.fileb" => "Datei B:",
        "dlg.cmp.side" => "Nebeneinander anzeigen",
        "dlg.cmp.note" => "Byteweiser Vergleich an gleicher Position.",
        "dlg.cmp.run" => "Vergleichen",
        "dlg.insert.title" => "Bytes einfügen",
        "dlg.insert.count" => "Anzahl:",
        "dlg.insert.value" => "Wert (Hex):",
        "dlg.insert.apply" => "Einfügen",
        "dlg.fill.title" => "Auswahl füllen",
        "dlg.fill.hint" => "Muster (Hex), wird über die Auswahl wiederholt:",
        "dlg.fill.apply" => "Füllen",
        "dlg.cancel" => "Abbrechen",
        "dlg.invalid" => "Ungültige Eingabe",
        "dlg.invalidpattern" => "Ungültiges Muster",
        "dlg.close.title" => "Ungespeicherte Änderungen",
        "dlg.close.msg" => "“{name}” wurde geändert. Änderungen speichern?",
        "dlg.close.save" => "Speichern",
        "dlg.close.discard" => "Nicht speichern",
        "dlg.quit.title" => "101Craft beenden",
        "dlg.quit.msg" => "{n} Datei(en) haben ungespeicherte Änderungen. Trotzdem beenden?",
        "dlg.quit.yes" => "Beenden ohne Speichern",
        "dlg.about.title" => "Über 101Craft",
        "dlg.about.tag" => "Hex- und Binär-Editor mit Vorlagen.",
        "dlg.about.version" => "Version {v}",
        "dlg.about.body" => {
            "Bytes ansehen und bearbeiten, Dateiformate mit Vorlagen zerlegen, suchen, vergleichen und Prüfsummen berechnen."
        }
        "dlg.keys.title" => "Tastenkürzel",
        "keys.open" => "Öffnen",
        "keys.save" => "Speichern",
        "keys.undo" => "Rückgängig",
        "keys.redo" => "Wiederholen",
        "keys.copy" => "Kopieren",
        "keys.cut" => "Ausschneiden",
        "keys.paste" => "Einfügen",
        "keys.find" => "Suchen",
        "keys.replace" => "Ersetzen",
        "keys.next" => "Weitersuchen",
        "keys.prev" => "Rückwärts suchen",
        "keys.bookmark" => "Lesezeichen setzen",
        "keys.nextbm" => "Nächstes Lesezeichen",
        "keys.arrows" => "Pfeile",
        "keys.open_save" => "{a} / {b}",
        "keys.undo_redo" => "{a} / {b}",
        "keys.clipboard" => "{a} / {b} / {c}",
        "keys.selectall" => "Alles auswählen",
        "keys.find_replace" => "{a} / {b}",
        "keys.next_prev" => "{a} / {b}",
        "keys.goto" => "Gehe zu Adresse",
        "keys.bookmark_next" => "{a} / {b}",
        "keys.run" => "Vorlage ausführen",
        "keys.insert" => "Einfügen / Überschreiben",
        "keys.tab" => "Zwischen Hex- und Textspalte wechseln",
        "keys.shiftarrows" => "Auswahl erweitern",
        "keys.nextfile" => "Nächste Datei",
        "dlg.tpled.title" => "Vorlagen-Editor",
        "dlg.tpled.name" => "Name:",
        "dlg.tpled.name.default" => "Neue Vorlage",
        "dlg.tpled.loadexample" => "Beispiel laden…",
        "dlg.tpled.run" => "▶ Auf aktuelle Datei ausführen",
        "dlg.tpled.load" => "📂 Laden…",
        "dlg.tpled.save" => "💾 Speichern…",
        "dlg.tpled.hint" => {
            "C-ähnliche Sprache: struct, typedef, enum, if/while/for, local-Variablen, FTell(), FSeek(), ReadUInt(), Printf() …"
        }
        "dlg.tpled.saved" => "Vorlage gespeichert: {path}",
        "dlg.tpled.savefail" => "Speichern fehlgeschlagen: {e}",
        "dlg.tpled.filter" => "Vorlagen",
        "dlg.runfilter" => "Vorlagen",

        "msg.ready" => "101Craft bereit.",
        "msg.sample" => {
            "Beispieldatei „{name}“ geöffnet – mit Datei › Öffnen eigene Dateien laden."
        }
        "msg.filefallback" => "Datei",
        "msg.untitled" => "Unbenannt {n}",
        "msg.opened" => "Geöffnet: {name} ({size})",
        "msg.openfail" => "Konnte {path} nicht öffnen: {e}",
        "msg.open" => "Datei öffnen",
        "msg.saveas" => "Speichern unter",
        "msg.saved" => "Gespeichert: {path}",
        "msg.savefail" => "Speichern fehlgeschlagen: {e}",
        "msg.tpl.run" => "Vorlage „{name}“ auf {doc} ausgeführt ({n} Felder, {ms} ms).",
        "msg.tpl.error" => "Vorlagenfehler: {e}",
        "msg.tpl.runfile" => "Vorlage ausführen",
        "msg.tpl.readfail" => "Vorlage konnte nicht gelesen werden: {e}",
        "msg.tpl.nomatch" => "Keine passende Vorlage gefunden – im Menü Vorlagen eine auswählen.",
        "msg.find.empty" => "Bitte Suchbegriff eingeben.",
        "msg.find.badinput" => "Ungültige Eingabe für diese Suchart.",
        "msg.find.found" => "Gefunden bei 0x{p}",
        "msg.find.notfound" => "Nicht gefunden.",
        "msg.find.hits" => "{n} Treffer",
        "msg.find.log" => "Suche nach „{q}“: {n} Treffer",
        "msg.replace.invalid" => "Ungültiger Ersatz.",
        "msg.replace.done" => "{n} Stellen ersetzt",
        "msg.cmp.need2" => "Bitte zwei verschiedene Dateien zum Vergleichen wählen.",
        "msg.cmp.done" => "Vergleich {a} ↔ {b}: {n} Unterschiede, {size} abweichend",
        "msg.shot.savefail" => "Bildschirmfoto konnte nicht gespeichert werden: {e}",
        "msg.shot.badsize" => "Bildschirmfoto: ungültige Bildgröße",
        "msg.shot.timeout" => "Bildschirmfoto kam nicht an – Abbruch.",
        "msg.bookmark" => "Lesezeichen {n}",
        "msg.signature" => "Kennung „BM“",

        "insp.i8" => "int8 (signed byte)",
        "insp.u8" => "uint8 (byte)",
        "insp.i16" => "int16 (short)",
        "insp.u16" => "uint16 (ushort)",
        "insp.i32" => "int32 (int)",
        "insp.u32" => "uint32 (uint)",
        "insp.i64" => "int64",
        "insp.u64" => "uint64",
        "insp.f32" => "float",
        "insp.f64" => "double",
        "insp.bin" => "Binär",
        "insp.char" => "Zeichen (ASCII)",
        "insp.utf16" => "Zeichen (UTF-16)",
        "insp.dosdate" => "DOS-Datum",
        "insp.dostime" => "DOS-Zeit",
        "insp.unixtime" => "Unix-Zeit (time_t)",
        "insp.filetime" => "Windows FILETIME",

        "search.text" => "Text (ASCII/UTF-8)",
        "search.utf16" => "Text (UTF-16)",
        "search.hex" => "Hex-Bytes",
        "search.int32" => "Ganzzahl (32 Bit)",

        "checksum.crc32" => "CRC-32",
        "checksum.adler32" => "Adler-32",
        "checksum.sum8" => "Summe (8 Bit)",
        "checksum.sum16" => "Summe (16 Bit)",
        "checksum.sum32" => "Summe (32 Bit)",
        "checksum.xor8" => "XOR (8 Bit)",
        "checksum.md5" => "MD5",
        "checksum.sha1" => "SHA-1",
        "checksum.sha256" => "SHA-256",

        "unit.bytes" => "{n} Bytes",

        "err.line.word" => "Zeile",
        "err.line" => "Zeile {line}: {m}",
        "err.abort.word" => "Abbruch",
        "err.toomanysteps" => "Abbruch: zu viele Schritte (Endlosschleife?)",
        "err.toomanyfields" => "Abbruch: zu viele Felder",
        "err.eof_read" => "Dateiende erreicht beim Lesen an 0x{at}",
        "err.eof" => "Dateiende erreicht",
        "err.eof_in" => "Dateiende erreicht in „{name}“ bei Element {i}",
        "err.eof_need" => "Dateiende erreicht: „{name}“ braucht {total} Bytes ab 0x{pos}",
        "err.unknownvar" => "Unbekannte Variable „{n}“ (mit „local“ anlegen)",
        "err.unknownvar2" => "Unbekannte Variable „{n}“",
        "err.unknownname" => "Unbekannter Name „{n}“",
        "err.fieldnotfound" => "Feld nicht gefunden",
        "err.indexout" => "Index {idx} außerhalb von {name}",
        "err.negsize" => "Zeile {line}: negative Feldgröße für „{name}“",
        "err.unknowntype" => "Unbekannter Typ „{ty}“",
        "err.fieldbig" => "Feld zu groß",
        "err.elems" => "[{n} Elemente]",
        "err.optext" => "Operator {op} nicht für Texte",
        "err.div0" => "Division durch 0",
        "err.unknownop" => "Unbekannter Operator {op}",
        "err.syntax" => "Syntaxfehler: {e}",
        "err.fnarg.sizeof" => "sizeof: Argument fehlt",
        "err.unknown.sizeof" => "sizeof: unbekannt",
        "err.fnarg.startof" => "startof: Argument fehlt",
        "err.unknown.startof" => "startof: unbekannt",
        "err.fnarg.exists" => "exists: Argument fehlt",
        "err.unknownfunc" => "Unbekannte Funktion „{name}“",
        "err.warn" => "Warnung: {s}",
        "err.incr" => "Zeile {line}: ++/-- nur bei Variablen",
        "err.dowhile" => "Zeile {line}: „while“ erwartet",
        "err.unexpected_end" => "Unerwartetes Ende: „}“ fehlt",
        "err.bracket_missing" => "„]“ fehlt",
        "err.gt_missing" => "„>“ fehlt",
        "err.badnum" => "Zeile {line}: ungültige Zahl",
        "err.badchar" => "Zeile {line}: unerwartetes Zeichen „{c}“",
        "err.expected" => "Zeile {line}: „{p}“ erwartet, gefunden {t}",
        "err.expected_name" => "Zeile {line}: Name erwartet, gefunden {t}",
        "err.unexpected" => "Zeile {line}: unerwartet {t}",
        "err.eof.tok" => "Dateiende",

        "tpl.bmp" => "BMP-Bild",
        "tpl.png" => "PNG-Bild",
        "tpl.gif" => "GIF-Bild",
        "tpl.wav" => "WAV-Audio",
        "tpl.zip" => "ZIP-Archiv",
        "tpl.exe" => "EXE/DLL (PE)",

        "scene.sample.bmp" => "beispiel.bmp",
        "scene.sample.newbmp" => "beispiel-neu.bmp",
        "scene.sample.png" => "bild.png",
        "scene.bm.pixels" => "Pixeldaten",
        "scene.bm.width" => "Bildbreite",
        "scene.tpl.bmp" => "BMP-Bild",
        "scene.png.text" => "Erstellt mit 101Craft",

        "main.unknownscene" => "Unbekannte Szene „{s}“. Möglich: {list}",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_the_default() {
        assert_eq!(Lang::default(), Lang::En);
        // A missing config file must not fall back to German.
        assert_eq!(Lang::from_code(""), Lang::En);
        assert_eq!(Lang::from_code("Deutsch"), Lang::De);
    }

    #[test]
    fn both_languages_are_wired_up() {
        for key in ["menu.file", "menu.save", "panel.inspector", "tb.undo", "status.size"] {
            assert_ne!(get(Lang::En, key), get(Lang::De, key), "{key} not translated");
            assert!(!get(Lang::En, key).is_empty());
            assert!(!get(Lang::De, key).is_empty());
        }
    }

    #[test]
    fn unknown_keys_pass_through() {
        assert_eq!(get(Lang::En, "free-form text"), "free-form text");
        assert_eq!(get(Lang::De, "free-form text"), "free-form text");
    }

    #[test]
    fn placeholders_are_substituted() {
        assert_eq!(tr!(Lang::En, "msg.bookmark", n = 3), "Bookmark 3");
        assert_eq!(tr!(Lang::De, "msg.bookmark", n = 3), "Lesezeichen 3");
        assert_eq!(
            tr!(Lang::En, "panel.cs.done", size = "4 KiB"),
            "Checksums computed over 4 KiB."
        );
        // A placeholder the string does not use is left alone, not panicked on.
        assert_eq!(tr!(Lang::En, "menu.file", n = 1), "File");
    }
}
