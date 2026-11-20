"""Generate the stock 13.0.5 character-select tile table from the game's own files.

    python3 -I tools/css-table/generate.py --arc <romfs/data.arc> --main <main NSO, uncompressed> \
        --readers <ssbu-recomp checkout> [--check]

Reads only, never writes the inputs:
  * ui/param/database/ui_chara_db.prc: CSS order (disp_order), typed identity (row, ui_chara_id),
    fighter_kind hash, DLC and Mii flags;
  * ui/message/msg_name.msbt, English region 2: the tile caption;
  * ui/layout/menu/chara_select/chara_select/layout.arc, blyt/chara_select.bflyt: the 13 x 7
    set_parts_chr_s_NN tile slots and their pitch;
  * the exact 13.0.5 main NSO: the fighter-kind-ordered table of hash40("fighter_kind_*") words,
    whose index is the application fighter kind.

Writes tools/css-table/css-table.json (full table and provenance) and src/css_table.rs.
--check regenerates both in memory and fails if a checked-in file differs.
The archive and PRC readers are the ssbu-recomp tools; their SHA-256 is recorded too.
"""
import argparse
import hashlib
import importlib.util
import json
import struct
import sys
import types
import zlib
from pathlib import Path

from compression import zstd

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
JSON_OUT = HERE / "css-table.json"
RS_OUT = REPO / "src" / "css_table.rs"

ASSETS = {
    "chara_db": ("ui/param/database/ui_chara_db.prc", 0,
                 "c65a0da57a20fe790c2f0948d017a54c4885341885d9a30866c590ccf380f380"),
    "names": ("ui/message/msg_name.msbt", 2,
              "9efc357db8ea46e671222dbbd90bbdc0ea5455383090fae3fe2ba6017e9da3ad"),
    "layout": ("ui/layout/menu/chara_select/chara_select/layout.arc", 0,
               "751fede427d5072f3c9aa0939c677f1d2abb54f0a6190bf06ad5550d1b0f4150"),
}
MAIN_SHA256 = "8b3dc2eb011f26dda1f87947e4f62665e8d8fe3d84397e85ecf9b3cd9d0137ce"
BUILD_ID = "21450c647b8c5940ed5b68218ea128fd3c5d597d"
KIND_TABLE = 0x453CC58  # main+ virtual address, read-only data
KIND_COUNT = 94
TILES = 87
TAG = 0xC1
NO_ALTERNATE = 0xC1FF_FF00_0000_0000
# The one ui_chara whose fighter_kind is not a fighter: the game plays its leader.
LEADER = {"ice_climber": "popo"}


def hash40(text):
    data = text.encode("ascii")
    return len(data) << 32 | zlib.crc32(data)


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def assets(arc_path, readers):
    sys.modules["zstandard"] = types.SimpleNamespace(ZstdDecompressor=lambda: types.SimpleNamespace(
        decompress=lambda data, max_output_size: zstd.decompress(data)))
    arc_tool = readers / "tools/nro/arc_extract.py"
    arc = load("arc_extract", arc_tool).Arc(str(arc_path))
    blobs, records = {}, {}
    for key, (path, region, digest) in ASSETS.items():
        index = arc.path_index_from_hash(hash40(path))
        record = arc.file_record(index, region)
        data = arc.read(record)
        if sha256(data) != digest:
            raise SystemExit(f"{path}: SHA-256 {sha256(data)} != {digest}")
        blobs[key] = data
        records[key] = dict(path=path, path_hash40=f"0x{hash40(path):010x}", region=region,
                            arc_offset=record["offset"], comp_size=record["comp_size"],
                            size=record["decomp_size"], sha256=digest)
    return blobs, records, arc_tool


def chara_rows(data, prc):
    root = prc.Prc(data).root
    _, db = next(value for _, value in root[1] if value[0] == "list")
    wanted = ("ui_chara_id", "name_id", "fighter_kind", "disp_order", "is_dlc", "is_mii")
    rows = []
    for row, (_, fields) in enumerate(db):
        values = {key: value for key, (_, value) in fields}
        rows.append(dict(row=row, **{name: values[prc.hash40(name)] for name in wanted}))
    return rows


