#!/usr/bin/env bash
# Сборка ядер Linux 7.0 для главы 29: обычного и отладочного (KASAN,
# kmemleak, lockdep). Результат — каталоги `make O=…`
#   target/ch29-kernel/build        tinyconfig + ch29.config
#   target/ch29-kernel/build-debug  tinyconfig + ch29.config + ch29-debug.config
# Их находят тесты главы (code/ch29/src/host.rs); другой путь задают
# переменные CH29_KERNEL и CH29_KERNEL_DEBUG.
#
# Нужны: clang, lld и llvm 18 (сборка с LLVM=1), make, flex, bison, bc,
# libelf-dev, cpio; rustc из rust-toolchain.toml с компонентом rust-src и
# bindgen-cli 0.72 (`cargo install --locked bindgen-cli@0.72.1`). Чего не
# хватает, скажет `make rustavailable` до начала сборки.
#
# Исходники — архив linux_7.0.0.orig.tar.gz из Ubuntu: ядро 7.0 без
# патчей дистрибутива; контрольная сумма ниже. Уже скачанный архив можно
# указать в CH29_LINUX_TARBALL. Повторный запуск пересобирает только то,
# что изменилось.
#
#   bash code/ch29/kernel/build.sh            оба ядра
#   bash code/ch29/kernel/build.sh plain      только обычное
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
out=${CH29_KERNEL_DIR:-$root/target/ch29-kernel}
url=https://archive.ubuntu.com/ubuntu/pool/main/l/linux/linux_7.0.0.orig.tar.gz
sha256=c6343795c8c7e4feff55f5b309365c8e0eaf49cd6abcb412e32315313f149536
tarball=${CH29_LINUX_TARBALL:-$out/linux_7.0.0.orig.tar.gz}
src=$out/linux-7.0
jobs=${JOBS:-$(nproc)}
[ $# -gt 0 ] || set -- plain debug

# Ядро и модули собирает тот же rustc, что и книгу.
toolchain=$(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$root/rust-toolchain.toml")
export RUSTUP_TOOLCHAIN=$toolchain

mkdir -p "$out"
if [ ! -f "$tarball" ]; then
  echo "загрузка $url"
  curl -fL --retry 4 -o "$tarball.part" "$url"
  mv "$tarball.part" "$tarball"
fi
echo "$sha256  $tarball" | sha256sum -c --quiet
if [ ! -f "$src/.ch29-unpacked" ]; then
  rm -rf "$src"
  tar -C "$out" -xzf "$tarball"
  touch "$src/.ch29-unpacked"
fi

make -s -C "$src" LLVM=1 rustavailable

for flavor in "$@"; do
  case $flavor in
    plain) build=$out/build; frags=("$here/ch29.config") ;;
    debug) build=$out/build-debug; frags=("$here/ch29.config" "$here/ch29-debug.config") ;;
    *) echo "неизвестное ядро: $flavor (plain или debug)" >&2; exit 2 ;;
  esac
  mkdir -p "$build"
  make -s -C "$src" ARCH=x86_64 LLVM=1 O="$build" tinyconfig
  "$src/scripts/kconfig/merge_config.sh" -m -O "$build" "$build/.config" "${frags[@]}" > "$build/merge_config.log"
  make -s -C "$src" ARCH=x86_64 LLVM=1 O="$build" olddefconfig
  # Kconfig молча выкидывает опции с неудовлетворёнными зависимостями:
  # проверяем, что всё заказанное включено и выключено.
  missing=0
  for o in $(grep -h '^CONFIG_' "${frags[@]}" | sed 's/=.*//'); do
    grep -q "^$o=" "$build/.config" || { echo "$flavor: не включено: $o" >&2; missing=1; }
  done
  for o in $(grep -h '^# CONFIG_.* is not set' "${frags[@]}" | awk '{print $2}'); do
    if grep -q "^$o=" "$build/.config"; then echo "$flavor: не выключено: $o" >&2; missing=1; fi
  done
  [ $missing = 0 ] || exit 1
  echo "сборка: $flavor → $build"
  start=$(date +%s)
  make -s -C "$src" ARCH=x86_64 LLVM=1 O="$build" -j"$jobs"
  echo "$flavor: $(( $(date +%s) - start )) с, $(du -sh "$build" | cut -f1), bzImage $(du -h "$build/arch/x86/boot/bzImage" | cut -f1)"
done
