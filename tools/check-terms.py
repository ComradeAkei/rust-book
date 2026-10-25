#!/usr/bin/env python3
"""Проверяет, что термины книги раскрыты.

Глоссарий — src/appendix-f.md. Каждая его статья — элемент списка
определений:

    <span id="niche"></span>**Ниша** (niche)
    : Битовый шаблон, который не встречается ни у одного значения типа. ...
      → [6.4](ch06/04-niches.md)
      <!-- term: ниш(?:а|и|у|е|ей|ам|ами|ах)\\b -->

В скобках после термина — английский оригинал или расшифровка сокращения.
`term:` — регулярное выражение, по которому термин ищется в прозе без учёта
регистра, `term-cs:` — с учётом регистра (сокращения: `UB` не должно ловить
слово «ub» в `ub_checks`). Образец в обратных кавычках ищется только как
целый фрагмент кода: `` `cmov\\w*` `` найдёт `cmovne`, но не `cmovne eax, ecx`.
Образцов у статьи может быть несколько: обычно один для прозы и один для кода.
Пометка `<!-- once -->` — сквозной термин (UB, LLVM): ссылка нужна только при
первом упоминании в книге, а не в каждой главе. `<!-- only: ch13 appendix-e/ch13 -->`
ограничивает проверку и расстановку ссылок перечисленными единицами, а
`<!-- skip: ch29 -->` исключает единицы: так слово, у которого в других главах
свой смысл («адаптер» итератора и отладочный, модуль Rust и модуль ядра),
размечается только там, где оно термин.

Проверки:
1. Статьи разобраны, id уникальны, каждая стоит под своей буквой и по
   алфавиту.
2. Первая ссылка после «→» ведёт на раздел, где термин выделен курсивом или
   жирным, а в том же абзаце стоит то, что в скобках после термина. Так
   глоссарий не ссылается на место, где термин только упомянут.
3. В каждой главе (все файлы src/chNN/ подряд) и на каждой отдельной
   странице (введение, части, проект, приложения, разборы приложения E)
   первое употребление термина — его выделенное определение или ссылка на
   глоссарий `[текст][g-id]`.
4. Ссылки `[...][g-id]` определены в конце файла, и адрес с подсказкой
   (title, всплывает при наведении) совпадают с глоссарием.

Употреблением считается только проза: блоки кода, заголовки, строки плана
главы (`| [N.M](...) | ... |`) и оглавления разборов, адреса ссылок,
HTML-теги и тексты прочих ссылок пропускаются.

Запуск из корня репозитория:
  python3 tools/check-terms.py [ЕДИНИЦА...]        проверка (без аргументов — вся книга)
  python3 tools/check-terms.py --link ЕДИНИЦА...   сделать первые употребления ссылками
  python3 tools/check-terms.py --refs              обновить определения ссылок g-…
  python3 tools/check-terms.py --report [ID...]    где термин встречается впервые
Единица — ch06, introduction, part2, project, appendix-a, appendix-e,
appendix-e/ch06.
"""

import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "src"
GLOSSARY = "appendix-f.md"
MARK = "<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->"
FILL = "\x01"

ENTRY = re.compile(r'^<span id="([a-z0-9][a-z0-9-]*)"></span>\*\*(.+?)\*\*(?: \((.+)\))?$')
LETTER = re.compile(r"^## (\S)$")
HOME = re.compile(r"→ \[[^\]]+\]\(([^)#\s]+\.md)[^)]*\)")
COMMENT = re.compile(r"<!--\s*(term-cs|term|once|only|skip)\b:?\s*(.*?)\s*-->")

