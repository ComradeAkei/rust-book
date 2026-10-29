"""Схемы подготовительного раздела; запуск: python3 tools/diagrams_foundations.py."""

from diagram_style import Diagram


def bits():
    d = Diagram("Как восемь битов образуют число", "Байт 00001101: единицы выбирают веса 8, 4 и 1. Их сумма равна 13.", height=440)
    d.text(36, 132, "Вес каждого разряда", "small")
    for i, (weight, bit) in enumerate(zip((128, 64, 32, 16, 8, 4, 2, 1), "00001101")):
        x = 40 + i * 116
        d.text(x + 52, 174, str(weight), "label", "middle")
        d.rect(x, 193, 104, 82, "green" if bit == "1" else "gray")
        d.text(x + 52, 246, bit, "title", "middle")
    d.box(40, 310, 916, 74, "0b0000_1101 = 8 + 4 + 1 = 13", tone="green")
    d.note("В одном байте 2⁸ = 256 комбинаций: для u8 это числа от 0 до 255.")
    d.save("basic-bits.svg")


def address():
    d = Diagram("Адрес и содержимое памяти", "Указатель p содержит адрес 1002. По этому адресу находится байт 30. Копия указателя не создаёт копию байта.", height=460)
    d.box(36, 151, 286, 120, "Указатель p", ["хранит адрес 1002", "*p читает число 30"])
    d.text(365, 128, "Адреса байтов", "small")
    for i, value in enumerate((10, 20, 30, 40, 50, 60)):
        x = 365 + 98 * i
        d.text(x + 46, 163, str(1000 + i), "small", "middle")
        d.rect(x, 183, 92, 85, "green" if i == 2 else "gray", radius=6)
        d.text(x + 46, 236, str(value), "title", "middle")
    d.path("M 180 271 V 321 H 607 V 270", tone="blue")
    d.text(365, 356, "Переход к данным по адресу", "body")
    d.note("Адрес 1002 и лежащее по нему число 30 — разные части этой модели.", y=400)
    d.note("Право доступа к памяти проверяется отдельно от числового адреса.", y=430)
    d.save("basic-address.svg")


def stack_heap():
    d = Diagram("Стек, куча и данные Vec", "Объект вектора в кадре функции хранит указатель, длину и ёмкость. Указатель ведёт к отдельному блоку с четырьмя байтами в куче.", height=540)
    d.box(36, 145, 425, 300, "Кадр текущей функции", ["frame: Vec<u8>"])
    for x, label in ((60, "ptr"), (190, "len = 4"), (320, "cap = 4")):
        d.rect(x, 257, 115, 65, "blue", radius=6)
        d.text(x + 57.5, 297, label, "code", "middle")
    d.text(60, 418, "Локальные данные вызова", "small")
    d.box(550, 145, 414, 300, "Куча", ["Отдельный блок памяти"])
    for i, value in enumerate((10, 20, 30, 40)):
        x = 574 + 92 * i
        d.rect(x, 257, 82, 65, "green", radius=6)
        d.text(x + 41, 297, str(value), "code", "middle")
    d.path("M 117 322 V 366 H 615 V 323", tone="blue")
    d.text(574, 418, "Блоком владеет frame", "small")
    d.note("Схема размещения: оптимизатор может держать поля Vec в регистрах.")
    d.save("basic-stack-heap.svg")


def build():
    d = Diagram("Как исходник становится программой", "Компилятор переводит исходник в объектный файл. Компоновщик объединяет объектный файл и библиотеки в готовый файл для выбранной платформы.", height=540)
    d.box(36, 143, 240, 114, "Исходник", ["main.rs"])
    d.box(380, 143, 240, 114, "Компилятор", ["rustc"])
    d.box(724, 143, 240, 114, "Объектный файл", ["код и ссылки"])
    d.arrow(276, 200, 377, 200)
    d.arrow(620, 200, 721, 200)
    d.box(36, 341, 240, 114, "Библиотеки", ["готовые части"] , tone="purple")
    d.box(380, 341, 240, 114, "Компоновщик", ["соединяет части"], tone="purple")
    d.box(724, 341, 240, 114, "Готовый файл", ["для выбранной", "платформы"], tone="green")
    d.arrow(276, 398, 377, 398, tone="purple")
    d.arrow(620, 398, 721, 398, tone="green")
    d.path("M 844 257 V 295 H 500 V 338", tone="purple")
    d.note("cargo build организует сборку; cargo run после сборки запускает программу.")
    d.save("basic-build.svg")


def option():
    d = Diagram("Значение есть или отсутствует", "В match Some(n) извлекает число n, а None выбирает запасное значение. Some(0) даёт 0, None даёт 42.", height=550)
    d.box(36, 135, 420, 78, "reading = Some(0)")
    d.box(544, 135, 420, 78, "reading = None", tone="orange")
    d.arrow(246, 215, 246, 258)
    d.arrow(754, 215, 754, 258, tone="orange")
    d.box(36, 260, 420, 103, "Some(n) => n", ["n получает число 0"])
    d.box(544, 260, 420, 103, "None => 42", ["числа нет: берём запасное"], tone="orange")
    d.arrow(246, 365, 246, 408, tone="green")
    d.arrow(754, 365, 754, 408, tone="orange")
    d.box(36, 410, 420, 75, "Результат: 0", tone="green")
    d.box(544, 410, 420, 75, "Результат: 42", tone="orange")
    d.note("Some(0) содержит число; None обозначает отсутствие числа.")
    d.save("basic-option.svg")


def main():
    for draw in (bits, address, stack_heap, build, option):
        draw()


if __name__ == "__main__":
    main()
