"""Наглядные схемы к главам 1–15. Запуск: python3 tools/diagrams_core.py."""

from diagram_style import Diagram


def endian():
    d = Diagram(
        "Одно число — два порядка байтов",
        "Число 0x11223344: little-endian хранит байты 44, 33, 22, 11 "
        "по возрастающим адресам, big-endian — 11, 22, 33, 44.",
        height=510,
    )
    d.text(36, 128, "Число u32: 0x1122_3344", "code")
    for y, label, detail, values, tone in (
        (180, "little-endian", "младший байт первым", ["44", "33", "22", "11"], "blue"),
        (310, "big-endian", "старший байт первым", ["11", "22", "33", "44"], "purple"),
    ):
        d.text(36, y + 34, label, "label")
        d.text(36, y + 65, detail, "small")
        for i, value in enumerate(values):
            x = 350 + i * 145
            d.rect(x, y, 125, 72, tone)
            d.text(x + 62.5, y + 45, "0x" + value, "code", "middle")
            d.text(x + 62.5, y - 12, "a" if i == 0 else f"a + {i}", "small", "middle")
    d.arrow(350, 428, 910, 428)
    d.text(630, 459, "адрес увеличивается", "body", "middle")
    d.save("core-endian.svg")


def slice_view():
    d = Diagram(
        "Срез смотрит внутрь массива",
        "Массив a содержит 10, 20, 30, 40. Ссылка s на a[1..3] хранит "
        "адрес элемента 20 и длину 2, не создавая копию элементов.",
        height=500,
    )
    d.text(36, 128, "a: [u8; 4] = [10, 20, 30, 40]", "code")
    for i, value in enumerate([10, 20, 30, 40]):
        x = 340 + 145 * i
        d.text(x + 65, 162, f"индекс {i}", "small", "middle")
        d.rect(x, 180, 130, 85, "green" if i in (1, 2) else "gray")
        d.text(x + 65, 232, str(value), "label", "middle")
    d.box(60, 340, 280, 100, "s: &[u8]", ["ptr  |  len = 2"], "blue")
    d.arrow(340, 370, 485, 265, tone="blue")
    d.text(575, 323, "s = &a[1..3]", "code", "middle")
    d.text(575, 358, "доступны 20 и 30", "body", "middle")
    d.note("Длина — число элементов; верхняя граница 3 в срез не входит.")
    d.save("core-slice.svg")


def ownership():
    d = Diagram(
        "Перемещение передаёт владение",
        "Два снимка одного буфера Vec: до send им владеет frame в produce, "
        "после передачи — параметр frame в send. Адрес буфера не меняется.",
        height=590,
    )
    d.text(65, 128, "ДО вызова", "label")
    d.text(600, 128, "ВО ВРЕМЯ send", "label")
    d.box(65, 158, 300, 125, "produce: frame", ["ptr = А", "len = 64, cap = 64"], "blue")
    d.box(600, 158, 320, 125, "send: frame", ["ptr = А", "len = 64, cap = 64"], "blue")
    d.arrow(385, 219, 575, 219, "send(frame)", tone="blue")
    d.arrow(215, 283, 215, 367, tone="green")
    d.arrow(760, 283, 760, 367, tone="green")
    d.box(65, 370, 300, 100, "Буфер по адресу А", ["0xA5  0xA5  …"], "green")
    d.box(600, 370, 320, 100, "Тот же буфер А", ["0xA5  0xA5  …"], "green")
    d.text(600, 315, ["Старое имя", "недоступно"], "small", fill="#a93832")
    d.text(65, 512, "Это два момента времени, а не два выделенных блока.", "body")
    d.note("При выходе из send буфер освобождается один раз.")
    d.save("core-ownership.svg")


