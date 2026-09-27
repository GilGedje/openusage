#!/usr/bin/env python3
"""Builds the icon assets from the Claude mark (a square PNG), with no image libraries:

- crates/usage-core/src/claude_mark.rgba — the mark as raw RGBA, MARK_SIZE x MARK_SIZE, composited
  into the tray icon at runtime (the ring around it shows budget used).
- crates/tray/icons/*.png, icon.ico — app icons: the mark inside a budget ring on a dark tile.

    python3 tools/icons/make_icons.py <claude-mark.png>

Needs macOS `sips` (for resizing); run it after changing the mark, then commit the outputs.
"""

import math
import pathlib
import struct
import subprocess
import sys
import tempfile
import zlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
MARK_SIZE = 20  # keep in sync with usage-core/src/gauge.rs


def resized(src: pathlib.Path, size: int) -> tuple[int, int, bytes]:
    """RGBA pixels of `src` resized to `size` (via sips, which outputs 8-bit RGBA PNG)."""
    with tempfile.TemporaryDirectory() as tmp:
        out = pathlib.Path(tmp) / "m.png"
        subprocess.run(["sips", "-Z", str(size), str(src), "--out", str(out)], check=True, capture_output=True)
        return decode_png(out.read_bytes())


def decode_png(data: bytes) -> tuple[int, int, bytes]:
    assert data[:8] == b"\x89PNG\r\n\x1a\n", "not a PNG"
    pos, idat = 8, b""
    w = h = 0
    ctype = 6
    while pos < len(data):
        n, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + n]
        if kind == b"IHDR":
            w, h, depth, ctype = struct.unpack(">IIBB", body[:10])
            assert depth == 8 and ctype in (2, 6), f"unsupported PNG (depth {depth}, color type {ctype})"
        elif kind == b"IDAT":
            idat += body
        pos += 12 + n
    bpp = 4 if ctype == 6 else 3
    raw = zlib.decompress(idat)
    stride = w * bpp
    out = bytearray()
    prev = bytearray(stride)
    i = 0
    for _ in range(h):
        f = raw[i]
        line = bytearray(raw[i + 1:i + 1 + stride])
        i += 1 + stride
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0
            b = prev[x]
            c = prev[x - bpp] if x >= bpp else 0
            if f == 1:
                line[x] = (line[x] + a) & 255
            elif f == 2:
                line[x] = (line[x] + b) & 255
            elif f == 3:
                line[x] = (line[x] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        prev = line
        if bpp == 3:
            for x in range(w):
                out += line[x * 3:x * 3 + 3] + b"\xff"
        else:
            out += line
    return w, h, bytes(out)


def encode_png(w: int, h: int, rgba: bytes) -> bytes:
    raw = b"".join(b"\x00" + rgba[y * w * 4:(y + 1) * w * 4] for y in range(h))

    def chunk(kind: bytes, body: bytes) -> bytes:
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)

    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def app_icon(mark: pathlib.Path, n: int, used: float = 0.63) -> bytes:
    """Dark rounded tile, a budget ring (blue), the mark in the middle."""
    px = bytearray(n * n * 4)
    ss = 4
    for y in range(n):
        for x in range(n):
            acc = [0, 0, 0, 0]
            for sy in range(ss):
                for sx in range(ss):
                    fx = (x + (sx + 0.5) / ss) / n - 0.5
                    fy = (y + (sy + 0.5) / ss) / n - 0.5
                    r = 0.22
                    ax, ay = abs(fx), abs(fy)
                    inside = (ax <= 0.5 and ay <= 0.5) if (ax <= 0.5 - r or ay <= 0.5 - r) else math.hypot(ax - (0.5 - r), ay - (0.5 - r)) <= r
                    col = (28, 28, 30, 255) if inside else (0, 0, 0, 0)
                    d = math.hypot(fx, fy)
                    if 0.31 <= d <= 0.37:
                        turn = (math.degrees(math.atan2(fx, -fy)) + 360) % 360 / 360
                        col = (10, 132, 255, 255) if turn <= used else (72, 72, 76, 255)
                    for i in range(4):
                        acc[i] += col[i]
            px[(y * n + x) * 4:(y * n + x) * 4 + 4] = bytes(v // (ss * ss) for v in acc)
    ms = round(n * 0.46)
    mw, mh, m = resized(mark, ms)
    ox, oy = (n - mw) // 2, (n - mh) // 2
    for y in range(mh):
        for x in range(mw):
            s = m[(y * mw + x) * 4:(y * mw + x) * 4 + 4]
            a = s[3] / 255
            i = ((y + oy) * n + (x + ox)) * 4
            for c in range(3):
                px[i + c] = round(s[c] * a + px[i + c] * (1 - a))
            px[i + 3] = max(px[i + 3], s[3])
    return encode_png(n, n, bytes(px))


def main() -> None:
    mark = pathlib.Path(sys.argv[1])
    w, h, rgba = resized(mark, MARK_SIZE)
    assert (w, h) == (MARK_SIZE, MARK_SIZE), f"mark must be square (got {w}x{h})"
    (ROOT / "crates/usage-core/src/claude_mark.rgba").write_bytes(rgba)

    icons = ROOT / "crates/tray/icons"
    for n, name in [(32, "32x32.png"), (128, "128x128.png"), (256, "128x128@2x.png"), (512, "icon.png")]:
        (icons / name).write_bytes(app_icon(mark, n))
    p256 = (icons / "128x128@2x.png").read_bytes()
    ico = struct.pack("<HHH", 0, 1, 1) + struct.pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(p256), 22) + p256
    (icons / "icon.ico").write_bytes(ico)
    print("mark", MARK_SIZE, "px and app icons written")


if __name__ == "__main__":
    main()
