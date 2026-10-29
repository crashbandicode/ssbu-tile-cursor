# Hook sites and game data (13.0.5)

All offsets are relative to `.text`. The plugin checks 64 guard bytes at each
site listed in `src/model.rs` (`SITES`), and it installs either all hooks or
none. The guarded sites are the three hooks plus seven places whose data
layout the hooks depend on. The game version must also read `13.0.5`.

## Hooks (all inline, game main thread)

| Site | Instruction | Registers | What the plugin does |
|---|---|---|---|
| `0x1a2b5a4` | `mov x28, x0` after the prologue of the CSS players update `0x1a2b570` | x0 = CSS object | Finds P1's hand and the fighter tiles. While the hand carries a token, it polls P1 and runs one step if one is due. |
| `0x1b29cbc` | `mov x19, x0` after the prologue of the stage-select update `0x1b29cb0` (vtable `0x50aa728` slot 0) | x0 = stage-select object | The same, with the stage tiles, whenever the hand is enabled. |
| `0x3786244` | `mov x0, x20` before `bl 0x3763840` (the stick/D-pad query) in the hand cursor update `0x3786140` | x19 = cursor, x20 = input context | Only for P1's hand cursor, only while a screen hook has just seen that hand able to step, and only while L3 is held: points x20 at a static copy of the context. The copy's input object has its four D-pad words cleared. The stick passes through unchanged. When the query returns zero, the rest of the update does not read x20. |

The two screen hooks sit after the prologue rather than at the function entry.
This leaves the entry free for other plugins that replace it.

### Why the CSS hook is in the players update

The CSS object runs a 12-state machine. Its state functions are member
pointers at `css+0x28 + state*0x10`, and `0x1a18f30` applies requested state
changes (request stack at `css+0xe8`, depth at `css+0x134`). The fighter-picking
update `0x1a12f60` is state 4 only. Nothing calls it directly: its one
reference stores it in that table (`0x1845638`).

The players update `0x1a2b570` is called every frame by the CSS scene task
(vtable slot `0x50a8b20` -> `0x1a31860`, CSS object at task `+0x80`), whatever
the state. It is the local CSS's only caller of the hand's own update
`0x32d2ba0`: the loop over `css+0x238` at `0x1a2fcc8`..`0x1a2fcec`. A hand that
moves and animates is therefore updated by this function in that frame.

Build `fcd1395` hooked `0x1a12f8c` and picked "P1" as the player whose `+0x30`
was 0. That field is the slot of the carried token, so the build found P1's
hand only while it held P1's own token.

## Layout witnesses (guarded, not hooked)

| Site | What it confirms |
|---|---|
| `0x3763840` | Cursor input query. It reads the input object at `[ctx+8]`: enabled flag at `+0x4aa`, stick vector at `+0x430`, D-pad bit 4 at `+0x264` (up), `+0x268` (down), `+0x26c` (left) and `+0x270` (right). |
| `0x1a130d0` | CSS fighter hit test. Tiles are `[css+0xdd0]`; a vector of 0xb0-byte entries with the rectangle at `+0x48`. |
| `0x1b32fe0` | Stage select `get_hand_on_stage_panel_id`. A vector at `+0x168` of 0x18-byte entries with the rectangle at `+0x8`. |
| `0x1a0e528` | CSS player constructor (`0x1a0e450`). It stores the player index at `+0x28` and -1 at `+0x2c` and `+0x30`. |
| `0x1a14740` | CSS pick-up. It sets the hand's holding flag `+0x167` and the player's carried slot `+0x30` together. The put-down at `0x1a164f4` clears both. |
| `0x1a13078` | CSS fighter-picking update, per player. It reads the carried slot `+0x30`, the active flag `+0x36`, the hand `+0x20` and its cursor `+0x100`. |
| `0x1a13148` | The same update. It skips the hover when the carried slot is -1. Otherwise it compares the hit tile with the hover record of that slot. |

## Data

### Hand

The hand is shared by both screens. It is 0x170 bytes; the constructor is
`0x32d2400` and setup is `0x32d2840`.

| Offset | Field |
|---|---|
| `+0x100` | The cursor. It is created by `0x3786640` and its vtable is `0x523dc80`. |
| `+0x163` | Enabled. The binding `set_hand_enable` (`0x1b33440`) sets it. |
| `+0x166` | Over a token that can be picked up. |
| `+0x167` | Holding a token. |
| `+0xe8` | Animation state of the hand's own 7-state machine (`0x32d2ba0`): 1 wait, 2 put, 3 point, 4 take/hold, 5 and 6 outside the hand area. |

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
  rectangle at `+0x48`. The fighter-picking update (`0x1a130d0..0x1a13148`)
  hit-tests a hand only when the hand is not easing and carries a token. It
  stores the hovered index at `[css+0xdd0]+0xe0+slot*0x18`, where the slot is
  the carried token's. A change previews that fighter on the slot's panel
  (`0x19f11d0`).
- **Stage select.** Vector at `sss+0x168` / `sss+0x170`, stride 0x18,
  rectangle at `+0x8`. It is built from each `set_parts_n_stage_%03d/hit` pane,
  adding the parent panes' translations. The Lua binding
  `get_hand_on_stage_panel_id` returns the first rectangle that contains the
  hand.

### Players and the token

- **CSS players.** Vector at `css+0x238` / `+0x240` of 16-byte entries; the
  player object is at entry `+8`. Each player object has:
  - its hand at `+0x20`;
  - its index at `+0x28` (0 = P1);
  - the slot of the token under its hand at `+0x2c`;
  - the slot of the token its hand carries at `+0x30` (-1 for none; 0 is P1's
    own token, 1 the token of slot 2, and so on);
  - the hand-active flag at `+0x36`.

  The plugin steps P1's hand on the CSS only while `+0x30` is a slot (0..7)
  and the hand's `+0x167` is set. With an empty hand, L3 + D-pad does nothing
  and the D-pad behaves as in the unmodified game.
- **CSS panels.** Vector at `css+0x250` of 16-byte entries, panel at `+8`.
  Each panel has its slot at `+0x150`.
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

The game itself still sees every button. The only change is that P1's hand
cursor's D-pad query returns nothing while L3 is held and the hand can step.
