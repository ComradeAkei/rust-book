#!/usr/bin/env bash
# Собирает samples.c из введения к главе 4 и сверяет с текстом: AddressSanitizer
# находит обращение к освобождённой памяти, а статические анализаторы GCC и
# clang молчат. Затем проверяет count.c и overlap.c из раздела 4.5: где GCC
# и clang оставляют запись счётчика — в цикле или после него, и что даёт
# нарушенный restrict.
# Запуск: bash code/ch04/c/run.sh. Нужен gcc с libasan; если есть clang и
# arm-none-eabi-gcc, проверяются и они, иначе эти части пропускаются.
set -u
export LC_ALL=C
cd "$(dirname "$0")"
CC=${CC:-gcc}
CLANG=${CLANG:-clang}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0

# expect <описание> <есть|нет> <подстрока> <текст>
expect() {
    if grep -qF -- "$3" <<<"$4"; then found=есть; else found=нет; fi
    if [ "$found" = "$2" ]; then
        echo "ok    $1"
    else
        echo "FAIL  $1: «$3» ожидалось: $2"
        echo "$4" | sed 's/^/      /'
        fail=1
    fi
}

if ! "$CC" -O1 -g -fsanitize=address -o "$tmp/asan" samples.c 2>"$tmp/cc.log"; then
    echo "FAIL  samples.c не собирается с -fsanitize=address"
    sed 's/^/      /' "$tmp/cc.log"
    exit 1
fi
out=$(ASAN_OPTIONS=detect_leaks=0 "$tmp/asan" 2>&1)
expect "ASan: обращение к освобождённой памяти" есть "ERROR: AddressSanitizer: heap-use-after-free" "$out"
expect "ASan: место обращения — main"           есть "in main" "$out"
expect "ASan: строка обращения"                 есть "samples.c:37" "$out"
expect "ASan: освобождено в realloc из push"    есть "in push" "$out"

if ! "$CC" -O2 -o "$tmp/plain" samples.c 2>"$tmp/cc.log"; then
    echo "FAIL  samples.c не собирается с -O2"
    sed 's/^/      /' "$tmp/cc.log"
    exit 1
fi
out=$("$tmp/plain" 2>&1)
expect "без ASan программа печатает число"     есть "максимум " "$out"
expect "без ASan это не максимум 1007"          нет "максимум 1007" "$out"

w=$("$CC" -O2 -Wall -Wextra -fanalyzer -c -o /dev/null samples.c 2>&1)
expect "GCC -Wall -Wextra -fanalyzer молчит"   нет "warning" "$w"

if command -v "$CLANG" >/dev/null; then
    w=$("$CLANG" --analyze -o /dev/null samples.c 2>&1)
    expect "clang --analyze молчит"             нет "warning" "$w"
else
    echo "skip  clang: не найден"
fi

# store <функция> <ассемблер файла> <регулярное выражение записи>: печатает
# «в цикле», если запись стоит между меткой и обратным переходом на неё,
# иначе «после цикла».
store() {
    sed -n "/^$1:/,/^\.Lfunc_end\|\.size[[:space:]]*$1,/p" <<<"$2" |
        awk -v st="$3" '
            { line[NR] = tolower($0) }
            /^\.L[A-Za-z0-9_]+:/ { l = $1; sub(/:.*/, "", l); pos[l] = NR }
            $1 ~ /^(b|j)[a-z.]*$/ && ($2 in pos) {
                for (i = pos[$2]; i < NR; i++) if (line[i] ~ st) found = 1
            }
            END { print (found ? "в цикле" : "после цикла") }'
}

# count <компилятор> <регулярное выражение записи> <флаги...>
count() {
    local cc=$1 st=$2
    shift 2
    local s
    s=$("$cc" -O2 -S -o - "$@" count.c 2>&1) || { echo "FAIL  $cc $*: $s"; fail=1; return; }
    expect "$cc $*: uint8_t, запись в цикле"        есть "в цикле" "$(store count_flags "$s" "$st")"
    expect "$cc $*: uint16_t, запись после цикла"   есть "после цикла" "$(store count_flags16 "$s" "$st")"
    expect "$cc $*: restrict, запись после цикла"   есть "после цикла" "$(store count_flags_restrict "$s" "$st")"
    asm=$s
}

# overlap.c: счётчик внутри данных. Без restrict ответ 0x7f, с restrict
# запись вынесена из цикла и ответ другой; санитайзеры молчат.
if ! "$CC" -O2 -o "$tmp/overlap" overlap.c count.c 2>"$tmp/cc.log"; then
    echo "FAIL  overlap.c не собирается"
    sed 's/^/      /' "$tmp/cc.log"
    exit 1
fi
out=$("$tmp/overlap" 2>&1)
expect "overlap: без restrict 0x7f, с restrict 0x7e" есть "без restrict 0x7f, с restrict 0x7e" "$out"
if "$CC" -O2 -g -fsanitize=address,undefined -o "$tmp/overlap-san" overlap.c count.c 2>"$tmp/cc.log"; then
    out=$("$tmp/overlap-san" 2>&1)
    expect "overlap: UBSan молчит"                 нет "runtime error" "$out"
    expect "overlap: ASan молчит"                  нет "AddressSanitizer" "$out"
else
    echo "FAIL  overlap.c не собирается с -fsanitize=address,undefined"
    sed 's/^/      /' "$tmp/cc.log"
    fail=1
fi

x86='mov[[:space:]]+dword ptr \[rdi\],'
arm='str[[:space:]]+r[0-9]+, \[r0\]'
count "$CC" "$x86" -masm=intel
if command -v "$CLANG" >/dev/null; then
    count "$CLANG" "$x86" -masm=intel
    # clang проверяет пересечение во время выполнения и векторизует цикл
    # для случая без пересечения.
    expect "clang: векторный цикл в count_flags"   есть "pcmpeqb" "$(sed -n '/^count_flags:/,/^\.Lfunc_end/p' <<<"$asm")"
    count "$CLANG" "$arm" --target=thumbv7em-none-eabi -mcpu=cortex-m4
else
    echo "skip  clang: не найден"
fi
if command -v arm-none-eabi-gcc >/dev/null; then
    count arm-none-eabi-gcc "$arm" -mcpu=cortex-m4 -mthumb
else
    echo "skip  arm-none-eabi-gcc: не найден"
fi

exit $fail
