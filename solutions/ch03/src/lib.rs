//! Эталонные решения задач главы 3.
//!
//! Задачи B и D собираются и без `std`: страж, транзакция и буферы приёма
//! пригодны для прошивки (`PinnedRx` требует только `alloc`). Задача A пользуется журналом из `ch03::drop_order`
//! и живёт только на хосте.
#![cfg_attr(not(test), no_std)]

extern crate alloc;

#[cfg(not(target_os = "none"))]
pub mod a_drop_order;
pub mod b_scope_guard;
pub mod c_abi;
pub mod d_self_ref;
