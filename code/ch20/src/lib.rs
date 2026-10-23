//! Глава 20: потоки, Send и Sync. Как Rust даёт настоящую многопоточность и
//! при этом на уровне типов запрещает гонки данных.
//!
//! - 20.1 `spawn`/`join` и перемещение владения в поток (`spawn`);
//! - 20.2 маркеры `Send` и `Sync` (`marker`);
//! - 20.3 общее состояние: `Arc`, `Mutex`, `RwLock`, отравление (`shared`);
//! - 20.4 области потоков `thread::scope` (`scoped`);
//! - 20.5 каналы `mpsc`, `Condvar`, `park`/`unpark` (`channels`);
//! - 20.6 цена: ложное разделение кэш-линий и выравнивание (`padding`).
//!
//! Глава хостовая (std-потоки), под Cortex-M не идёт; Miri исполняет её и
//! проверяет на гонки данных.

pub mod channels;
pub mod marker;
pub mod padding;
pub mod scoped;
pub mod shared;
pub mod spawn;
