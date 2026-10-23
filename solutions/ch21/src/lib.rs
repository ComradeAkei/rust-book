//! Эталонные решения задач главы 21. Условия — в разделе 21.7, разбор — в
//! приложении E.
//!
//! - A: спинлок на `compare_exchange_weak` с Acquire/Release.
//! - B: справедливый ticket lock на `fetch_add`.
//! - C: проба на буфер записи (store buffering) с измерением частоты исхода
//!   `(0, 0)` при Relaxed против SeqCst.
//! - D: lock-free стек Трайбера с версионной головой против ABA.
//!
//! Глава хостовая (потоки std); под Cortex-M и Miri тяжёлые тесты не идут.

pub mod a_spinlock;
pub mod b_ticket;
pub mod c_store_buffering;
pub mod d_treiber;
