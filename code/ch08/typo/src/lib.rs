// ANCHOR: typo
#[cfg(feature = "dam")]
pub fn start_dma() {}

#[cfg(target_os = "linxu")]
pub fn epoll() {}
// ANCHOR_END: typo