def msbt_labels(data):
    assert data[:8] == b"MsgStdBn"
    count, = struct.unpack_from("<H", data, 14)
    pos, sections = 0x20, {}
    for _ in range(count):
        tag, size = struct.unpack_from("<4sI", data, pos)
        sections[tag] = data[pos + 0x10:pos + 0x10 + size]
        pos = (pos + 0x10 + size + 15) & -16
    groups, texts = sections[b"LBL1"], sections[b"TXT2"]
    n, = struct.unpack_from("<I", texts)
    offsets = struct.unpack_from(f"<{n}I", texts, 4) + (len(texts),)
    labels = {}
    for i in range(struct.unpack_from("<I", groups)[0]):
        size, cursor = struct.unpack_from("<II", groups, 4 + 8 * i)
        for _ in range(size):
            length = groups[cursor]
            name = groups[cursor + 1:cursor + 1 + length].decode("ascii")
            index, = struct.unpack_from("<I", groups, cursor + 1 + length)
            labels[name] = texts[offsets[index]:offsets[index + 1]].decode("utf-16-le").rstrip("\0")
            cursor += 1 + length + 4
    return labels


def sarc_file(data, wanted):
    assert data[:4] == b"SARC"
    header, = struct.unpack_from("<H", data, 4)
    base, = struct.unpack_from("<I", data, 0x0C)
    assert data[header:header + 4] == b"SFAT"
    count, = struct.unpack_from("<H", data, header + 6)
    names = header + 12 + 16 * count
    assert data[names:names + 4] == b"SFNT"
    for i in range(count):
        _, attr, start, end = struct.unpack_from("<IIII", data, header + 12 + 16 * i)
        at = names + 8 + (attr & 0xFFFF) * 4
        if attr & 0x0100_0000 and data[at:data.index(b"\0", at)].decode() == wanted:
            return data[base + start:base + end]
    raise SystemExit(f"layout.arc has no {wanted}")


def panes(flyt):
    """Absolute translation, size, scale and rotation of every named pane."""
    assert flyt[:4] == b"FLYT"
    header, = struct.unpack_from("<H", flyt, 6)
    count, = struct.unpack_from("<H", flyt, 0x10)
    pos, stack, last, out = header, [], None, {}
    for _ in range(count):
        magic, size = flyt[pos:pos + 4], struct.unpack_from("<I", flyt, pos + 4)[0]
        if magic == b"pas1":
            stack.append(last)
        elif magic == b"pae1":
            stack.pop()
        elif magic in (b"pan1", b"pic1", b"txt1", b"wnd1", b"bnd1", b"prt1", b"scr1", b"ali1"):
            name = flyt[pos + 0x0C:pos + 0x24].split(b"\0")[0].decode("ascii")
            tx, ty, _ = struct.unpack_from("<3f", flyt, pos + 0x2C)
            rotate = struct.unpack_from("<3f", flyt, pos + 0x38)
            scale = struct.unpack_from("<2f", flyt, pos + 0x44)
            w, h = struct.unpack_from("<2f", flyt, pos + 0x4C)
            out[name] = dict(kind=magic.decode(), local=(tx, ty), size=(w, h), scale=scale, rotate=rotate,
                             parent=stack[-1] if stack else None, offset=pos)
            last = name
        pos += size
    for pane in out.values():
        x, y = pane["local"]
        parent = pane["parent"]
        while parent is not None:
            p = out[parent]
            pane.setdefault("chain", []).append(parent)
            x, y = f32(p["local"][0] + x), f32(p["local"][1] + y)
            parent = p["parent"]
        pane["at"] = (x, y)
    return out


def untransformed(layout, name):
    """A pane whose ancestors neither scale nor rotate; its absolute translation is then exact."""
    for parent in layout[name].get("chain", []):
        p = layout[parent]
        assert p["scale"] == (1.0, 1.0) and p["rotate"] == (0.0, 0.0, 0.0), (name, parent)
    return layout[name]


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def main_kinds(path):
    data = Path(path).read_bytes()
    if sha256(data) != MAIN_SHA256:
        raise SystemExit(f"main: SHA-256 {sha256(data)} != {MAIN_SHA256}")
    assert data[:4] == b"NSO0" and data[0x40:0x54].hex() == BUILD_ID
    segments = [struct.unpack_from("<3I", data, 0x10 + 0x10 * i) for i in range(3)]
    file_offset = next(fo + KIND_TABLE - mo for fo, mo, size in segments if mo <= KIND_TABLE < mo + size)
    words = list(struct.unpack_from(f"<{KIND_COUNT}Q", data, file_offset))
    assert len(set(words)) == KIND_COUNT
    table = data[file_offset:file_offset + 8 * KIND_COUNT]
    return words, dict(file="main NSO, uncompressed", sha256=MAIN_SHA256, build_id=BUILD_ID,
                       kind_table=f"main+0x{KIND_TABLE:x}", file_offset=f"0x{file_offset:x}",
                       entries=KIND_COUNT, table_sha256=sha256(table))


