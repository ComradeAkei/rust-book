//! Задача D: `include_svd!`. Оба UART lm3s6965 из одного файла SVD:
//! UART1 описан через `derivedFrom="UART0"`. Путь — от каталога этого
//! крейта.

// ANCHOR: invoke
crate::include_svd!("../../code/ch24/svd/lm3s6965.svd", "UART0");
crate::include_svd!("../../code/ch24/svd/lm3s6965.svd", "UART1");
// ANCHOR_END: invoke
