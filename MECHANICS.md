# 101Craft – editing rules (from the original's code)

Read with Ghidra from 010Editor.exe (sha256 d1b430fa…). Rules are re-implemented in our
own code; nothing is copied.

## Keys (0x1404149c0, editor action table)
| Action | Keys |
|---|---|
| Delete Current | Delete (Qt 0x1000007) |
| Delete Previous | Backspace (0x1000003) or Shift+Backspace (0x3000003) |

## Delete Previous = Backspace (0x140420860, key handler)
1. Caret at byte 0, first digit, no selection: nothing happens. With a selection: the
   selection is deleted.
2. Otherwise the caret steps one character to the left (mid-byte → first digit of the
   same byte; at a byte boundary → the previous byte), then Delete Current runs
   (only if the caret is still inside the file).
3. The caret move (0x14041a5c0) keeps the selection, so with a selection Backspace deletes
   the selection.

## Delete Current (0x1401428b0)
- If the editor is in **overwrite** mode and *Allow Delete in Overwrite Mode* is off:
  refused with the status text "Delete not allowed in overwrite mode." (0x1407f4690).
- With a selection: the selection is deleted. Without: the byte at the caret.
- Caret goes to the start of what was deleted.
- Status bar: "Deleted: " + byte count (0x14042e580).

## Allow Delete in Overwrite Mode
- Options default **1 (on)**: `mov dword [rdi+0x7a0], 1` at 0x1403d6d33 inside the options
  defaults constructor 0x1403d5db0 (same function sets "h:mm AP M/d/yyyy", "#RRGGBB").
- Checked as `options+0x7a0` in 0x1401428b0; shown in Options as "Allow Delete in Overwrite
  Mode" (0x1401e80a0). In 101Craft: Edit menu checkbox, default on.

## Byte count wording (0x14042e580)
| n | Text |
|---|---|
| 1 | "1 byte" |
| 2–9 | "N bytes" |
| ≥ 10 | "N [Xh] bytes" (hex in brackets) |
| < 0 | "Error on <action>" |

## Cut (0x140142730)
- No selection: status "No bytes selected." and nothing else.
- With a selection: copy, then Delete Current, status "Cut: " + byte count.

## Bytes per line (0x14019da70)
- Must parse as a number ≥ 0 and the parse must succeed, else "Invalid bytes per line."
  101Craft keeps its toolbar choice (default 16 – *estimated*, not found in the defaults yet).

## Crash fix (this round)
The hex view measured the file length before handling keys and painted with the old length
afterwards; after a delete it read one byte past the end. It now re-reads the length and
clamps caret and selection after handling keys.