def derive(args):
    readers = Path(args.readers)
    blobs, records, arc_tool = assets(args.arc, readers)
    prc_tool = readers / "fighter_sdk/tools/brawl/prc.py"
    rows = chara_rows(blobs["chara_db"], load("prc", prc_tool))
    labels = msbt_labels(blobs["names"])
    kinds, main_record = main_kinds(args.main)
    kind_of = {word: kind for kind, word in enumerate(kinds)}

    # CSS order: displayed rows by disp_order. Rows sharing one disp_order share a tile, the
    # first as its typed word and the second as its alternate. The combined Mii row (is_mii
    # without a fighter kind) is not a stock Training tile; the three Mii fighters are.
    shown = sorted((r for r in rows if r["disp_order"] >= 0 and not (r["is_mii"] and r["fighter_kind"] == 0)),
                   key=lambda r: (r["disp_order"], r["row"]))
    groups = []
    for r in shown:
        if groups and groups[-1][0]["disp_order"] == r["disp_order"]:
            groups[-1].append(r)
        else:
            groups.append([r])
    assert len(groups) == TILES and all(len(g) in (1, 2) for g in groups), len(groups)

    flyt = sarc_file(blobs["layout"], "blyt/chara_select.bflyt")
    layout = panes(flyt)
    slots = [untransformed(layout, f"set_parts_chr_s_{i:02d}") for i in range(91)]
    assert all(s["kind"] == "prt1" for s in slots)
    columns = sum(1 for s in slots if s["at"][1] == slots[0]["at"][1])
    pitch = (f32(slots[1]["at"][0] - slots[0]["at"][0]), f32(slots[0]["at"][1] - slots[columns]["at"][1]))
    assert len(slots) % columns == 0
    for i, s in enumerate(slots):
        expected = (f32(slots[0]["at"][0] + pitch[0] * (i % columns)), f32(slots[0]["at"][1] - pitch[1] * (i // columns)))
        assert s["at"] == expected, (i, s["at"], expected)
    full = TILES // columns * columns
    rest = TILES - full
    # The partial last row is centred on the full grid.
    assert (columns - rest) % 2 == 0
    shift = (columns - rest) // 2
    area = untransformed(layout, "set_chr_area")

    tiles = []
    for tile, group in enumerate(groups):
        first = group[0]
        name_id = first["name_id"]
        # The typed word carries the hash itself; single-row tiles also resolve its text.
        ui_chara = [f"ui_chara_{r['name_id']}" if r["ui_chara_id"] == hash40("ui_chara_" + r["name_id"])
                    else f"0x{r['ui_chara_id']:010x}" for r in group]
        assert len(group) == 2 or ui_chara[0] == "ui_chara_" + name_id, group
        word = TAG << 56 | first["row"] << 40 | first["ui_chara_id"]
        alternate = (TAG << 56 | group[1]["row"] << 40 | group[1]["ui_chara_id"]) if len(group) == 2 else NO_ALTERNATE
        fk = first["fighter_kind"]
        if len(group) == 2:
            kind, rule = None, "none: two ui_chara rows share this tile"
        elif fk == 0:
            kind, rule = None, "none: ui_chara has no fighter_kind"
        elif fk in kind_of:
            kind, rule = kind_of[fk], "main kind table index"
        elif fk == hash40("fighter_kind_" + name_id) and name_id in LEADER:
            kind, rule = kind_of[hash40("fighter_kind_" + LEADER[name_id])], f"leader fighter_kind_{LEADER[name_id]}"
        else:
            kind, rule = None, "none: fighter_kind is not in the main kind table"
        key = next(k for k in (f"nam_chr1_00_{name_id}", f"nam_chr0_00_{name_id}") if k in labels)
        name = labels[key].rstrip()
        assert name and "\n" not in name and '"' not in name, (key, name)
        slot = tile if tile < full else tile + shift
        cx, cy = slots[slot]["at"]
        half = (f32(pitch[0] * 0.5), f32(pitch[1] * 0.5))
        rect = [f32(cx - half[0]), f32(cx + half[0]), f32(cy - half[1]), f32(cy + half[1])]
        tiles.append(dict(tile=tile, ui_rows=[r["row"] for r in group], name_ids=[r["name_id"] for r in group],
                          ui_chara_ids=ui_chara,
                          word=f"0x{word:016x}", alternate=f"0x{alternate:016x}",
                          fighter_kind=f"0x{fk:010x}", kind=kind, kind_rule=rule, label_key=key, name=name,
                          disp_order=first["disp_order"], dlc=bool(first["is_dlc"]), mii=bool(first["is_mii"]),
                          hover_focus=f"css.grid.{name_id}", placed_focus=f"css.medal.{name_id}",
                          slot=f"set_parts_chr_s_{slot:02d}", rect=rect))
    provenance = dict(
        generator="tools/css-table/generate.py",
        readers={"arc_extract.py": sha256(arc_tool.read_bytes()), "prc.py": sha256(prc_tool.read_bytes())},
        assets=records, main=main_record,
        layout=dict(pane_file="blyt/chara_select.bflyt", pane_file_sha256=sha256(flyt), columns=columns,
                    rows=len(slots) // columns, pitch=list(pitch), last_row_tiles=rest, last_row_shift=shift,
                    set_chr_area=dict(at=list(area["at"]), size=list(area["size"]))),
        rules=["tile order: ui_chara_db rows with disp_order >= 0 sorted by (disp_order, row), excluding the "
               "combined Mii row (is_mii=1, fighter_kind=0)",
               "rows sharing a disp_order share one tile: first row = typed word, second row = alternate word",
               "typed word = 0xc1 << 56 | ui_chara_db row << 40 | hash40(ui_chara_id)",
               "application kind = index of the ui_chara fighter_kind hash in the main kind table; "
               "ice_climber plays fighter_kind_popo; composite tiles and Random have none",
               "caption = English msg_name nam_chr1_00_<name_id> (else nam_chr0_00_<name_id>), trailing space trimmed",
               "rect = slot centre +- half the slot pitch; the partial last row is centred on the 13-column grid"],
    )
    return dict(screen="css", tiles=tiles, provenance=provenance)


def rust(table):
    p = table["provenance"]
    def rs_f32(v):
        text = repr(float(v))
        return text if "." in text or "e" in text else text + ".0"
    def rs_str(v):
        return json.dumps(v, ensure_ascii=False)
    lines = [
        "// @generated by tools/css-table/generate.py from the stock 13.0.5 game files. Do not edit.",
        "//! Stock Training character-select tiles, in native tile-index order.",
        f"//! ui_chara_db.prc {p['assets']['chara_db']['sha256']}",
        f"//! msg_name.msbt (region 2) {p['assets']['names']['sha256']}",
        f"//! chara_select layout.arc {p['assets']['layout']['sha256']}",
        f"//! main {p['main']['sha256']}, kind table {p['main']['kind_table']} ({p['main']['entries']} entries)",
        "",
        "pub struct CssTile {",
        "    /// Typed preview word: tag 0xc1, ui_chara_db row, Hash40(ui_chara_id).",
        "    pub word: u64,",
        "    /// Application fighter kind; none for composite tiles and Random.",
        "    pub kind: Option<u16>,",
        "    pub name: &'static str,",
        "    pub hover_focus: &'static str,",
        "    pub placed_focus: &'static str,",
        "}",
        "",
        f"pub static CSS_TILES: [CssTile; {len(table['tiles'])}] = [",
    ]
    for t in table["tiles"]:
        kind = "None" if t["kind"] is None else f"Some({t['kind']})"
        lines.append(f"    CssTile {{ word: 0x{int(t['word'], 16):016x}, kind: {kind}, name: {rs_str(t['name'])}, "
                     f"hover_focus: {rs_str(t['hover_focus'])}, placed_focus: {rs_str(t['placed_focus'])} }},")
    lines += ["];", "", "/// Alternate word and rectangle (xmin, xmax, ymin, ymax) derived from the stock layout.",
              "#[cfg(test)]", f"pub static CSS_GEOMETRY: [(u64, [f32; 4]); {len(table['tiles'])}] = ["]
    for t in table["tiles"]:
        lines.append(f"    (0x{int(t['alternate'], 16):016x}, [{', '.join(rs_f32(v) for v in t['rect'])}]),")
    lines += ["];", ""]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--arc", required=True)
    parser.add_argument("--main", required=True)
    parser.add_argument("--readers", required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    table = derive(args)
    outputs = {JSON_OUT: json.dumps(table, ensure_ascii=False, indent=1) + "\n", RS_OUT: rust(table)}
    stale = [str(path) for path, text in outputs.items()
             if not path.exists() or path.read_text(encoding="utf-8") != text]
    if args.check:
        if stale:
            raise SystemExit("stale generated files: " + ", ".join(stale))
        print("generated files are current")
        return
    for path, text in outputs.items():
        path.write_text(text, encoding="utf-8")
        print(path, sha256(text.encode("utf-8")))


if __name__ == "__main__":
    main()
