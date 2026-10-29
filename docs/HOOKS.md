# Hook sites and game data (13.0.5)

All offsets are relative to `.text`. The plugin checks 64 guard bytes at each
site listed in `src/model.rs` (`SITES`), and it installs either all hooks or
none. The guarded sites are the three hooks plus three functions whose data
layout the hooks depend on. The game version must also read `13.0.5`.

## Hooks (all inline, game main thread)

| Site | Instruction | Registers | What the plugin does |
|---|---|---|---|
| `0x1a12f8c` | `mov x19, x0` after the prologue of the CSS main-state update `0x1a12f60` | x0 = CSS object | Finds P1's hand and the fighter tiles, polls P1, and runs one step if one is due. |
| `0x1b29cbc` | `mov x19, x0` after the prologue of the stage-select update `0x1b29cb0` (vtable `0x50aa728` slot 0) | x0 = stage-select object | The same, with the stage tiles. |
| `0x3786244` | `mov x0, x20` before `bl 0x3763840` (the stick/D-pad query) in the hand cursor update `0x3786140` | x19 = cursor, x20 = input context | Only for P1's hand cursor, and only while L3 is held: points x20 at a static copy of the context. The copy's input object has its four D-pad words cleared. The stick passes through unchanged. When the query returns zero, the rest of the update does not read x20. |

The two screen hooks sit after the prologue rather than at the function entry.
This leaves the entry free for other plugins: the online lobby-window plugin,
for example, replaces `0x1a12f60`.

## Layout witnesses (guarded, not hooked)

| Site | What it confirms |
|---|---|
| `0x3763840` | Cursor input query. It reads the input object at `[ctx+8]`: enabled flag at `+0x4aa`, stick vector at `+0x430`, D-pad bit 4 at `+0x264` (up), `+0x268` (down), `+0x26c` (left) and `+0x270` (right). |
| `0x1a130d0` | CSS fighter hit test. Tiles are `[css+0xdd0]`; a vector of 0xb0-byte entries with the rectangle at `+0x48`. |
| `0x1b32fe0` | Stage select `get_hand_on_stage_panel_id`. A vector at `+0x168` of 0x18-byte entries with the rectangle at `+0x8`. |

## Data

### Hand

The hand is shared by both screens. It is 0x170 bytes; the constructor is
`0x32d2400` and setup is `0x32d2840`.

| Offset | Field |
|---|---|
| `+0x100` | The cursor. It is created by `0x3786640` and its vtable is `0x523dc80`. |
| `+0x163` | Enabled. The binding `set_hand_enable` (`0x1b33440`) sets it. |

### Cursor

Positions are `(x, y, 0, 0)`, in layout units with the origin at the screen
centre and y pointing up.

| Offset | Field |
|---|---|
| `+0x38` | The hand pane. |
| `+0x60` | Current position. |
| `+0x70` | Target position. |
| `+0x80` | Previous position. |
| `+0x90..+0x9c` | Clamp rectangle: xmin, xmax, ymin, ymax. |

The update at `0x3786140` works as follows:

- If target ≠ current, it moves current 30 % of the way to target, snapping
  when the remaining distance squared is 100 or less. It does not read the
  stick or the D-pad in this case.
- Otherwise it runs the input query and moves current by the result, then sets
  target = current.
- In both cases it clamps the position, writes the pane translate, and sets the
  matrix-dirty flag.

The game's own `move_hand_interpolated` (`0x1b33550`) only writes the target.
The plugin does the same.

### Tile rectangles

Each rectangle is four floats: xmin, xmax, ymin, ymax. They are in the hand's
coordinate space, and the game compares the hand's current position against
them directly.

- **CSS.** Vector at `[[css+0xdd0]+0]` to `[[css+0xdd0]+8]`, stride 0xb0,
  rectangle at `+0x48`. The CSS update (`0x1a130d0..0x1a13148`) hit-tests the
  hand only when the hand is not easing. It stores the hovered index for each
  player slot at `[css+0xdd0]+0xe0+slot*0x18`.
- **Stage select.** Vector at `sss+0x168` / `sss+0x170`, stride 0x18,
  rectangle at `+0x8`. It is built from each `set_parts_n_stage_%03d/hit` pane,
  adding the parent panes' translations. The Lua binding
  `get_hand_on_stage_panel_id` returns the first rectangle that contains the
  hand.

### Players and the token

- **CSS players.** Vector at `css+0x238` / `+0x240` of 16-byte entries; the
  player object is at entry `+8`. Each player object has its hand at `+0x20`,
  its slot at `+0x30` (0 = P1), and the hand-active flag at `+0x36`.
- **Stage select hand.** At `sss+0x60`.
- **Token ("medal") manager.** At `[css+0xdf0]` on the CSS and `[sss+0x70]` on
  stage select: 10 entries of 0x130 bytes, plus the token area rectangle at
  `+0xbec`. A token being carried follows the hand through the game's own
  logic. The plugin does not touch it.

## Input

The plugin reads P1's buttons with `nn::hid` for controller No.1 and for
handheld, combining the two. It uses the full key, handheld, joy-dual or
GameCube state, whichever the style set selects. Reading this way does not
consume input.

- L3 is bit 4.
- D-pad left, up, right and down are bits 12, 13, 14 and 15.

The game itself still sees every button. The only change is that the hand
cursor's D-pad query returns nothing while L3 is held.
