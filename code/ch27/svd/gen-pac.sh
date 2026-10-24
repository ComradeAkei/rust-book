#!/usr/bin/env bash
# Раздел 27.3: строит крейт code/ch27-pac из lm3s6965.svd. Нужен svd2rust
# 0.37.1 в PATH; CI запускает скрипт и проверяет, что результат совпадает с
# тем, что лежит в репозитории (git diff --exit-code code/ch27-pac).
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
pac="$root/code/ch27-pac"
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT

svd2rust --version | grep -q '^svd2rust 0\.37\.1 ' || {
    echo "нужен svd2rust 0.37.1, а в PATH: $(svd2rust --version)" >&2
    exit 1
}
svd2rust -i "$here/lm3s6965.svd" --target cortex-m -o "$out" --log warn \
    --reexport-core-peripherals --reexport-interrupt
# svd2rust пишет код одной строкой на элемент; rustfmt с настройками книги
# делает его читаемым и стабильным для git diff. Edition 2021: в
# сгенерированном коде #[no_mangle] и extern "C" без unsafe.
rustfmt --edition 2021 --config-path "$root/rustfmt.toml" "$out/lib.rs" "$out/build.rs"
cp "$out/lib.rs" "$pac/src/lib.rs"
cp "$out/build.rs" "$out/device.x" "$pac/"
