#!/usr/bin/env bash
# Собирает switch.c из введения к главе 2 и сверяет вывод программы и
# предупреждения компиляторов с тем, что сказано в тексте.
# Запуск: bash code/ch02/c/run.sh. Нужен gcc; если есть clang, проверяются
# и его предупреждения, иначе эта часть пропускается.
set -u
# Диагностика GCC зависит от локали: в C.UTF-8 имена берутся в кавычки
# ‘…’, в ru_RU.UTF-8 сообщения переведены. В тексте главы вывод локали C.
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

# warnings <компилятор> <файл.c> [флаги...]: диагностика компиляции с -O2,
# как в тексте. -fsyntax-only не годится: GCC ищет провал между ветками
# после разбора, на этапе GIMPLE.
warnings() {
    local cc="$1" src="$2"
    shift 2
    "$cc" -O2 "$@" -c -o /dev/null "$src" 2>&1
}

if ! "$CC" -O2 -o "$tmp/a.out" switch.c 2>"$tmp/cc.log"; then
    echo "FAIL  switch.c не собирается"
    sed 's/^/      /' "$tmp/cc.log"
    exit 1
fi
out=$("$tmp/a.out")
expect "mode_name молча возвращает «?»"   есть "mode_name(MODE_SLEEP) = ?" "$out"
expect "reply_len проваливается в 0x02"   есть "reply_len(0x01) = 8"       "$out"

w=$(warnings "$CC" switch.c -Wall)
expect "GCC -Wall: -Wswitch"              есть "switch.c:11:5: warning: enumeration value 'MODE_SLEEP' not handled in switch [-Wswitch]" "$w"
expect "GCC -Wall молчит о провале"       нет  "-Wimplicit-fallthrough" "$w"
w=$(warnings "$CC" switch.c -Wall -Wextra)
expect "GCC -Wextra: провал"              есть "switch.c:25:13: warning: this statement may fall through [-Wimplicit-fallthrough=]" "$w"
expect "GCC -Wextra: место провала"       есть "switch.c:26:5: note: here" "$w"

# Ветка default в mode_name: -Wswitch замолкает, -Wswitch-enum — нет.
sed 's/^    case MODE_TX:   return "tx";$/&\n    default:        break;/' switch.c >"$tmp/default.c"
expect "в копии есть ветка default"       есть "default:        break;" "$(cat "$tmp/default.c")"
w=$(warnings "$CC" "$tmp/default.c" -Wall -Wextra)
expect "GCC с default молчит о MODE_SLEEP" нет "MODE_SLEEP" "$w"
w=$(warnings "$CC" "$tmp/default.c" -Wall -Wextra -Wswitch-enum)
expect "GCC -Wswitch-enum видит MODE_SLEEP" есть "[-Wswitch-enum]" "$w"

if command -v "$CLANG" >/dev/null; then
    w=$(warnings "$CLANG" switch.c -Wall -Wextra)
    expect "clang -Wall -Wextra: -Wswitch"      есть "enumeration value 'MODE_SLEEP' not handled in switch" "$w"
    expect "clang -Wall -Wextra молчит о провале" нет "fallthrough" "$w"
    w=$(warnings "$CLANG" switch.c -Wimplicit-fallthrough)
    expect "clang -Wimplicit-fallthrough: провал" есть "[-Wimplicit-fallthrough]" "$w"
else
    echo "skip  clang: не найден"
fi

exit $fail
