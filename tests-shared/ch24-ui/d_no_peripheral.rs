// Периферии нет в файле: ошибка на её имени со списком тех, что есть.
// Путь — от каталога проекта, который trybuild собирает в target/tests/trybuild.
tasks_macros::include_svd!("../../../../code/ch24/svd/lm3s6965.svd", "UART7");

fn main() {}
