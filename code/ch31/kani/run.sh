#!/usr/bin/env bash
# Разделы 31.7 и 31.10: доказательства Kani и их вердикты из текста главы.
# В ch31 два доказательства не проходят, и в этом их смысл: середина
# отрезка `(a + b) / 2` переполняется, а первая версия разбора команд
# паникует на калибровке из девяти точек — той, которую фаззер на сырых
# байтах не нашёл. Для неё Kani печатает контрпример как тест (concrete
# playback): это ровно команда на 9 точек. С развёрткой меньше нужной
# доказательство исправленного разбора проваливает проверку развёртки, а
# доказательство с противоречивыми ограничениями «проходит» и для версии с
# ошибкой — это видно только по недостижимому kani::cover!.
# Остальные доказательства
# проходят: исправленные функции главы, разбор кадров проекта
# (telemetry-frame) и очередь SPSC главы 21 для всех N от 2 до 8.
# Решения задач 31.C и 31.D (ch31-solutions): кольцо со свободно
# бегущими индексами из заготовки переполняется на `head + 1`; после
# замены на wrapping_* портит порядок при N, не делящем 2⁶⁴ (3, 5, 6, 7);
# исправленное проходит при всех N от 1 до 8. Круговой обход TimeSync
# доказывается и для версии с ошибкой спецификации.
# Запуск: bash code/ch31/kani/run.sh [флаги cargo kani, например
# --target-dir DIR]. Нужен Kani: cargo install kani-verifier и
# cargo kani setup.
set -u
# Вывод cargo сверяется с текстом: без кодов цвета, даже если снаружи
# задан CARGO_TERM_COLOR=always (так в CI).
export LC_ALL=C CARGO_TERM_COLOR=never
cd "$(dirname "$0")/../../.." || exit 2
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0

# prove <пакет> <флаги cargo kani...>: все доказательства пакета разом.
prove() {
    local pkg=$1
    shift
    cargo kani -p "$pkg" --output-format terse "$@" >"$tmp/$pkg.log" 2>&1
    echo "      $pkg: $(grep -E '^Complete - ' "$tmp/$pkg.log")"
}

# expect <пакет> <доказательство> <SUCCESSFUL|FAILED> <проваленная проверка или ->
expect() {
    local pkg=$1 name=$2 want=$3 needle=$4 got checks
    awk -v name="$name" '
        /^Checking harness / { on = ($3 ~ ("(^|::)" name "\\.\\.\\.$")) }
        /^Manual Harness Summary:/ { on = 0 }
        on' "$tmp/$pkg.log" >"$tmp/one"
    got=$(sed -n 's/^VERIFICATION:- //p' "$tmp/one")
    checks=$(sed -n 's/^Failed Checks: //p' "$tmp/one")
    if [ "$got" = "$want" ] && { [ "$needle" = - ] || grep -qxF -- "$needle" <<<"$checks"; }; then
        echo "ok    $pkg $name: $got${checks:+ ($checks)}, $(sed -n 's/^Verification Time: //p' "$tmp/one")"
    else
        echo "FAIL  $pkg $name: ожидалось $want${needle#-}, получено ${got:-ничего}"
        sed -n '1,40p' "$tmp/one" | sed 's/^/      /'
        fail=1
    fi
}

prove ch31 -Z concrete-playback --concrete-playback=print "$@"
expect ch31 midpoint_v1_no_overflow FAILED "attempt to add with overflow"
expect ch31 midpoint_is_exact SUCCESSFUL -
expect ch31 crc_table_step_matches_bitwise SUCCESSFUL -
expect ch31 parse_v1_never_panics FAILED \
    "index out of bounds: the length is less than or equal to the given index"
expect ch31 parse_never_panics SUCCESSFUL -
# Пустое доказательство: ограничения противоречат друг другу, Kani
# отвечает SUCCESSFUL и для parse_v1, а kani::cover! — недостижим.
expect ch31 parse_v1_vacuous SUCCESSFUL -
if grep -qxF ' ** 0 of 1 cover properties satisfied (1 unreachable)' "$tmp/one"; then
    echo "ok    ch31 parse_v1_vacuous: cover недостижим — входов нет"
else
    echo "FAIL  ch31 parse_v1_vacuous: нет строки о недостижимом cover"
    fail=1
fi

# Контрпример к parse_v1_never_panics: значения kani::any() по порядку —
# 43 байта буфера, потом длина. Иначе как маркер, калибровка, длина 39,
# датчик и n = 9 паники не бывает: 3 + 4·n ≤ 39 и n > 8.
awk '/^fn kani_concrete_playback_parse_v1_never_panics/ { on = 1 }
     on && /^ *vec!\[/ { gsub(/[^0-9,]/, ""); sub(/,$/, ""); print }
     on && /concrete_playback_run/ { exit }' "$tmp/ch31.log" >"$tmp/vals"
mapfile -t vals <"$tmp/vals"
if [ "${#vals[@]}" -eq 44 ] && [ "${vals[0]},${vals[1]},${vals[2]},${vals[5]}" = "90,3,39,9" ] &&
    [ "${vals[43]}" = "43,0,0,0,0,0,0,0" ]; then
    echo "ok    контрпример: 5a 03 27, датчик $(( vals[3] | vals[4] << 8 )), 9 точек, длина 43"
else
    echo "FAIL  контрпример к parse_v1_never_panics не тот: ${vals[*]:-нет}"
    grep -A 20 -F 'Concrete playback unit test for `verify::proofs::parse_v1_never_panics`' "$tmp/ch31.log" |
        sed 's/^/      /'
    fail=1
fi

# Та же проверка исправленного разбора с развёрткой 20 — меньше, чем нужно
# циклу CRC по 41 байту: Kani не говорит «доказано», а проваливает
# проверку развёртки, остальные проверки остаются неопределёнными.
cargo kani -p ch31 --harness verify::proofs::parse_never_panics --exact --unwind 20 \
    --output-format terse "$@" >"$tmp/ch31-unwind20.log" 2>&1
expect ch31-unwind20 parse_never_panics FAILED "unwinding assertion loop 1"

prove telemetry-frame "$@"
expect telemetry-frame decode_any_buffer_up_to_24 SUCCESSFUL -
expect telemetry-frame frames_step_makes_progress SUCCESSFUL -

prove ch21 "$@"
for n in 2 3 4 5 6 7 8; do
    expect ch21 "step_$n" SUCCESSFUL -
done

prove ch31-solutions "$@"
expect ch31-solutions original::n3 FAILED "attempt to add with overflow"
for n in 1 2 4 8; do
    expect ch31-solutions "wrapping::n$n" SUCCESSFUL -
done
for n in 3 5 6 7; do
    expect ch31-solutions "wrapping::n$n" FAILED "assertion failed: q.pop() == Some(i as u8)"
done
for n in 1 2 3 4 5 6 7 8; do
    expect ch31-solutions "fixed::n$n" SUCCESSFUL -
done
expect ch31-solutions original_round_trip SUCCESSFUL -
expect ch31-solutions fixed_round_trip SUCCESSFUL -

if [ "$fail" -ne 0 ]; then
    echo "доказательства главы 31: есть расхождения"
    exit 1
fi
echo "доказательства главы 31: вердикты совпали с текстом"
