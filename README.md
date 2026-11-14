<div align="center">

# 101Craft

**The fast hex and binary editor that understands your files.**

View and edit bytes, then split file formats into readable fields with templates —
for Windows, Linux and macOS.

[![Release](https://github.com/johmarjac/101Craft/actions/workflows/release.yml/badge.svg)](https://github.com/johmarjac/101Craft/actions/workflows/release.yml)
[![Latest release](https://img.shields.io/github/v/release/johmarjac/101Craft?label=Download)](https://github.com/johmarjac/101Craft/releases/latest)
![Platforms](https://img.shields.io/badge/Platforms-Windows%20%7C%20Linux%20%7C%20macOS-2d78d7)
![Rust](https://img.shields.io/badge/written%20in-Rust-b7410e)

![101Craft – Overview](docs/screenshots/overview.png)

</div>

---

## Why 101Craft?

A normal hex editor shows you a wall of numbers. **101Craft shows you what they mean.**
Open an image, an archive or a program — 101Craft detects the format, colours every field
and lists it with name, value and position. Click a field to highlight the matching bytes,
double-click a value to change it right in the file.

- ⚡ **Fast and lean** — a single binary, no installation needed (portable build)
- 🧩 **Templates** break file formats into named fields automatically
- 🔍 **Search and replace** by text, UTF-16, hex bytes or numbers
- ⚖️ **Compare files** — differences side by side, highlighted in colour
- 🧮 **Checksums and hashes** — CRC-32 through SHA-256 in one click
- 🌙 **Light or dark** — whichever you prefer

## Highlights

<table>
<tr>
<td width="50%">

**Templates in the dark theme**
A PNG image broken down into signature, chunks, width, height and bit depth.

![Dark theme with PNG template](docs/screenshots/dark.png)

</td>
<td width="50%">

**Comparing files**
Two versions side by side, every difference marked red and listed.

![File comparison](docs/screenshots/compare.png)

</td>
</tr>
<tr>
<td width="50%">

**Search and bookmarks**
All hits at a glance, and remember important spots under your own names.

![Search and bookmarks](docs/screenshots/search.png)

</td>
<td width="50%">

**Histogram and checksums**
Byte distribution with entropy — spots compressed or encrypted data instantly.

![Histogram and checksums](docs/screenshots/histogram.png)

</td>
</tr>
<tr>
<td colspan="2">

**Write your own templates**
A small, C-like language — edit it in the app and run it immediately.

![Template editor](docs/screenshots/editor.png)

</td>
</tr>
</table>

## Download

Ready-made builds are available under **[Releases](https://github.com/johmarjac/101Craft/releases/latest)**:

| System | File | Notes |
|---|---|---|
| Windows x64 | `…-windows-x64-setup.exe` | Installer with Start menu entry |
| Windows x64 | `…-windows-x64-portable.zip` | Unzip and run — no installation, e.g. from a USB stick |
| Windows ARM64 | `…-windows-arm64-setup.exe` / `…-portable.zip` | For Snapdragon and other ARM devices |
| Linux x64 / ARM64 | `…-linux-x64.tar.gz` / `…-linux-arm64.tar.gz` | Unpack, then run `./101Craft` |
| macOS | `…-macos-universal.dmg` | Runs on Apple Silicon and Intel |

> **macOS:** The app is not notarized. On first launch, confirm with right-click › *Open*.

## All features

**Editing**
- Multiple files in tabs, hex and text column (switch with `Tab`)
- Insert and overwrite mode, unlimited undo steps
- Copy as hex, text or C array; paste from hex or text
- Insert bytes, fill a selection with a pattern
- Text view for UTF-8 files, drag & drop to open

**Understanding**
- **Inspector:** values at the cursor as int8 … int64, float, double, binary, ASCII,
  UTF-16, DOS date/time, Unix time and Windows FILETIME — editable via double-click
- **Templates** for BMP, PNG, GIF, WAV, ZIP and EXE/DLL, detected automatically on open
- Coloured highlight for every field, field name of the current byte in the status bar
- Little and big endian switchable

**Finding and checking**
- Find/replace (text, UTF-16, hex, 32-bit number), "find all" with a hit list
- Go to address — hex (`0x1A0`, `1A0h`), decimal or relative (`+16`)
- Bookmarks with names and colours
- Checksums: CRC-32, Adler-32, sum 8/16/32, XOR, MD5, SHA-1, SHA-256
- Side-by-side file comparison with synchronised scrolling
- Histogram with entropy display

## Writing templates yourself

Templates describe a file format in a small, C-like language.
101Craft reads the fields one after another from the file:

```c
LittleEndian();

typedef struct {
    char   magic[4];               // signature
    uint   version;
    uint   count <comment="Number of entries">;
} HEADER;

typedef struct {
    ushort id;
    ushort length;
    uchar  data[length];           // length comes from the field before
} ENTRY;

HEADER header;
ENTRY  entries[header.count];
```

Supported are `struct`, `typedef`, `enum`, arrays with computed length, `if`/`while`/`for`,
`local` variables and functions such as `FTell()`, `FSeek()`, `FileSize()`, `ReadUInt()` and `Printf()`.
The bundled templates in [`templates/`](templates) are good examples.

## Keyboard shortcuts

| Key | Action |
|---|---|
| `Ctrl+O` / `Ctrl+S` | Open / save |
| `Ctrl+Z` / `Ctrl+Y` | Undo / redo |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Copy / cut / paste |
| `Ctrl+F` / `Ctrl+H` | Find / replace |
| `F3` / `Shift+F3` | Find next / previous |
| `Ctrl+G` | Go to address |
| `Ctrl+B` / `F2` | Set bookmark / next bookmark |
| `F5` | Run template |
| `Insert` | Insert / overwrite |
| `Tab` | Switch between hex and text column |
| `Ctrl+Tab` | Next file |

## Building it yourself

You need [Rust](https://rustup.rs).

```sh
git clone https://github.com/johmarjac/101Craft.git
cd 101Craft
cargo run --release -- [files...]
```

On Linux you also need the development packages for X11/Wayland
(e.g. `libxkbcommon-dev libwayland-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev`).

Useful launch options:

- `--screenshot image.png` — starts, renders about 120 frames, saves a screenshot and exits
- `--scene NAME` — opens a demo view (`overview`, `dark`, `compare`, `search`, `histogram`, `editor`);
  this is how the images in this README were made

## Publishing a new release

The GitHub workflow [`Release`](.github/workflows/release.yml) builds all platforms
(Windows x64/ARM64 with installer and portable ZIP, Linux x64/ARM64, macOS Universal)
and creates a release with checksums. A version tag is all it takes:

```sh
git tag v0.1.0
git push origin v0.1.0
```

Via *Actions › Release › Run workflow* the build can also be started without a tag —
the artifacts are then attached to the run.
