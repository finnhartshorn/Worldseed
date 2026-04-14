#!/usr/bin/env python3
"""
sprite_bounds.py — Find bounding boxes of distinct sprites in a PNG spritesheet.

Uses connected-component analysis on the alpha channel (pure Python, no dependencies).
Useful for locating sprites in sheets with irregular layouts or variable spacing.

Usage:
    python3 tools/sprite_bounds.py <path/to/sheet.png> [--scan-width W] [--scan-height H] [--min-pixels N]
    python3 tools/sprite_bounds.py <path/to/sheet.png> --frame-width W --frame-height H [--stable] [--per-row]

Arguments:
    path          Path to the PNG file.
    --scan-width  Only analyse the leftmost W pixels (default: full width).
    --scan-height Only analyse the topmost H pixels (default: full height).
    --min-pixels   Ignore components with fewer than N opaque pixels (default: 5).
    --frame-width  Fixed frame width for frame-aware analysis.
    --frame-height Fixed frame height for frame-aware analysis.
    --stable       Report a stable crop box shared by all populated frames.
    --per-row      With frame-aware analysis, also report stable crop boxes per row.
    --emit-bevy-helper NAME
                   With --stable and frame analysis, emit a Bevy helper function with this name.

Output:
    Numbered list of sprite bounding boxes with pixel coordinates and sizes,
    suitable for pasting into Bevy's Rect::new() or TextureAtlasLayout::add_texture().
"""

import argparse
import struct
import sys
import zlib
from typing import Optional


# ---------------------------------------------------------------------------
# PNG decoder (stdlib only)
# ---------------------------------------------------------------------------

