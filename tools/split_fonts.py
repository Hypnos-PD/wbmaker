#!/usr/bin/env python3
"""把标题/署名字体切成 unicode-range 小块（按需加载，参照 shadowverse-wb.com 的做法）。

输出:
  web/fonts/chunks/<key>/<idx:03d>.otf   每块按码点范围切片
  web/font-chunks.json                   码点范围 -> 块文件 清单

分块策略：非 CJK 区按块整包（拉丁/希腊/标点/假名/全角/谚文），
CJK 统一表意区按 512 字一块。
"""
import json
import os
import sys
from fontTools.ttLib import TTFont
from fontTools.subset import Subsetter, Options

ROOT = os.path.join(os.path.dirname(__file__), "..")
WEB = os.path.join(ROOT, "web")

# 字体 -> (源文件, key)；sig- 前缀为署名字体（Noto 常规体各语言）
FONTS = {
    "chs": "assets/fonts/arweibeigbpro_bd.otf",
    "cht": "assets/fonts/DFT_W7-930.ttf",
    "jpn": "assets/fonts/MOC-KaiminTsuki-B.otf",
    "kor": "assets/fonts/NanumGothic-ExtraBold.ttf",
    "eng": "assets/fonts/MOC-KaiminTsuki-B.otf",
    "chs-sig": "assets/fonts/NotoSansSC-Regular.otf",
    "cht-sig": "assets/fonts/NotoSansTC-Regular.otf",
    "jpn-sig": "assets/fonts/NotoSansJP-Regular.otf",
    "kor-sig": "assets/fonts/NotoSansKR-Regular.otf",
    "eng-sig": "assets/fonts/NotoSansSC-Regular.otf",
}

# 非 CJK 的整块区间（按区块切）
BASE_BLOCKS = [
    (0x0000, 0x02FF),   # 拉丁/希腊/西里尔等
    (0x0370, 0x03FF),   # 希腊
    (0x2000, 0x206F),   # 常规标点（※ … 等）
    (0x20A0, 0x20CF),   # 货币
    (0x2100, 0x214F),   # 字母式符号
    (0x2150, 0x218F),   # 数字形式（罗马数字）
    (0x2190, 0x22FF),   # 箭头/数学
    (0x25A0, 0x25FF),   # 几何图形
    (0x3000, 0x303F),   # CJK 标点
    (0x3040, 0x30FF),   # 假名
    (0x3100, 0x312F),   # 注音
    (0xAC00, 0xD7A3),   # 谚文（仅 kor 需要，其它字体空块跳过）
    (0xFF00, 0xFFEF),   # 全角
]
CJK_BLOCK = (0x4E00, 0x9FFF)
CJK_BUCKET = 128


def font_codepoints(path):
    f = TTFont(path)
    cmap = f.getBestCmap()
    cps = set(cmap.keys())
    cps.discard(0)
    return sorted(cps)


def subset_to(path, cps, out):
    opts = Options()
    # wasm 渲染只用到 cmap/轮廓/advance，不需要 GSUB/GPOS/hinting，
    # 裁掉可让每块从 ~400KB 降到几十 KB
    opts.layout_features = []
    opts.hinting = False
    opts.desubroutinize = True  # CFF 子程序内联，避免每块携带共享子程序池
    opts.notdef_glyph = True
    opts.notdef_outline = True
    opts.name_IDs = ["*"]
    f = TTFont(path)
    ss = Subsetter(options=opts)
    ss.populate(unicodes=cps)
    ss.subset(f)
    f.save(out)
    f.close()


def main():
    manifest = {}
    for key, src in FONTS.items():
        src_path = os.path.join(ROOT, src)
        if not os.path.exists(src_path):
            print(f"skip {key}: {src} 不存在（先复制原字体到 assets/fonts/）")
            continue
        cps = font_codepoints(src_path)
        entries = []
        idx = 0
        # 非 CJK 块
        for lo, hi in BASE_BLOCKS:
            block = [c for c in cps if lo <= c <= hi]
            if not block:
                continue
            entries.append((block, f"{key}/{idx:03d}.otf"))
            idx += 1
        # CJK 512 字一块
        cjk = [c for c in cps if CJK_BLOCK[0] <= c <= CJK_BLOCK[1]]
        for i in range(0, len(cjk), CJK_BUCKET):
            entries.append((cjk[i:i + CJK_BUCKET], f"{key}/{idx:03d}.otf"))
            idx += 1
        # 兜底块：其余所有
        rest = [c for c in cps if not (
            (BASE_BLOCKS and any(lo <= c <= hi for lo, hi in BASE_BLOCKS))
            or CJK_BLOCK[0] <= c <= CJK_BLOCK[1])]
        if rest:
            entries.append((rest, f"{key}/{idx:03d}.otf"))
            idx += 1

        out_dir = os.path.join(WEB, "fonts", "chunks", key)
        os.makedirs(out_dir, exist_ok=True)
        files = []
        total = 0
        for block, rel in entries:
            out = os.path.join(WEB, "fonts", "chunks", rel)
            subset_to(src_path, block, out)
            total += os.path.getsize(out)
            files.append({"file": "fonts/chunks/" + rel, "ranges": compact(block)})
        manifest[key] = {"files": files}
        print(f"{key}: {len(files)} 块, 共 {total/1024:.0f} KB")

    with open(os.path.join(WEB, "font-chunks.json"), "w", encoding="utf-8") as f:
        json.dump(manifest, f, ensure_ascii=False, separators=(",", ":"))
    print("manifest -> web/font-chunks.json")


def compact(sorted_cps):
    ranges = []
    start = prev = None
    for c in sorted_cps:
        if start is None:
            start = prev = c
        elif c == prev + 1:
            prev = c
        else:
            ranges.append([start, prev])
            start = prev = c
    if start is not None:
        ranges.append([start, prev])
    return ranges


if __name__ == "__main__":
    main()