FENCE = re.compile(r"^\s*(```|~~~)")
# Строки, которые не проза: заголовки, include, определения ссылок, HTML,
# строки плана главы и оглавления разборов (`| N. Глава | [разбор](chNN.md) |`).
NOT_PROSE = re.compile(
    r"^\s*(?:#{1,6}\s|\{\{#|\[[^\]]+\]:\s|<!--|</?[a-zA-Z][^>]*>\s*$|\|\s*\[\d+(?:\.\d+)?\]\("
    r"|\|\s*\d+\.\s[^|]*\|\s*\[[^\]]+\]\(ch\d\d\.md\))"
)
CODE = re.compile(r"(`+)(.+?)\1", re.S)
TAG = re.compile(r"<!--.*?-->|<[^>\n]+>|https?://[^\s)>\]]+|\{\{[^}]*\}\}", re.S)
LINK = re.compile(r"\[((?:[^\[\]]|\[[^\[\]]*\])*)\](\([^()\s]*(?:\s+\"[^\"]*\")?\)|\[[^\[\]]*\])")
BOLD = re.compile(r"\*\*(?=\S)(.+?)(?<=\S)\*\*", re.S)
ITALIC = re.compile(r"(?<![*\w\\])\*(?=[^\s*])(.+?)(?<=[^\s*])\*(?![*\w])", re.S)
GLINK = re.compile(r"^\[g-([a-z0-9-]+)\]$|^\([^)\s]*" + re.escape(GLOSSARY) + r"#([a-z0-9-]+)")
GREF = re.compile(r"\]\[g-([a-z0-9-]+)\]")
GDEF = re.compile(r"^\[g-([a-z0-9-]+)\]:", re.M)


@dataclass
class Term:
    id: str
    name: str
    pair: str
    text: str
    home: str
    # (образец, ищется ли он в коде): у статьи их может быть несколько.
    patterns: list[tuple[re.Pattern, bool]]
    once: bool
    line: int
    # Единицы, где термин размечается: None — везде, кроме skip.
    only: set[str] | None
    skip: set[str]

    def applies(self, unit: str) -> bool:
        return (self.only is None or unit in self.only) and unit not in self.skip

    def title(self) -> str:
        head = f"{self.name} ({self.pair})" if self.pair else self.name
        return f"{head}: {first_sentence(self.text)}"


def plain(md: str) -> str:
    """Текст без разметки markdown: для подсказок и сравнений."""
    s = re.sub(r"<!--.*?-->", "", md, flags=re.S)
    s = re.sub(r"<[^>]+>", "", s)
    s = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", s)
    s = re.sub(r"\[([^\]]*)\]\[[^\]]*\]", r"\1", s)
    s = s.replace("**", "").replace("`", "")
    s = re.sub(r"(?<![\w*])\*(?=\S)(.+?)(?<=\S)\*(?!\w)", r"\1", s)
    return re.sub(r"\s+", " ", s).strip()


def first_sentence(text: str) -> str:
    # Конец фразы — точка перед заглавной буквой или концом текста; «т. е.»
    # и «6.4» фразу не обрывают.
    m = re.match(r"(.+?[.!?])(?=\s+[«(A-ZА-ЯЁ]|$)", text)
    return m.group(1) if m else text


def letter_of(name: str) -> str:
    for ch in name:
        if ch.isalnum():
            return "Е" if ch in "Ёё" else ch.upper()
    return "?"


def sort_key(name: str) -> str:
    return "".join(c for c in name.casefold().replace("ё", "е") if c.isalnum() or c == " ")