def borrowing():
    d = Diagram(
        "Читатели или один писатель",
        "Периоды двух разделяемых заимствований могут перекрываться. "
        "Уникальное заимствование тех же данных начинается после их последнего использования.",
        height=535,
    )
    d.arrow(260, 149, 940, 149)
    d.text(937, 130, "порядок исполнения", "small", "end")
    for y, label in [(233, "Читатель 1"), (303, "Читатель 2"), (373, "Писатель")]:
        d.text(36, y, label, "body")
    d.rect(260, 200, 285, 52, "blue")
    d.text(402, 234, "r1: &T", "code", "middle")
    d.rect(335, 270, 210, 52, "green")
    d.text(440, 304, "r2: &T", "code", "middle")
    d.rect(630, 340, 285, 52, "orange")
    d.text(772, 374, "w: &mut T", "code", "middle")
    d.path("M 585 180 V 415", arrow=False, dashed=True)
    d.text(610, 218, ["Оба читателя", "больше не нужны"], "small")
    d.text(260, 450, "Проверка относится к одним и тем же данным.", "body")
    d.note("Периоды доступа читателей и писателя не перекрываются.")
    d.save("core-borrowing.svg")


def lifetime():
    d = Diagram(
        "Ссылка не продлевает жизнь данных",
        "В вызове longest(&port, &mode) результат best допустим только "
        "пока доступны оба аргумента. После уничтожения mode пользоваться best нельзя.",
        height=570,
    )
    d.text(36, 126, "best = longest(&port, &mode)", "code")
    d.arrow(225, 153, 935, 153)
    d.text(934, 132, "порядок исполнения", "small", "end")
    for label, x, y, w, tone in [
        ("port", 225, 188, 695, "blue"),
        ("mode", 370, 278, 280, "green"),
        ("best", 480, 368, 170, "purple"),
    ]:
        d.text(36, y + 36, label, "code")
        d.rect(x, y, w, 56, tone)
        d.text(x + w / 2, y + 36, "данные доступны" if label != "best" else "доступ по ссылке", "small", "middle")
    d.path("M 660 264 V 455", arrow=False, dashed=True, tone="red")
    d.text(690, 304, ["mode", "уничтожен"], "small", fill="#a93832")
    d.box(700, 365, 240, 85, "Здесь нельзя", ["использовать best"], "red")
    d.text(36, 490, "'a описывает допустимый участок использования ссылки.", "body")
    d.note("Аннотация времени жизни не заставляет mode существовать дольше.")
    d.save("core-lifetime.svg")


def struct_layout():
    d = Diagram(
        "Откуда берутся 12 байт структуры",
        "repr(C) с полями u8 channel, u32 value и u8 flags: channel по смещению 0, "
        "value по 4, flags по 8; три байта заполнения перед value и три в конце.",
        height=540,
    )
    d.text(36, 125, "repr(C): channel: u8, value: u32, flags: u8", "code")
    values = ["ch", "·", "·", "·", "v0", "v1", "v2", "v3", "fl", "·", "·", "·"]
    for i, value in enumerate(values):
        x = 130 + 66 * i
        tone = "blue" if i == 0 else "purple" if 4 <= i <= 7 else "green" if i == 8 else "gray"
        d.text(x + 30, 186, str(i), "small", "middle")
        d.rect(x, 205, 60, 72, tone, dashed=value == "·")
        d.text(x + 30, 250, value, "code", "middle")
    d.text(36, 175, "байт", "small")
    d.text(160, 311, "channel", "small", "middle")
    d.text(523, 311, "value: байты v0…v3", "small", "middle")
    d.text(688, 342, "flags", "small", "middle")
    d.box(40, 367, 420, 100, "Полезные данные: 6 байт", ["1 + 4 + 1"], "blue")
    d.box(530, 367, 420, 100, "Заполнение: 6 байт", ["3 перед value + 3 в конце"], "gray")
    d.note("Размер 12 кратен выравниванию структуры 4; точка означает padding.")
    d.save("core-struct-layout.svg")


