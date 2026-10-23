// Раздел 8.7: оба вызова идут в одно и то же определение.
// ANCHOR: main
fn main() {
    println!("{}", uart::station_reset() + dma::station_reset());
}
// ANCHOR_END: main