def load_glossary(errors: list[str]) -> list[Term]:
    path = SRC / GLOSSARY
    lines = path.read_text(encoding="utf-8").splitlines()
    terms: list[Term] = []
    seen: dict[str, int] = {}
    letter = None
    previous = None
    i = 0
    while i < len(lines):
        line = lines[i]
        m = LETTER.match(line)
        if m:
            letter, previous = m.group(1), None
            i += 1
            continue
        if not line.startswith('<span id="'):
            i += 1
            continue
        where = f"src/{GLOSSARY}:{i + 1}"
        m = ENTRY.match(line)
        if not m:
            errors.append(f"{where}: статья не разобрана: {line}")
            i += 1
            continue
        tid, name_md, pair = m.group(1), m.group(2), plain(m.group(3) or "")
        body = []
        j = i + 1
        if j < len(lines) and lines[j].startswith(": "):
            body.append(lines[j][2:])
            j += 1
            while j < len(lines) and lines[j].startswith("  ") and lines[j].strip():
                body.append(lines[j].strip())
                j += 1
        else:
            errors.append(f"{where}: после термина нет строки «: определение»")
        dd = "\n".join(body)
        name = plain(name_md)
        if tid in seen:
            errors.append(f"{where}: id {tid} уже занят строкой {seen[tid]}")
        seen[tid] = i + 1
        if letter != letter_of(name):
            errors.append(f"{where}: «{name}» стоит под буквой {letter}")
        elif previous is not None and sort_key(name) < sort_key(previous):
            errors.append(f"{where}: «{name}» должен стоять раньше «{previous}»")
        previous = name
        home = HOME.search(dd)
        sources, once, only, skip = [], False, None, set()
        for kind, value in COMMENT.findall(dd):
            if kind == "once":
                once = True
            elif kind == "only":
                only = (only or set()) | set(value.split())
            elif kind == "skip":
                skip |= set(value.split())
            else:
                sources.append((value, re.I if kind == "term" else 0))
        text = plain(dd.split("→")[0])
        if not home:
            errors.append(f"{where}: у «{name}» нет ссылки «→ [N.M](файл.md)» на место определения")
        if not sources:
            errors.append(f"{where}: у «{name}» нет образца <!-- term: ... -->")
        if '"' in first_sentence(text) or '"' in name or '"' in pair:
            errors.append(f"{where}: прямые кавычки ломают подсказку, нужны «ёлочки»")
        if not text:
            errors.append(f"{where}: у «{name}» пустое определение")
        patterns = []
        for source, flags in sources:
            code = source.startswith("`") and source.endswith("`") and len(source) > 2
            if code:
                source = source[1:-1]
            try:
                patterns.append((re.compile(source, flags), code))
            except re.error as e:
                errors.append(f"{where}: образец «{source}» не компилируется: {e}")
        if home and patterns and len(patterns) == len(sources):
            terms.append(Term(tid, name, pair, text, home.group(1), patterns, once, i + 1, only, skip))
        i = j
    return terms


def units() -> list[tuple[str, list[str]]]:
    """Единицы проверки в порядке SUMMARY.md: (имя, файлы относительно src)."""
    result: list[tuple[str, list[str]]] = []
    for line in (SRC / "SUMMARY.md").read_text(encoding="utf-8").splitlines():
        m = re.search(r"\]\(([^)]+\.md)\)", line)
        if not m or m.group(1) == GLOSSARY:
            continue
        file = m.group(1)
        if re.match(r"ch\d\d/", file):
            name = file.split("/")[0]
        elif file == "appendix-e/index.md":
            name = "appendix-e"
        else:
            name = file.removesuffix(".md")
        if result and result[-1][0] == name:
            result[-1][1].append(file)
        else:
            result.append((name, [file]))
    return result


@dataclass
class Page:
    file: str
    raw: str
    masked: str
    paragraphs: list[tuple[int, int]]
    links: list[tuple[int, int, str | None]]
    emphasis: list[tuple[int, int]]
    codes: list[tuple[int, int, str]]

    def line(self, offset: int) -> int:
        return self.raw.count("\n", 0, offset) + 1

    def paragraph(self, offset: int) -> tuple[int, int]:
        for s, e in self.paragraphs:
            if s <= offset < e:
                return s, e
        return offset, offset


def blank(chars: list[str], s: int, e: int) -> None:
    for k in range(s, e):
        if chars[k] != "\n":
            chars[k] = FILL


