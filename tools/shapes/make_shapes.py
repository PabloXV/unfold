"""Traces Noto animal emoji into colored triangulated shapes, for the removed mystery mode.
usage: uv run --with pillow --with numpy --with opencv-python-headless tools/shapes/make_shapes.py"""

import base64
import io
import json
import math
import urllib.request
from pathlib import Path

import cv2
import numpy as np
from PIL import Image, ImageDraw

LEVEL_CORNERS = [5, 7, 9, 12, 16]

ANIMALS = [
    ("fox", "1f98a", []),
    ("wolf", "1f43a", []),
    ("panda", "1f43c", []),
    ("elephant", "1f418", []),
    ("mammoth", "1f9a3", []),
    ("giraffe", "1f992", []),
    ("turtle", "1f422", ["tortoise"]),
    ("rabbit", "1f407", ["bunny", "hare"]),
    ("cat", "1f408", ["kitten", "kitty"]),
    ("dog", "1f415", ["puppy"]),
    ("horse", "1f40e", ["pony"]),
    ("cow", "1f404", []),
    ("ox", "1f403", ["bull"]),
    ("bison", "1f9ac", ["buffalo"]),
    ("pig", "1f416", ["hog", "boar"]),
    ("sheep", "1f411", ["ewe", "lamb"]),
    ("ram", "1f40f", []),
    ("goat", "1f410", []),
    ("camel", "1f42a", ["dromedary"]),
    ("kangaroo", "1f998", []),
    ("hedgehog", "1f994", []),
    ("squirrel", "1f43f", ["chipmunk"]),
    ("bat", "1f987", []),
    ("rooster", "1f413", ["chicken", "hen"]),
    ("duck", "1f986", []),
    ("swan", "1f9a2", []),
    ("flamingo", "1f9a9", []),
    ("peacock", "1f99a", []),
    ("eagle", "1f985", []),
    ("dove", "1f54a", ["pigeon"]),
    ("owl", "1f989", []),
    ("penguin", "1f427", []),
    ("parrot", "1f99c", []),
    ("dodo", "1f9a4", []),
    ("crocodile", "1f40a", ["alligator", "croc"]),
    ("lizard", "1f98e", ["gecko"]),
    ("snake", "1f40d", []),
    ("t-rex", "1f996", ["tyrannosaurus", "dinosaur"]),
    ("sauropod", "1f995", ["brontosaurus", "diplodocus", "brachiosaurus"]),
    ("whale", "1f40b", []),
    ("dolphin", "1f42c", []),
    ("shark", "1f988", []),
    ("fish", "1f41f", []),
    ("tropical fish", "1f420", []),
    ("octopus", "1f419", []),
    ("jellyfish", "1fabc", []),
    ("crab", "1f980", []),
    ("lobster", "1f99e", []),
    ("snail", "1f40c", []),
    ("butterfly", "1f98b", []),
    ("bee", "1f41d", ["honeybee", "bumblebee"]),
    ("ladybug", "1f41e", ["ladybird"]),
    ("scorpion", "1f982", []),
    ("monkey", "1f412", []),
    ("gorilla", "1f98d", ["ape"]),
    ("rhinoceros", "1f98f", ["rhino"]),
    ("hippopotamus", "1f99b", ["hippo"]),
    ("tiger", "1f405", []),
    ("leopard", "1f406", ["cheetah", "jaguar"]),
    ("zebra", "1f993", []),
    ("deer", "1f98c", ["stag", "reindeer"]),
    ("mouse", "1f401", ["rat"]),
    ("otter", "1f9a6", []),
    ("sloth", "1f9a5", []),
    ("skunk", "1f9a8", []),
    ("badger", "1f9a1", []),
    ("seal", "1f9ad", ["sea lion"]),
]

SKIP = {"sheep", "ram", "sloth", "dog", "octopus", "jellyfish", "crocodile"}
DECOYS = ["bear", "lion", "frog", "koala", "llama", "moose", "raccoon", "beaver", "ostrich", "toucan", "seahorse", "spider", "ant", "worm"]

