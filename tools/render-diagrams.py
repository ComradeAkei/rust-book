#!/usr/bin/env python3
"""Пересоздать учебные SVG: python3 tools/render-diagrams.py.

Исходники сгруппированы в diagrams_*.py, единый стиль — diagram_style.py.
Генерация использует только стандартную библиотеку Python.
"""

from diagrams_foundations import main as foundations
from diagrams_core import main as core
from diagrams_advanced import main as advanced
from diagrams_systems import main as systems


if __name__ == "__main__":
    for draw in (foundations, core, advanced, systems):
        draw()