def parse(file: str) -> Page:
    raw = (SRC / file).read_text(encoding="utf-8")
    chars = list(raw)
    paragraphs: list[tuple[int, int]] = []
    pos, start, end, fence = 0, None, 0, False
    for line in raw.splitlines(keepends=True):
        text = line.rstrip("\n")
        prose = False
        if FENCE.match(text):
            fence = not fence
        elif not fence and text.strip() and not NOT_PROSE.match(text):
            prose = True
        if prose:
            if start is None:
                start = pos
            end = pos + len(text)
        else:
            blank(chars, pos, pos + len(text))
            if start is not None:
                paragraphs.append((start, end))
                start = None
        pos += len(line)
    if start is not None:
        paragraphs.append((start, end))
    links, emphasis, codes = [], [], []
    for ps, pe in paragraphs:
        p = raw[ps:pe]
        for m in CODE.finditer(p):
            codes.append((ps + m.start(), ps + m.end(), m.group(2).strip()))
            blank(chars, ps + m.start(), ps + m.end())
        for m in TAG.finditer(p):
            blank(chars, ps + m.start(), ps + m.end())
        view = "".join(chars[ps:pe])
        for m in LINK.finditer(view):
            g = GLINK.match(m.group(2))
            gid = (g.group(1) or g.group(2)) if g else None
            links.append((ps + m.start(1), ps + m.end(1), gid))
            blank(chars, ps + m.start(2), ps + m.end(2))
        view = "".join(chars[ps:pe])
        for m in BOLD.finditer(view):
            emphasis.append((ps + m.start(), ps + m.end()))
        unbold = BOLD.sub(lambda m: FILL * 2 + m.group(1) + FILL * 2, view)
        for m in ITALIC.finditer(unbold):
            emphasis.append((ps + m.start(), ps + m.end()))
    return Page(file, raw, "".join(chars), paragraphs, links, emphasis, codes)


def occurrences(term: Term, page: Page):
    """Употребления термина: (начало, конец, вид), вид — def, link или bare."""
    spans = []
    for pattern, code in term.patterns:
        if code:
            spans += [(s, e) for s, e, body in page.codes if pattern.fullmatch(body)]
        else:
            # Вторая половина составного слова («объект-владелец») —
            # не употребление термина.
            spans += [
                (m.start(), m.end())
                for m in pattern.finditer(page.masked)
                if m.end() > m.start() and FILL not in m.group(0)
                and not re.match(r"\w-", page.masked[max(m.start() - 2, 0):m.start()])
            ]
    last = -1
    for s, e in sorted(spans):
        if s < last:
            continue
        last = e
        link = next(((ls, le, gid) for ls, le, gid in page.links if ls <= s and e <= le), None)
        if link:
            if link[2] == term.id:
                yield s, e, "link"
            continue
        if any(es <= s and e <= ee for es, ee in page.emphasis):
            yield s, e, "def"
        else:
            yield s, e, "bare"


def first_in(term: Term, files: list[str], pages: dict[str, Page]):
    for f in files:
        for s, e, kind in occurrences(term, pages[f]):
            return f, s, e, kind
    return None


def check_home(term: Term, pages: dict[str, Page], errors: list[str]) -> None:
    where = f"src/{GLOSSARY}:{term.line}"
    page = pages.get(term.home)
    if page is None:
        errors.append(f"{where}: «{term.name}» ссылается на {term.home}, которого нет в SUMMARY.md")
        return
    pairs = [p.strip().casefold() for p in term.pair.split(",") if p.strip()]
    for s, e, kind in occurrences(term, page):
        if kind != "def":
            continue
        ps, pe = page.paragraph(s)
        text = plain(page.raw[ps:pe]).casefold()
        if not pairs or any(p in text for p in pairs):
            return
    need = f" и «{term.pair}» в том же абзаце" if term.pair else ""
    errors.append(
        f"{where}: в {term.home} нет определения «{term.name}»: нужен выделенный термин{need}"
    )


def refs_block(file: str, page_raw: str, by_id: dict[str, Term], errors: list[str]) -> str | None:
    """Ожидаемый блок определений ссылок g-… для файла (None — блок не нужен)."""
    body = page_raw.split(MARK)[0]
    used = sorted(set(GREF.findall(body)))
    lines = []
    rel = os.path.relpath(SRC / GLOSSARY, (SRC / file).parent)
    for gid in used:
        term = by_id.get(gid)
        if term is None:
            errors.append(f"src/{file}: ссылка на неизвестный термин g-{gid}")
            continue
        lines.append(f'[g-{gid}]: {rel}#{gid} "{term.title()}"')
    if not lines:
        return None
    return MARK + "\n" + "\n".join(lines) + "\n"


def with_refs(raw: str, block: str | None) -> str:
    body = raw.split(MARK)[0].rstrip("\n") + "\n"
    return body + ("\n" + block if block else "")


