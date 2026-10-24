//! Задача 31.D, часть 1: тесты хранилища проекта (`project/telemetry-store`),
//! которые убивают мутантов, выживших после
//! `cargo mutants -p telemetry-store`. Допишите их сюда. Проверка — общий
//! тест `d_store_mutants_killed`, он запускает
//!
//! ```text
//! cargo mutants -p telemetry-store --test-package telemetry-store,ch31-exercises \
//!     --baseline skip -- --test store --test d_store
//! ```
//!
//! и ждёт, что выживших не останется. Счётчики распределителя проекта
//! работают, только если в этом файле стоит
//! `#[global_allocator] static COUNTED: telemetry_store::Counted = telemetry_store::Counted;`.
