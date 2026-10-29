"""Общий стиль учебных SVG-схем; используется diagrams_*.py.

Координаты заданы явно, чтобы схема оставалась редактируемой без
специального графического редактора. SVG не зависит от внешних шрифтов,
скриптов или изображений. Текст сохраняется текстом, а не контурами.
"""

from html import escape
from pathlib import Path
import textwrap

ROOT = Path(__file__).resolve().parent.parent
IMAGES = ROOT / "src" / "images"
PALETTE = {
    "blue": ("#e8f1ff", "#2864b4"),
    "green": ("#e5f4ed", "#287354"),
    "orange": ("#fff0dc", "#a45b12"),
    "red": ("#fce8e6", "#a93832"),
    "purple": ("#f1eafa", "#7552a3"),
    "gray": ("#eef1f5", "#536477"),
}


class Diagram:
    def __init__(self, title, description, height=560, width=1000, tag="RUST · СХЕМА"):
        self.width, self.height = width, height
        self.parts = [
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
            f'viewBox="0 0 {width} {height}" role="img" aria-labelledby="title desc">',
            f'<title id="title">{escape(title)}</title>',
            f'<desc id="desc">{escape(description)}</desc>',
            '<defs>' + ''.join(
                f'<marker id="arrow-{tone}" viewBox="0 0 10 10" refX="9" refY="5" '
                'markerWidth="8" markerHeight="8" orient="auto-start-reverse">'
                f'<path d="M 0 0 L 10 5 L 0 10 z" fill="{colors[1]}"/></marker>'
                for tone, colors in PALETTE.items()
            ) + '</defs>',
            '<style>text{font-family:"DejaVu Sans",sans-serif;fill:#172b41}'
            '.tag{font-size:15px;font-weight:700;letter-spacing:2px;fill:#536477}'
            '.title{font-size:30px;font-weight:700}'
            '.label{font-size:23px;font-weight:700}'
            '.body{font-size:21px}.small{font-size:18px;fill:#455c73}'
            '.code{font-family:"DejaVu Sans Mono",monospace;font-size:21px}'
            '.note{font-size:20px;fill:#455c73}</style>',
            f'<rect width="{width}" height="{height}" rx="18" fill="#f8fafc"/>',
            f'<rect x="0" y="0" width="8" height="{height}" rx="4" fill="#2864b4"/>',
        ]
        self.text(36, 34, tag, "tag")
        self.text(36, 77, title, "title")

    def text(self, x, y, content, cls="body", anchor="start", fill=None, line_height=29):
        lines = content.split("\n") if isinstance(content, str) else content
        color = f' style="fill:{fill}"' if fill else ""
        for i, line in enumerate(lines):
            self.parts.append(
                f'<text x="{x}" y="{y + i * line_height}" class="{cls}" '
                f'text-anchor="{anchor}"{color}>{escape(str(line))}</text>'
            )

    def rect(self, x, y, w, h, tone="blue", radius=12, dashed=False):
        bg, stroke = PALETTE[tone]
        dash = ' stroke-dasharray="7 5"' if dashed else ""
        self.parts.append(
            f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{radius}" '
            f'fill="{bg}" stroke="{stroke}" stroke-width="2"{dash}/>'
        )

    def box(self, x, y, w, h, title, lines=(), tone="blue"):
        self.rect(x, y, w, h, tone)
        self.text(x + 18, y + 34, title, "label", fill=PALETTE[tone][1])
        if lines:
            self.text(x + 18, y + 67, lines, "body")

    def arrow(self, x1, y1, x2, y2, label=None, tone="gray", dashed=False):
        color = PALETTE[tone][1]
        dash = ' stroke-dasharray="7 5"' if dashed else ""
        self.parts.append(
            f'<path d="M {x1} {y1} L {x2} {y2}" fill="none" stroke="{color}" '
            f'stroke-width="2.5" marker-end="url(#arrow-{tone})"{dash}/>'
        )
        if label:
            self.text((x1 + x2) / 2, (y1 + y2) / 2 - 12, label, "small", "middle")

    def path(self, d, tone="gray", arrow=True, dashed=False):
        marker = f' marker-end="url(#arrow-{tone})"' if arrow else ""
        dash = ' stroke-dasharray="7 5"' if dashed else ""
        self.parts.append(
            f'<path d="{escape(d)}" fill="none" stroke="{PALETTE[tone][1]}" '
            f'stroke-width="2.5"{marker}{dash}/>'
        )

    def note(self, content, y=None):
        self.text(36, self.height - 37 if y is None else y, content, "note")

    def save(self, filename):
        IMAGES.mkdir(exist_ok=True)
        path = IMAGES / filename
        path.write_text("\n".join(self.parts + ["</svg>", ""]), encoding="utf-8")
        return path


def wrap(text, width=32):
    return textwrap.wrap(text, width=width, break_long_words=False, break_on_hyphens=False)