def decode_png(path: str) -> tuple[int, int, list[list[int]]]:
    """Decode a PNG file and return (width, height, pixel_rows).

    Each row is a flat list of RGBA bytes: [R, G, B, A, R, G, B, A, ...].
    Supports filter types 0-4. Requires 8-bit RGBA (color type 6).
    """
    with open(path, "rb") as f:
        data = f.read()

    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("Not a valid PNG file")

    width = struct.unpack(">I", data[16:20])[0]
    height = struct.unpack(">I", data[20:24])[0]
    bit_depth = data[24]
    color_type = data[25]

    if bit_depth != 8 or color_type != 6:
        raise ValueError(
            f"Only 8-bit RGBA PNGs are supported (got bit_depth={bit_depth}, color_type={color_type})"
        )

    # Collect and decompress all IDAT chunks
    idat = b""
    i = 8
    while i < len(data):
        chunk_len = struct.unpack(">I", data[i : i + 4])[0]
        chunk_type = data[i + 4 : i + 8]
        chunk_data = data[i + 8 : i + 8 + chunk_len]
        if chunk_type == b"IDAT":
            idat += chunk_data
        i += 12 + chunk_len

    raw = zlib.decompress(idat)

    bpp = 4  # bytes per pixel (RGBA)
    stride = width * bpp
    rows: list[list[int]] = []
    pos = 0
    prev = [0] * stride

    for _ in range(height):
        ft = raw[pos]
        pos += 1
        row = list(raw[pos : pos + stride])
        pos += stride

        if ft == 0:
            pass
        elif ft == 1:  # Sub
            for x in range(bpp, stride):
                row[x] = (row[x] + row[x - bpp]) & 0xFF
        elif ft == 2:  # Up
            for x in range(stride):
                row[x] = (row[x] + prev[x]) & 0xFF
        elif ft == 3:  # Average
            for x in range(stride):
                a = row[x - bpp] if x >= bpp else 0
                row[x] = (row[x] + (a + prev[x]) // 2) & 0xFF
        elif ft == 4:  # Paeth
            for x in range(stride):
                a = row[x - bpp] if x >= bpp else 0
                b = prev[x]
                c = prev[x - bpp] if x >= bpp else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pr = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                row[x] = (row[x] + pr) & 0xFF
        else:
            raise ValueError(f"Unknown PNG filter type {ft}")

        rows.append(row)
        prev = row

    return width, height, rows


# ---------------------------------------------------------------------------
# Connected-component analysis
# ---------------------------------------------------------------------------

def find_opaque_pixels(
    rows: list[list[int]],
    scan_w: int,
    scan_h: int,
) -> set[tuple[int, int]]:
    """Return the set of (x, y) pixels with alpha > 0 within the scan region."""
    opaque: set[tuple[int, int]] = set()
    for y in range(min(scan_h, len(rows))):
        row = rows[y]
        for x in range(min(scan_w, len(row) // 4)):
            if row[x * 4 + 3] > 0:
                opaque.add((x, y))
    return opaque


def flood_fill(
    start: tuple[int, int],
    opaque: set[tuple[int, int]],
    visited: set[tuple[int, int]],
) -> list[tuple[int, int]]:
    """4-connected flood fill starting from *start*. Returns pixels in component."""
    stack = [start]
    component: list[tuple[int, int]] = []
    while stack:
        p = stack.pop()
        if p in visited or p not in opaque:
            continue
        visited.add(p)
        component.append(p)
        x, y = p
        stack.extend([(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)])
    return component


def find_components(
    opaque: set[tuple[int, int]],
    min_pixels: int,
) -> list[dict]:
    """Return a list of component dicts sorted by (min_y, min_x)."""
    visited: set[tuple[int, int]] = set()
    components: list[dict] = []

    for p in sorted(opaque):
        if p in visited:
            continue
        comp = flood_fill(p, opaque, visited)
        if len(comp) < min_pixels:
            continue
        xs = [c[0] for c in comp]
        ys = [c[1] for c in comp]
        components.append(
            {
                "pixels": len(comp),
                "min_x": min(xs),
                "max_x": max(xs),
                "min_y": min(ys),
                "max_y": max(ys),
                "width": max(xs) - min(xs) + 1,
                "height": max(ys) - min(ys) + 1,
            }
        )

    components.sort(key=lambda c: (c["min_y"], c["min_x"]))
    return components


def component_to_bevy_rect(component: dict) -> str:
    """Format a component as a Bevy Rect::new() string."""
    x0, y0 = component["min_x"], component["min_y"]
    x1, y1 = component["max_x"] + 1, component["max_y"] + 1
    return f"Rect::new({x0}.0, {y0}.0, {x1}.0, {y1}.0)"


def find_frame_components(
    rows: list[list[int]],
    width: int,
    height: int,
    frame_width: int,
    frame_height: int,
    min_pixels: int,
) -> list[dict]:
    """Return sprite bounds for each populated frame in a fixed grid."""
    frames: list[dict] = []
    columns = width // frame_width
    rows_count = height // frame_height

    for row in range(rows_count):
        for col in range(columns):
            frame_min_x = col * frame_width
            frame_min_y = row * frame_height
            frame_max_x = min(frame_min_x + frame_width, width)
            frame_max_y = min(frame_min_y + frame_height, height)

            opaque: set[tuple[int, int]] = set()
            for y in range(frame_min_y, frame_max_y):
                pixel_row = rows[y]
                for x in range(frame_min_x, frame_max_x):
                    if pixel_row[x * 4 + 3] > 0:
                        opaque.add((x, y))

            components = find_components(opaque, min_pixels)
            if not components:
                continue

            min_x = min(component["min_x"] for component in components)
            min_y = min(component["min_y"] for component in components)
            max_x = max(component["max_x"] for component in components)
            max_y = max(component["max_y"] for component in components)

            frames.append(
                {
                    "frame_index": row * columns + col,
                    "frame_col": col,
                    "frame_row": row,
                    "frame_min_x": frame_min_x,
                    "frame_min_y": frame_min_y,
                    "frame_max_x": frame_max_x,
                    "frame_max_y": frame_max_y,
                    "min_x": min_x,
                    "min_y": min_y,
                    "max_x": max_x,
                    "max_y": max_y,
                    "width": max_x - min_x + 1,
                    "height": max_y - min_y + 1,
                    "pixels": sum(component["pixels"] for component in components),
                    "rel_min_x": min_x - frame_min_x,
                    "rel_min_y": min_y - frame_min_y,
                    "rel_max_x": (max_x + 1) - frame_min_x,
                    "rel_max_y": (max_y + 1) - frame_min_y,
                }
            )

    return frames


def stable_crop_box(frames: list[dict]) -> Optional[dict]:
    """Return a shared crop box relative to a frame cell."""
    if not frames:
        return None

    rel_min_x = min(frame["rel_min_x"] for frame in frames)
    rel_min_y = min(frame["rel_min_y"] for frame in frames)
    rel_max_x = max(frame["rel_max_x"] for frame in frames)
    rel_max_y = max(frame["rel_max_y"] for frame in frames)

    return {
        "min_x": rel_min_x,
        "min_y": rel_min_y,
        "max_x": rel_max_x,
        "max_y": rel_max_y,
        "width": rel_max_x - rel_min_x,
        "height": rel_max_y - rel_min_y,
    }


def print_frame_components(frames: list[dict]) -> None:
    """Print populated frame bounds."""
    print(f"Found {len(frames)} populated frame(s):\n")
    print(
        f"  {'idx':>3}  {'row':>3}  {'col':>3}  {'x range':>14}  {'y range':>14}  {'size':>12}  {'pixels':>8}  Bevy Rect::new()"
    )
    print(
        f"  {'-'*3}  {'-'*3}  {'-'*3}  {'-'*14}  {'-'*14}  {'-'*12}  {'-'*8}  {'-'*32}"
    )

    for frame in frames:
        x_range = f"{frame['min_x']}–{frame['max_x']}"
        y_range = f"{frame['min_y']}–{frame['max_y']}"
        size_str = f"{frame['width']}×{frame['height']}"
        print(
            f"  {frame['frame_index']:>3}  {frame['frame_row']:>3}  {frame['frame_col']:>3}  "
            f"{x_range:>14}  {y_range:>14}  {size_str:>12}  {frame['pixels']:>8}  "
            f"{component_to_bevy_rect(frame)}"
        )


def print_stable_crop(label: str, crop: dict) -> None:
    """Print a stable crop summary."""
    print(f"{label}:")
    print(
        f"  relative box : ({crop['min_x']}, {crop['min_y']}) -> ({crop['max_x']}, {crop['max_y']})"
    )
    print(f"  size         : {crop['width']}×{crop['height']}")
    print(
        f"  Bevy URect   : URect::new({crop['min_x']}, {crop['min_y']}, {crop['max_x']}, {crop['max_y']})"
    )
    print()


def print_bevy_helper(function_name: str, frame_width: int, frame_height: int, width: int, height: int, crop: dict) -> None:
    """Print a Bevy helper function for a stable fixed-grid cropped atlas."""
    columns = width // frame_width
    rows = height // frame_height

    print(f"Bevy helper (`{function_name}`):")
    print("```rust")
    print(f"pub fn {function_name}(")
    print("    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,")
    print(") -> Handle<TextureAtlasLayout> {")
    print(
        f"    let mut layout = TextureAtlasLayout::new_empty(UVec2::new({columns} * {frame_width}, {rows} * {frame_height}));"
    )
    print()
    print(f"    let crop = URect::new({crop['min_x']}, {crop['min_y']}, {crop['max_x']}, {crop['max_y']});")
    print()
    print(f"    for row in 0..{rows} {{")
    print(f"        for col in 0..{columns} {{")
    print(f"            let origin = UVec2::new(col * {frame_width}, row * {frame_height});")
    print("            layout.add_texture(URect {")
    print("                min: origin + crop.min,")
    print("                max: origin + crop.max,")
    print("            });")
    print("        }")
    print("    }")
    print()
    print("    texture_atlas_layouts.add(layout)")
    print("}")
    print("```")
    print()


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Find sprite bounding boxes in a PNG spritesheet via connected-component analysis."
    )
    parser.add_argument("png", help="Path to the PNG spritesheet")
    parser.add_argument(
        "--scan-width",
        type=int,
        default=0,
        help="Only scan the leftmost N pixels (0 = full width)",
    )
    parser.add_argument(
        "--scan-height",
        type=int,
        default=0,
        help="Only scan the topmost N pixels (0 = full height)",
    )
    parser.add_argument(
        "--min-pixels",
        type=int,
        default=5,
        help="Ignore components with fewer than N opaque pixels (default: 5)",
    )
    parser.add_argument(
        "--frame-width",
        type=int,
        default=0,
        help="Analyse as fixed-size frames with this width",
    )
    parser.add_argument(
        "--frame-height",
        type=int,
        default=0,
        help="Analyse as fixed-size frames with this height",
    )
    parser.add_argument(
        "--stable",
        action="store_true",
        help="Report a stable crop box shared by all populated frames",
    )
    parser.add_argument(
        "--per-row",
        action="store_true",
        help="With frame analysis, also report stable crop boxes per row",
    )
    parser.add_argument(
        "--emit-bevy-helper",
        default="",
        help="With --stable and frame analysis, emit a Bevy helper function with this name",
    )
    args = parser.parse_args()

    try:
        width, height, rows = decode_png(args.png)
    except (ValueError, FileNotFoundError) as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    scan_w = args.scan_width or width
    scan_h = args.scan_height or height

    print(f"Image : {width}×{height} px")
    print(f"Scan  : {scan_w}×{scan_h} px")
    print()

    if bool(args.frame_width) != bool(args.frame_height):
        print(
            "Error: --frame-width and --frame-height must be provided together",
            file=sys.stderr,
        )
        sys.exit(1)

    if args.frame_width and args.frame_height:
        frames = find_frame_components(
            rows,
            min(scan_w, width),
            min(scan_h, height),
            args.frame_width,
            args.frame_height,
            args.min_pixels,
        )

        if not frames:
            print("No populated frames found.")
            return

        print_frame_components(frames)
        print()

        if args.stable:
            crop = stable_crop_box(frames)
            if crop is not None:
                print_stable_crop("Stable crop box", crop)
                if args.emit_bevy_helper:
                    print_bevy_helper(
                        args.emit_bevy_helper,
                        args.frame_width,
                        args.frame_height,
                        min(scan_w, width),
                        min(scan_h, height),
                        crop,
                    )

        if args.per_row:
            row_indices = sorted({frame["frame_row"] for frame in frames})
            for row_index in row_indices:
                row_frames = [frame for frame in frames if frame["frame_row"] == row_index]
                crop = stable_crop_box(row_frames)
                if crop is not None:
                    print_stable_crop(f"Row {row_index} stable crop box", crop)

        return

    opaque = find_opaque_pixels(rows, scan_w, scan_h)
    components = find_components(opaque, args.min_pixels)

    if not components:
        print("No sprite components found.")
        return

    print(f"Found {len(components)} sprite(s):\n")
    print(f"  {'#':>3}  {'x range':>14}  {'y range':>14}  {'size':>12}  {'pixels':>8}  Bevy Rect::new()")
    print(f"  {'-'*3}  {'-'*14}  {'-'*14}  {'-'*12}  {'-'*8}  {'-'*32}")

    for i, c in enumerate(components, 1):
        size_str = f"{c['width']}×{c['height']}"
        x_range = f"{c['min_x']}–{c['max_x']}"
        y_range = f"{c['min_y']}–{c['max_y']}"
        print(
            f"  {i:>3}  {x_range:>14}  {y_range:>14}  {size_str:>12}  "
            f"{c['pixels']:>8}  {component_to_bevy_rect(c)}"
        )


if __name__ == "__main__":
    main()
