#!/usr/bin/env python3
"""Проверяет ссылки внутри собранной книги.

tools/check-includes.py смотрит исходники и ловит ссылку на несуществующую
страницу, но якорь после `#` не проверяет: mdBook выводит id заголовка из
его текста, и переименованный заголовок молча ломает ссылку
`[...](07-tasks.md#c-тегированный-указатель)` — браузер откроет страницу и
останется наверху. Здесь проверяется уже собранный HTML:

- каждая внутренняя ссылка ведёт на существующую страницу, а её якорь — на
  существующий id;
- если текст ссылки — номер раздела (`[17.5](...)`, `[раздел 4.2](...)`),
  страница-цель начинается с заголовка с этим номером. Так ловится ссылка,
  у которой номер в тексте и файл разошлись.

Запуск из корня репозитория после `mdbook build`:
python3 tools/check-links.py book
"""

import html
import re
import sys
from pathlib import Path
from urllib.parse import unquote

# Страницы, собранные из всех остальных: ошибки в них повторяют ошибки глав.
SKIP = {"print.html", "toc.html", "404.html"}
HREF = re.compile(r'<a href="([^"]+)"[^>]*>(.*?)</a>', re.S)
# Номер раздела в тексте ссылки: «4.2», «раздел 4.2», «разделе 17.5».
NUMBER = re.compile(r"^(?:[Рр]аздел\w*\s+)?(\d{1,2}\.\d{1,2})$")


def text(fragment: str) -> str:
    return html.unescape(re.sub(r"<[^>]+>", "", fragment)).strip()


def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else "book").resolve()
    pages = [p for p in sorted(root.rglob("*.html")) if p.name not in SKIP]
    ids: dict[Path, set[str]] = {}
    titles: dict[Path, str] = {}
    mains: dict[Path, str] = {}
    for page in pages:
        source = page.read_text(encoding="utf-8")
        ids[page] = {html.unescape(i) for i in re.findall(r'\bid="([^"]+)"', source)}
        m = re.search(r"<main>(.*)</main>", source, re.S)
        mains[page] = m.group(1) if m else ""
        h1 = re.search(r"<h1[^>]*>(.*?)</h1>", mains[page], re.S)
        titles[page] = text(h1.group(1)) if h1 else ""
    errors = []
    checked = 0
    for page in pages:
        where = page.relative_to(root)
        for href, label in HREF.findall(mains[page]):
            if re.match(r"^[a-z]+:", href):
                continue
            path, _, frag = href.partition("#")
            target = (page.parent / unquote(path)).resolve() if path else page
            checked += 1
            if target not in ids:
                errors.append(f"{where}: нет страницы {href}")
                continue
            if frag and unquote(html.unescape(frag)) not in ids[target]:
                errors.append(f"{where}: на {target.relative_to(root)} нет якоря #{unquote(frag)}")
            number = NUMBER.match(text(label))
            if number and not frag and path:
                if not titles[target].startswith(number.group(1) + "."):
                    errors.append(
                        f"{where}: ссылка «{text(label)}» ведёт на «{titles[target]}»"
                    )
    for e in errors:
        print(e)
    print(f"проверено ссылок: {checked}, ошибок: {len(errors)}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
