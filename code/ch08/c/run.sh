#!/usr/bin/env bash
# Собирает C-примеры главы 8 и сверяет с текстом:
# - ifdef.c (раздел 8.3): опечатка в #ifdef молча выключает код, -Wundef
#   ловит только опечатку в #if;
# - uart.c, log.c, main.c (раздел 8.7): две глобальные переменные с одним
#   именем. По умолчанию (GCC 10+, -fno-common) компоновщик их отвергает, с
#   -fcommon молча сливает в одну, и журнал пишет текст в счётчик UART.
# Запуск: bash code/ch08/c/run.sh. Нужен gcc; если есть clang, проверяется
# и он.
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

# same <описание> <ожидаемое> <текст>
same() {
    if [ "$2" = "$3" ]; then
        echo "ok    $1"
    else
        echo "FAIL  $1"
        diff <(echo "$2") <(echo "$3") | sed 's/^/      /'
        fail=1
    fi
}

for cc in "$CC" "$CLANG"; do
    if ! command -v "$cc" >/dev/null; then
        echo "skip  $cc: не найден"
        continue
    fi
    w=$("$cc" -Wall -Wextra -Wundef -o "$tmp/ifdef" ifdef.c 2>&1)
    expect "ifdef.c, $cc: -Wundef видит опечатку в #if" есть "ifdef.c:12:5: warning:" "$w"
    expect "ifdef.c, $cc: опечатку в #ifdef не видит никто" нет "ifdef.c:7" "$w"
    same "ifdef.c, $cc: DMA выключен молча" "UART: приём опросом" "$("$tmp/ifdef")"
done

w=$("$CC" -O2 -Wall -Wextra -o "$tmp/def" uart.c log.c main.c 2>&1)
expect "по умолчанию: два определения rx_count отвергнуты" есть "multiple definition of \`rx_count'" "$w"

w=$("$CC" -O2 -Wall -Wextra -fcommon -o "$tmp/common" uart.c log.c main.c 2>&1)
same "-fcommon: ни одного предупреждения" "" "$w"
same "-fcommon: журнал пишет в счётчик" $'rx_count = 3\nrx_count = 16961' "$("$tmp/common")"

w=$("$CC" -O2 -fcommon -Wl,--warn-common -o "$tmp/warn" uart.c log.c main.c 2>&1)
expect "--warn-common: компоновщик предупреждает" есть "common of \`rx_count' overriding smaller common" "$w"

exit $fail