SIZE = 512
HERE = Path(__file__).parent
CACHE = HERE / ".cache"
OUT = HERE.parent.parent / "web/src/lib/mystery/shapes.json"


def fetch(code: str) -> Image.Image:
    path = CACHE / f"{code}.png"
    if not path.exists():
        CACHE.mkdir(exist_ok=True)
        url = f"https://fonts.gstatic.com/s/e/notoemoji/latest/{code}/512.png"
        path.write_bytes(urllib.request.urlopen(url).read())
    img = Image.open(path).convert("RGBA")
    return img if img.size == (SIZE, SIZE) else img.resize((SIZE, SIZE))


def cross(o, a, b):
    return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])


def signed_area(p):
    """Positive counterclockwise, in image coordinates (y down) that's clockwise on screen."""
    return sum(cross((0, 0), p[i], p[(i + 1) % len(p)]) for i in range(len(p))) / 2


def segments_cross(a, b, c, d):
    return cross(a, b, c) * cross(a, b, d) < 0 and cross(c, d, a) * cross(c, d, b) < 0


def outline(img: Image.Image):
    """The biggest outline, and the share of the opaque area it encloses."""
    alpha = (np.array(img)[:, :, 3] > 128).astype(np.uint8) * 255
    contours, _ = cv2.findContours(alpha, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)
    c = max(contours, key=cv2.contourArea)
    share = cv2.contourArea(c) / max(1, sum(cv2.contourArea(x) for x in contours))
    c = cv2.approxPolyDP(c, 1.0, True)[:, 0, :]
    pts = [(float(x), float(y)) for x, y in c]
    if signed_area(pts) < 0:
        pts.reverse()
    return pts, share


def simplify(pts, k):
    """Visvalingam-Whyatt down to k corners, skipping removals that would self-intersect."""
    pts = list(pts)
    while len(pts) > k:
        n = len(pts)
        order = sorted(range(n), key=lambda i: abs(cross(pts[i - 1], pts[i], pts[(i + 1) % n])))
        for i in order:
            a, b = pts[i - 1], pts[(i + 1) % n]
            prev, nxt = (i - 1) % n, (i + 1) % n
            edges = [(pts[j], pts[(j + 1) % n]) for j in range(n) if j not in (prev, i) and (j + 1) % n not in (i, prev)]
            if not any(segments_cross(a, b, c, d) for c, d in edges if c not in (a, b) and d not in (a, b)):
                break
        pts.pop(i)
    if signed_area(pts) < 0:
        pts.reverse()
    return pts


def inside(p, poly):
    x, y, hit = p[0], p[1], False
    for i in range(len(poly)):
        (x1, y1), (x2, y2) = poly[i], poly[(i + 1) % len(poly)]
        if (y1 > y) != (y2 > y) and x < x1 + (y - y1) * (x2 - x1) / (y2 - y1):
            hit = not hit
    return hit


def min_angle(a, b, c):
    def angle(p, q, r):
        v, w = (q[0] - p[0], q[1] - p[1]), (r[0] - p[0], r[1] - p[1])
        d = math.hypot(*v) * math.hypot(*w)
        return math.acos(max(-1.0, min(1.0, (v[0] * w[0] + v[1] * w[1]) / d))) if d else 0.0
    return min(angle(a, b, c), angle(b, c, a), angle(c, a, b))


def triangulate(poly):
    """The triangulation whose thinnest triangle is fattest, by dynamic programming."""
    n = len(poly)

    def diagonal(i, j):
        if (j - i) % n in (1, n - 1):
            return True
        a, b = poly[i], poly[j]
        for e in range(n):
            if e in (i, j) or (e + 1) % n in (i, j):
                continue
            if segments_cross(a, b, poly[e], poly[(e + 1) % n]):
                return False
        return inside(((a[0] + b[0]) / 2, (a[1] + b[1]) / 2), poly)

    ok = [[diagonal(i, j) for j in range(n)] for i in range(n)]
    best, pick = {}, {}
    for span in range(1, n):
        for i in range(n - span):
            j = i + span
            best[i, j] = math.inf if span == 1 else -1.0
            for m in range(i + 1, j):
                if span > 1 and ok[i][m] and ok[m][j] and cross(poly[i], poly[m], poly[j]) > 0:
                    score = min(best[i, m], best[m, j], min_angle(poly[i], poly[m], poly[j]))
                    if score > best[i, j]:
                        best[i, j], pick[i, j] = score, m
    if best[0, n - 1] < 0:
        raise ValueError("no triangulation")
    tris = []

    def walk(i, j):
        if j - i >= 2:
            m = pick[i, j]
            tris.append((i, m, j))
            walk(i, m)
            walk(m, j)

    walk(0, n - 1)
    return tris, math.degrees(best[0, n - 1])


