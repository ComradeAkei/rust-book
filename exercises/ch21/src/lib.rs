//! Заготовки задач главы 21. Условия — в разделе 21.7.
//!
//! - A: спинлок на `compare_exchange_weak` с Acquire/Release.
//! - B: справедливый ticket lock на `fetch_add`.
//! - C: проба на буфер записи (store buffering), Relaxed против SeqCst.
//! - D: lock-free стек Трайбера с версионной головой против ABA.
//!
//! Глава хостовая (потоки std).

pub mod a_spinlock;
pub mod b_ticket;
pub mod c_store_buffering;
pub mod d_treiber;
