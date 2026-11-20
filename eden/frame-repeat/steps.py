"""Frame-repeat acceptance inside the shared runner; no wall-time input holds."""
import json
import re
import time
from pathlib import Path
import routes


def check_repeat(lines, screen):
    steps = [m.groups() for line in lines if (m := re.search(
        rf"step screen={screen} dir=right from=(\S+) to=(\S+)", line))]
    assert steps == [("0", "1"), ("1", "2"), ("2", "3")], steps
    pattern = rf"settled screen={screen} at=3 hovered=" + ("3" if screen == "css" else "none") + r"(?: |$)"
    assert any(re.search(pattern, line) for line in lines), lines
    return {"screen": screen, "steps": steps, "held_frames": 32, "final_tile": 3, "lines": lines}


def held_repeat(ctx, screen):
    if screen == "sss":
        routes.wait_sss(ctx)
    else:
        routes.gate(ctx, "css")
    walker = routes.TileWalker(ctx, screen)
    walker.walk_to(0)
    walker.cursor.close()
    cursor = routes.LogCursor(routes.instance_path(ctx, "tile_log"))
    try:
        ctx.play("held-right-32")
        lines = []
        until = time.monotonic() + 10
        while True:
            ctx.check_stop()
            lines.extend(cursor.read())
            if any(re.search(rf"settled screen={screen} at=3 hovered=", line) for line in lines):
                break
            if time.monotonic() > until:
                raise RuntimeError(f"held repeat never settled at 3: {lines}")
            time.sleep(.1)
        result = check_repeat(lines, screen)
        result["shot"] = str(ctx.shot(f"{screen}-held-repeat"))
        (Path(ctx.row_dir)/f"{screen}-repeat.json").write_text(json.dumps(result, indent=2)+"\n")
        return result
    finally:
        cursor.close()
