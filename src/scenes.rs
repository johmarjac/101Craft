//! Demo set-ups used with `--scene NAME` (for README screenshots).

use crate::app::{App, BottomTab, RightTab};
use crate::doc::Document;
use crate::i18n::Lang;
use crate::tools::{self, SearchKind};

use crate::tr;

pub const SCENES: &[&str] = &["overview", "dark", "compare", "search", "histogram", "editor"];

/// A small 64 x 48 colourful PNG with a text chunk.
fn sample_png(lang: Lang) -> Vec<u8> {
    let img = image::RgbImage::from_fn(64, 48, |x, y| {
        let r = (x * 4) as u8;
        let g = (y * 5) as u8;
        let b = ((x ^ y) * 4) as u8;
        image::Rgb([r, g, b])
    });
    let mut out = std::io::Cursor::new(Vec::new());
    let _ = img.write_to(&mut out, image::ImageFormat::Png);
    let mut png = out.into_inner();
    // Insert a tEXt chunk right after IHDR (8 + 25 bytes).
    let text = tr!(lang, "scene.png.text");
    let text = format!("Comment\x00{text}");
    let mut chunk = Vec::new();
    chunk.extend_from_slice(&(text.len() as u32).to_be_bytes());
    chunk.extend_from_slice(b"tEXt");
    chunk.extend_from_slice(text.as_bytes());
    let mut h = crc32fast::Hasher::new();
    h.update(b"tEXt");
    h.update(text.as_bytes());
    chunk.extend_from_slice(&h.finalize().to_be_bytes());
    if png.len() > 33 {
        png.splice(33..33, chunk);
    }
    png
}

fn open_sample(app: &mut App, name: String, data: Vec<u8>) -> usize {
    app.docs.push(Document::new(&name, data, None));
    let i = app.docs.len() - 1;
    app.active = i;
    app.auto_run_template(i);
    i
}

pub fn apply(app: &mut App, scene: &str) {
    match scene {
        "dark" => {
            app.dark = true;
            let i = open_sample(app, tr!(app.lang, "scene.sample.png"), sample_png(app.lang));
            app.docs[i].select_range(0x10, 13);
            if let Some(t) = app.docs[i].template.as_mut() {
                for n in ["/chunk[0]", "/chunk[1]"] {
                    t.expanded.insert(n.to_string());
                }
            }
        }
        "compare" => {
            app.docs.clear();
            let a = crate::sample::sample_bmp();
            let mut b = a.clone();
            for p in [0x12usize, 0x40, 0x41, 0x42, 0x90, 0x91, 0x120] {
                b[p] = b[p].wrapping_add(0x37);
            }
            b.extend_from_slice(b"ENDE");
            let ia = open_sample(app, tr!(app.lang, "scene.sample.bmp"), a);
            let ib = open_sample(app, tr!(app.lang, "scene.sample.newbmp"), b);
            app.compare.a = ia;
            app.compare.b = ib;
            app.compare.split = true;
            app.run_compare();
            app.active = ia;
            app.docs[ia].set_cursor(0x40, false);
            app.docs[ib].set_cursor(0x40, false);
        }
        "search" => {
            app.active = 0;
            app.find.open = true;
            app.find.kind = SearchKind::Hex;
            app.find.text = "22 8E".into();
            app.right_tab = RightTab::Bookmarks;
            let d = &mut app.docs[0];
            d.bookmarks.push(crate::doc::Bookmark {
                name: tr!(app.lang, "scene.bm.pixels").into(),
                start: 0x36,
                len: 48,
                color: egui::Color32::from_rgb(150, 220, 255),
            });
            d.bookmarks.push(crate::doc::Bookmark {
                name: tr!(app.lang, "scene.bm.width").into(),
                start: 0x12,
                len: 4,
                color: egui::Color32::from_rgb(190, 240, 160),
            });
            d.bookmarks.sort_by_key(|b| b.start);
            app.find_all();
        }
        "histogram" => {
            let i = open_sample(app, tr!(app.lang, "scene.sample.png"), sample_png(app.lang));
            app.bottom_tab = BottomTab::Histogram;
            app.right_tab = RightTab::Checksum;
            let data = app.docs[i].data.clone();
            app.checksum_results = tools::CHECKSUMS
                .iter()
                .map(|c| (c.0.to_string(), tools::checksum(c.0, &data)))
                .collect();
        }
        "editor" => {
            app.tpl_editor.open = true;
            app.tpl_editor.name = tr!(app.lang, "scene.tpl.bmp").into();
            app.tpl_editor.source = crate::template::BUILTINS[0].source.to_string();
        }
        _ => {}
    }
}
