#!/usr/bin/env python3
"""Prepare sv-byd-diy assets for wbm's DIY card renderer.

Reads from the sibling sv-byd-diy repo and writes resized/compressed copies
into wbmaker/assets/diy/ (embedded into the wasm via include_bytes!).

Everything is pre-sized to the 1920x1080 design space used by the DIY tool,
so the wasm never resizes at runtime (only 1x export is supported).
"""
import os
import sys
from PIL import Image

SRC = os.path.join(os.path.dirname(__file__), "..", "..", "sv-byd-diy")
DST = os.path.join(os.path.dirname(__file__), "..", "assets", "diy")

CLASSES = ["forestcraft", "swordcraft", "runecraft", "dragoncraft",
           "abysscraft", "havencraft", "portalcraft", "neutral"]
# CardDetail TextureRect 2648x1843 @ scale 0.4
DETAIL_BG_SIZE = (1059, 737)
# detail_spit 2423x3 @ scale 0.4 * 1.03
SPLIT_SIZE = (999, 2)
# TitleBg 1200x675 @ 1.6
TITLE_BG_SIZE = (1920, 1080)
# title_bottom 1200x697 @ 1.6
TITLE_BOTTOM_SIZE = (1920, 1115)
# Backgrounds cover-fit 1920x1080
BG_SIZE = (1920, 1080)
# Built-in crest icons (incl. the default luna crest)
CREST_SIZE = (64, 64)


def main():
    out = []
    # --- class backgrounds (jpg) ---
    for cls in CLASSES:
        for gen in (1, 2):
            src = f"{SRC}/img/background/{cls}-{gen}.jpg"
            dst = f"{DST}/backgrounds/{cls}-{gen}.jpg"
            out.append((src, dst, BG_SIZE, Image.LANCZOS, "JPEG"))

    # --- title class icons (keep native) ---
    for cls in CLASSES:
        out.append((f"{SRC}/img/title_class/{cls}.png",
                    f"{DST}/title_class/{cls}.png", None, None, "PNG"))

    # --- effect images ---
    out += [
        (f"{SRC}/img/effect/title_bg.png", f"{DST}/effect/title_bg.png",
         TITLE_BG_SIZE, Image.LANCZOS, "PNG"),
        (f"{SRC}/img/effect/title_bottom.png", f"{DST}/effect/title_bottom.png",
         TITLE_BOTTOM_SIZE, Image.LANCZOS, "PNG"),
        (f"{SRC}/img/effect/card_detail_background.png",
         f"{DST}/effect/card_detail_background.png", DETAIL_BG_SIZE, Image.LANCZOS, "PNG"),
        (f"{SRC}/img/effect/detail_spit.png", f"{DST}/effect/detail_spit.png",
         SPLIT_SIZE, Image.LANCZOS, "PNG"),
        # Section banners: stretched to the text block at runtime; keep native.
        (f"{SRC}/img/effect/evolve.png", f"{DST}/effect/evolve.png", None, None, "PNG"),
        (f"{SRC}/img/effect/super_evolve.png", f"{DST}/effect/super_evolve.png", None, None, "PNG"),
        (f"{SRC}/img/effect/detail_crest.png", f"{DST}/effect/detail_crest.png", None, None, "PNG"),
        # Crest name banners (449x48 native, stretched to 618x67 at runtime).
        (f"{SRC}/img/effect/Crest.png", f"{DST}/effect/Crest.png", None, None, "PNG"),
        (f"{SRC}/img/effect/Faith.png", f"{DST}/effect/Faith.png", None, None, "PNG"),
        (f"{SRC}/img/effect/Accelerate.png", f"{DST}/effect/Accelerate.png", None, None, "PNG"),
        (f"{SRC}/img/effect/Crystallize.png", f"{DST}/effect/Crystallize.png", None, None, "PNG"),
    ]

    # --- built-in crest icons + default crest (luna) ---
    builtin = sorted(os.listdir(f"{SRC}/img/build_in_crest"))
    builtin = [b for b in builtin if b.endswith(".png")]
    for name in builtin:
        out.append((f"{SRC}/img/build_in_crest/{name}",
                    f"{DST}/crests/{name}", CREST_SIZE, Image.LANCZOS, "PNG"))
    out.append((f"{SRC}/img/test/luna_crest.jpg",
                f"{DST}/crests/default_crest.png", CREST_SIZE, Image.LANCZOS, "PNG"))

    total = 0
    for src, dst, size, filt, fmt in out:
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        im = Image.open(src)
        if im.mode not in ("RGB", "RGBA"):
            im = im.convert("RGBA" if fmt == "PNG" else "RGB")
        if size:
            im = im.resize(size, filt)
        if fmt == "JPEG":
            im.save(dst, "JPEG", quality=85)
        else:
            im.save(dst, "PNG", optimize=True)
        total += os.path.getsize(dst)
    print(f"wrote {len(out)} assets -> {DST} ({total/1e6:.2f} MB)")


if __name__ == "__main__":
    main()
