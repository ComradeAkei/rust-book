#!/usr/bin/env bash
# Раздел 31.8: cargo mutants на разборе кадров проекта (telemetry-frame)
# и его исходы из текста главы. Первый прогон — с прежними тестами
# проекта (без tests/mutants.rs): выживают ровно два мутанта, граница в
# encode и формула в size_hint. Второй — со всеми тестами: выживших нет.
# В обоих прогонах мутанты CRC «нежизнеспособны» — их отвергает при
# сборке const-проверка контрольного значения, а мутанты, после которых
# Frames не сдвигается, зависают и снимаются по тайм-ауту.
# Доказательства Kani (src/proofs.rs) обычная сборка не видит, поэтому их
# файл исключён. Запуск: bash code/ch31/mutants/run.sh [флаги cargo
# mutants, например -j 4]. Нужен cargo-mutants 27.1.0.
set -u
# Вывод cargo сверяется с текстом: без кодов цвета, даже если снаружи
# задан CARGO_TERM_COLOR=always (так в CI).
export LC_ALL=C CARGO_TERM_COLOR=never
cd "$(dirname "$0")/../../.." || exit 2
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0

# mutants <имя> <код выхода> <выживших> [-- аргументы cargo test]
mutants() {
    local name=$1 want=$2 missed=$3
    shift 3
    TMPDIR="$tmp" cargo mutants -p telemetry-frame --gitignore true \
        --exclude project/telemetry-frame/src/proofs.rs --no-shuffle \
        -o "$tmp/$name" "${extra[@]}" "$@" >"$tmp/$name.log" 2>&1
    local got=$? out="$tmp/$name/mutants.out" counts
    counts=$(for k in caught missed timeout unviable; do
        printf '%s %s, ' "$k" "$(grep -c . "$out/$k.txt" 2>"$tmp/err")"
    done)
    counts=${counts%, }
    if [ "$got" -eq "$want" ] && [ "$(grep -c . "$out/missed.txt")" -eq "$missed" ]; then
        echo "ok    $name: код $got; $counts"
    else
        echo "FAIL  $name: ожидались код $want и выживших $missed, получен код $got; $counts"
        tail -n 30 "$tmp/$name.log" | sed 's/^/      /'
        fail=1
    fi
    sed 's/^/      выжил: /' "$out/missed.txt"
}

extra=("$@")
# Код выхода 3 — были тайм-ауты; он же и при выживших, так что выживших
# считаем по missed.txt.
# Прежние тесты: библиотека и два файла тестов, без tests/mutants.rs.
mutants before 3 2 -- --lib --test frame --test crc_table
for m in "project/telemetry-frame/src/lib.rs:291:19: replace < with <= in encode" \
    "project/telemetry-frame/src/stream.rs:86:33: replace - with + in <impl Iterator for Frames<'a>>::size_hint"; do
    if ! grep -qxF -- "$m" "$tmp/before/mutants.out/missed.txt"; then
        echo "FAIL  before: среди выживших нет «$m»"
        fail=1
    fi
done
mutants after 3 0

if [ "$fail" -ne 0 ]; then
    echo "мутанты главы 31: есть расхождения"
    exit 1
fi
echo "мутанты главы 31: исходы совпали с текстом"
