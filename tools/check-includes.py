#!/usr/bin/env python3
"""Проверяет ссылки {{#include файл:якорь}} в исходниках книги.

mdBook 0.5 на пропущенный файл печатает ERROR, но завершается с кодом 0, а
на пропущенный якорь молча вставляет пустой блок кода. Переименованный
якорь в коде главы тихо опустошил бы листинг в книге, поэтому CI
проверяет каждую ссылку отдельно.

Заодно проверяются ссылки на страницы книги — `[текст](путь.md)` и
`[текст](путь.md#якорь)`: на несуществующий файл mdBook тоже не ругается, а в
собранной книге такая ссылка ведёт на 404. Так случается, когда заглушка
главы `chNN.md` становится каталогом `chNN/`, а ссылки на неё из других глав
остаются прежними. Якорь после `#` здесь не проверяется, только файл:
якоря проверяет tools/check-links.py по собранной книге.

Также проверяются локальные Markdown-изображения: наличие файла,
непустой альтернативный текст и корректный XML у SVG. Это не позволяет
опубликовать главу с потерянным или повреждённым рисунком.

Запуск из корня репозитория: python3 tools/check-includes.py
"""

import re
import sys
from pathlib import Path
from urllib.parse import unquote
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "src"
INCLUDE = re.compile(r"\{\{#include\s+([^}\s]+)\s*\}\}")
# Номер строки или диапазон (`файл:10`, `файл:10:20`, `файл::20`) — не якорь.
LINES = re.compile(r"^\d*(:\d*)?$")
# Относительная ссылка на страницу; внешние адреса (с «:») и чистые якоря
# («#...») сюда не попадают.
PAGE = re.compile(r"\]\(([^()\s:#]+\.md)(?:#[^()\s]*)?\)")
# В alt могут быть скобки Rust: «&a[1..3]». Макросы vec![...] и #![...]
# изображениями не являются; вложенные скобки в alt должны быть парными.
IMAGE = re.compile(r"(?<![\w#`\\])!\[((?:[^\[\]\n]|\[[^\[\]\n]*\])*)\]\(([^()\s]+)\)")


def anchors(text: str) -> tuple[set[str], set[str]]:
    begin = set(re.findall(r"ANCHOR:\s*([\w-]+)", text))
    end = set(re.findall(r"ANCHOR_END:\s*([\w-]+)", text))
    return begin, end


def main() -> int:
    errors = []
    checked = 0
    pages = 0
    images = 0
    parsed_svg = set()
    for md in sorted(SRC.rglob("*.md")):
        for lineno, line in enumerate(md.read_text(encoding="utf-8").splitlines(), 1):
            for alt, target in IMAGE.findall(line):
                if re.match(r"[a-zA-Z][a-zA-Z0-9+.-]*:", target) or target.startswith("//"):
                    continue
                images += 1
                where = f"{md.relative_to(ROOT)}:{lineno}"
                if not alt.strip():
                    errors.append(f"{where}: у изображения нет альтернативного текста")
                file = (md.parent / unquote(target.split('#', 1)[0])).resolve()
                if not file.is_file():
                    errors.append(f"{where}: нет изображения {target}")
                elif file.suffix.lower() == ".svg" and file not in parsed_svg:
                    parsed_svg.add(file)
                    try:
                        svg = ET.parse(file).getroot()
                        if svg.tag != "{http://www.w3.org/2000/svg}svg":
                            errors.append(f"{where}: {target} не является SVG")
                    except ET.ParseError as error:
                        errors.append(f"{where}: повреждён SVG {target}: {error}")
            for page in PAGE.findall(line):
                pages += 1
                if not (md.parent / page).resolve().is_file():
                    errors.append(f"{md.relative_to(ROOT)}:{lineno}: нет страницы {page}")
            for target in INCLUDE.findall(line):
                checked += 1
                path, _, anchor = target.partition(":")
                where = f"{md.relative_to(ROOT)}:{lineno}"
                file = (md.parent / path).resolve()
                if not file.is_file():
                    errors.append(f"{where}: нет файла {path}")
                    continue
                if not anchor or LINES.match(anchor):
                    continue
                begin, end = anchors(file.read_text(encoding="utf-8"))
                if anchor not in begin:
                    errors.append(f"{where}: в {path} нет ANCHOR: {anchor}")
                elif anchor not in end:
                    errors.append(f"{where}: в {path} нет ANCHOR_END: {anchor}")
    for e in errors:
        print(e)
    print(f"проверено ссылок: {checked}, ссылок на страницы: {pages}, изображений: {images}, ошибок: {len(errors)}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