def result_flow():
    d = Diagram(
        "Оператор ? выбирает один из двух путей",
        "При Ok первое чтение отдаёт байт hi и выполнение продолжается. "
        "При Err ошибка возвращается из read_temp, а второе чтение не выполняется.",
        height=565,
    )
    d.box(345, 145, 310, 105, "Первое чтение", ["bus.read(0x00)?"], "blue")
    d.arrow(425, 250, 250, 335, "Ok", tone="green")
    d.arrow(575, 250, 750, 335, "Err", tone="red")
    d.box(50, 340, 400, 130, "Ok(hi)", ["извлечь байт hi", "выполнить второе чтение"], "green")
    d.box(550, 340, 400, 130, "Err(e)", ["вернуть Err(e)", "сразу выйти из read_temp"], "red")
    d.note("При ошибке второго чтения ? точно так же завершает функцию.")
    d.save("core-result-flow.svg")


def dispatch():
    d = Diagram(
        "Как выбирается вызываемый метод",
        "Обобщение специализируется для Thermo при компиляции. "
        "Вызов через dyn Sensor получает адрес реализации из таблицы методов; "
        "сама ссылка также хранит адрес исходного датчика.",
        height=645,
    )
    d.rect(40, 125, 920, 185, "gray")
    d.text(62, 162, "Обобщение: тип известен при компиляции", "label")
    d.box(65, 195, 260, 80, "read<Thermo>", tone="blue")
    d.arrow(325, 235, 580, 235, "прямой вызов", tone="green")
    d.box(590, 195, 330, 80, "Thermo::read", tone="green")
    d.rect(40, 345, 920, 235, "gray")
    d.text(62, 381, "dyn: выбор через таблицу методов", "label")
    d.box(65, 410, 230, 95, "&dyn Sensor", ["data | vtable"], "blue")
    d.arrow(295, 457, 405, 457, tone="purple")
    d.box(415, 410, 220, 95, "vtable", ["адрес read"], "purple")
    d.arrow(635, 457, 715, 457, tone="green")
    d.box(725, 410, 210, 95, "Thermo::read", tone="green")
    d.arrow(150, 505, 150, 523, tone="blue")
    d.rect(65, 525, 230, 40, "green")
    d.text(180, 552, "Thermo { raw }", "code", "middle")
    d.text(330, 553, "Данные остаются на прежнем месте", "small")
    d.note("Схема выбора вызова; оптимизатор может встроить или упростить его.")
    d.save("core-dispatch.svg")


def vec_capacity():
    d = Diagram(
        "Длина и ёмкость Vec — разные числа",
        "Vec u32 имеет длину 3 и ёмкость 8. В буфере созданы три значения, "
        "а пять мест ещё не содержат доступных u32. Буфер занимает 32 байта.",
        height=535,
    )
    d.box(40, 150, 280, 165, "Vec<u32>", ["ptr → буфер", "len = 3", "cap = 8"], "blue")
    d.text(390, 130, "Буфер: 8 мест по 4 байта", "label")
    for i in range(8):
        x = 390 + 66 * i
        d.text(x + 30, 170, str(i), "small", "middle")
        d.rect(x, 190, 60, 82, "green" if i < 3 else "gray", dashed=i >= 3)
        d.text(x + 30, 239, str((i + 1) * 10) if i < 3 else "·", "code", "middle")
    d.arrow(320, 231, 385, 231, tone="blue")
    d.path("M 390 290 V 310 H 582 V 290", arrow=False, tone="green")
    d.text(486, 345, "len = 3", "code", "middle")
    d.path("M 588 290 V 310 H 918 V 290", arrow=False)
    d.text(753, 345, ["5 свободных мест", "читать как u32 нельзя"], "small", "middle")
    d.path("M 390 397 V 414 H 918 V 397", arrow=False, tone="blue")
    d.text(654, 453, "Буфер: 8 × 4 = 32 байта", "code", "middle")
    d.note("Показаны логические поля Vec; их порядок в памяти не гарантирован.")
    d.save("core-vec-capacity.svg")


def main():
    for draw in (endian, slice_view, ownership, borrowing, lifetime, struct_layout, result_flow, dispatch, vec_capacity):
        draw()
    print("Создано схем к главам 1–15: 9")


if __name__ == "__main__":
    main()