def colors(img, poly, tris):
    rgba = np.array(img)
    opaque = rgba[:, :, 3] > 128
    out = []
    for t in tris:
        mask = Image.new("L", img.size, 0)
        ImageDraw.Draw(mask).polygon([poly[i] for i in t], fill=255)
        m = (np.array(mask) > 0) & opaque
        rgb = rgba[:, :, :3][m].mean(axis=0) if m.any() else np.array([200, 200, 200])
        out.append("#%02x%02x%02x" % tuple(int(v) for v in rgb))
    return out


def level(img, full, k):
    poly = simplify(full, k)
    tris, thinnest = triangulate(poly)
    return {
        "corners": [round(v, 2) for x, y in poly for v in (x, SIZE - y)],
        "triangles": [v for t in tris for v in t],
        "colors": colors(img, poly, tris),
        "thinnest": round(thinnest),
    }


def preview_svg(lv):
    xy = lv["corners"]
    pts = [(xy[i], SIZE - xy[i + 1]) for i in range(0, len(xy), 2)]
    t = lv["triangles"]
    polys = "".join(
        f'<polygon points="{" ".join(f"{pts[v][0]:.0f},{pts[v][1]:.0f}" for v in t[i:i + 3])}" fill="{c}"/>'
        for i, c in zip(range(0, len(t), 3), lv["colors"])
    )
    return f'<svg viewBox="0 0 {SIZE} {SIZE}">{polys}</svg>'


shapes, rows = [], []
decoys = list(DECOYS)
for name, code, aliases in ANIMALS:
    if name in SKIP:
        decoys += [name, *aliases]
        continue
    img = fetch(code)
    full, share = outline(img)
    levels = [level(img, full, k) for k in LEVEL_CORNERS]
    shapes.append({"name": name, "emoji": chr(int(code, 16)), "aliases": aliases, "levels": levels})
    note = f"outline holds {share:.0%} of the emoji" if share < 0.9 else ""
    thinnest = min(lv["thinnest"] for lv in levels)
    print(f"{name:14} {len(full):4} pts  thinnest {thinnest:2}°  {note}")
    buf = io.BytesIO()
    img.resize((96, 96)).save(buf, "PNG")
    cells = "".join(f"<td>{preview_svg(lv)}</td>" for lv in levels)
    emoji = f'<td><img src="data:image/png;base64,{base64.b64encode(buf.getvalue()).decode()}"></td>'
    rows.append(f"<tr><th>{name}<small>{note}</small></th>{cells}{emoji}</tr>")

OUT.parent.mkdir(parents=True, exist_ok=True)
OUT.write_text(json.dumps({"levelCorners": LEVEL_CORNERS, "shapes": shapes, "decoys": sorted(set(decoys))}, separators=(",", ":")))
head = "".join(f"<th>{k}</th>" for k in LEVEL_CORNERS)
(HERE / "preview.html").write_text(
    "<!doctype html><meta charset=utf-8><title>Shapes</title><style>"
    "body{font:14px system-ui;background:#f4f0e8;margin:16px}td{background:#fffdf8;border-radius:8px;padding:4px}"
    "svg,img{width:96px;height:96px;display:block}th{text-align:left;font-weight:500;padding-right:8px}"
    "small{display:block;color:#b44;font-weight:400}</style>"
    f"<table><tr><th></th>{head}<th>emoji</th></tr>{''.join(rows)}</table>"
)
print(f"{len(shapes)} shapes -> {OUT.relative_to(HERE.parent.parent)} ({OUT.stat().st_size // 1024} KB)")