def main(argv: list[str]) -> int:
    mode = "check"
    if argv and argv[0] in ("--link", "--refs", "--report"):
        mode, argv = argv[0][2:], argv[1:]
    errors: list[str] = []
    terms = load_glossary(errors)
    by_id = {t.id: t for t in terms}
    all_units = units()
    names = [n for n, _ in all_units]
    pages = {f: parse(f) for _, files in all_units for f in files}

    if mode == "report":
        chosen = [t for t in terms if not argv or t.id in argv]
        for t in chosen:
            row = []
            for name, files in all_units:
                hit = first_in(t, files, pages)
                if hit:
                    f, s, _, kind = hit
                    row.append(f"{name}:{pages[f].line(s)}:{kind}")
            print(f"{t.id}: " + " ".join(row))
        return 0

    selected = argv or names
    unknown = [u for u in selected if u not in names]
    if unknown:
        print("нет таких единиц: " + ", ".join(unknown))
        return 2

    for t in terms:
        check_home(t, pages, errors)
        for u in sorted((t.only or set()) | t.skip):
            if u not in names:
                errors.append(f"src/{GLOSSARY}:{t.line}: у «{t.name}» в only/skip нет единицы {u}")

    # Первое употребление в книге — для сквозных терминов только оно.
    first_unit: dict[str, str] = {}
    for t in terms:
        for name, files in all_units:
            if first_in(t, files, pages):
                first_unit[t.id] = name
                break
        else:
            errors.append(f"src/{GLOSSARY}:{t.line}: «{t.name}» в прозе книги не встречается")

    edits: dict[str, list[tuple[int, int, str]]] = {}
    for name, files in all_units:
        if name not in selected:
            continue
        for t in terms:
            if t.once and first_unit.get(t.id) != name or not t.applies(name):
                continue
            hit = first_in(t, files, pages)
            if not hit or hit[3] != "bare":
                continue
            f, s, e, _ = hit
            if mode == "link":
                edits.setdefault(f, []).append((s, e, t.id))
            else:
                word = pages[f].raw[s:e].replace("\n", " ")
                errors.append(
                    f"src/{f}:{pages[f].line(s)}: первое в {name} «{word}» — "
                    f"не определение и не ссылка [{word}][g-{t.id}]"
                )

    changed = set()
    for f, items in edits.items():
        raw = pages[f].raw
        items.sort()
        kept, last = [], -1
        for s, e, gid in items:
            if s < last:
                print(f"src/{f}:{pages[f].line(s)}: g-{gid} пересекается с соседней ссылкой, пропущен")
                continue
            kept.append((s, e, gid))
            last = e
        for s, e, gid in reversed(kept):
            raw = raw[:s] + f"[{raw[s:e]}][g-{gid}]" + raw[e:]
        pages[f].raw = raw
        changed.add(f)

    # Определения ссылок g-… в конце файлов.
    for f in sorted(SRC.rglob("*.md")):
        rel = f.relative_to(SRC).as_posix()
        if rel == GLOSSARY:
            continue
        raw = pages[rel].raw if rel in pages else f.read_text(encoding="utf-8")
        body = raw.split(MARK)[0]
        stray = GDEF.findall(body)
        if stray:
            errors.append(f"src/{rel}: определения g-{', g-'.join(stray)} стоят не в блоке в конце файла")
        expected = with_refs(raw, refs_block(rel, raw, by_id, errors))
        if MARK not in raw and not GREF.search(body):
            continue
        if expected != raw:
            if mode in ("link", "refs"):
                if rel in pages:
                    pages[rel].raw = expected
                changed.add(rel)
                (SRC / rel).write_text(expected, encoding="utf-8")
            else:
                errors.append(f"src/{rel}: блок ссылок на глоссарий устарел: python3 tools/check-terms.py --refs")
    for rel in changed:
        if rel in pages:
            (SRC / rel).write_text(pages[rel].raw, encoding="utf-8")

    for e in errors:
        print(e)
    if mode == "check":
        print(f"терминов: {len(terms)}, единиц: {len(selected)}, ошибок: {len(errors)}")
    else:
        print(f"изменено файлов: {len(changed)}, ошибок: {len(errors)}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
