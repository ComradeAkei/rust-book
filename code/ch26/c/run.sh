#!/usr/bin/env bash
# Раздел 26.1: что приносит в прошивку snprintf из newlib. Раздел 26.5:
# когда memset на C вызывает саму себя.
# Нужны arm-none-eabi-gcc и libnewlib-arm-none-eabi (задание thumb в CI).
set -eu
cd "$(dirname "$0")"
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
fail=0
for specs in "--specs=nosys.specs" "--specs=nano.specs --specs=nosys.specs"; do
    # Предупреждения компоновщика о заглушках _close, _fstat и прочих из
    # nosys — часть опыта, но не его предмет.
    # shellcheck disable=SC2086
    arm-none-eabi-gcc -mcpu=cortex-m3 -mthumb -O2 $specs snprintf.c -o "$out/a.elf" 2>"$out/ld.log"
    syms=$(arm-none-eabi-nm "$out/a.elf")
    for s in _malloc_r _sbrk; do
        if grep -q " T $s\$" <<<"$syms"; then
            echo "ok    $specs: $s в прошивке"
        else
            echo "FAIL  $specs: нет $s"
            fail=1
        fi
    done
done

# Раздел 26.5: своя memset циклом и ключи, которые решают, позовёт ли она
# саму себя.
for flags in "-O2" "-O2 -nostdlib" "-O2 -ffreestanding" "-O2 -fno-builtin"; do
    # shellcheck disable=SC2086
    arm-none-eabi-gcc -mcpu=cortex-m3 -mthumb $flags -S -o "$out/memset.s" memset.c
    calls=$(sed -n '/^memset:/,/\.size/p' "$out/memset.s" | grep -c 'bl[[:space:]]*memset$' || true)
    case "$flags" in
        "-O2" | "-O2 -nostdlib") want=1 ;;
        *) want=0 ;;
    esac
    if [ "$calls" = "$want" ]; then
        echo "ok    memset $flags: вызовов самой себя $calls"
    else
        echo "FAIL  memset $flags: вызовов самой себя $calls, ожидалось $want"
        fail=1
    fi
done
exit $fail
