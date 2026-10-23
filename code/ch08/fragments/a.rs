// Задача 8.A. Библиотека `station`; места ⟨1⟩–⟨5⟩ — модули, из которых
// обращаются к элементам, ⟨6⟩ — другой крейт, подключивший `station`.

// ⟨1⟩ корень крейта
const CLOCK_HZ: u32 = 12_000_000;

pub mod hal {
    // ⟨2⟩ hal
    pub mod uart {
        // ⟨3⟩ hal::uart
        pub struct Uart {
            pub(crate) baud: u32,
            fifo: [u8; 16],
        }

        impl Uart {
            pub(super) fn reset(&mut self) {
                self.fifo = [0; 16];
                self.baud = super::super::CLOCK_HZ / 104;
            }
        }
    }

    mod dma {
        // ⟨4⟩ hal::dma
        pub struct Channel(u8);

        pub fn start(ch: &Channel, uart: &mut super::uart::Uart) {
            uart.reset();
            let _ = ch.0;
        }
    }

    pub use dma::Channel as DmaChannel;
}

mod app {
    // ⟨5⟩ app
    fn render() {}
}
