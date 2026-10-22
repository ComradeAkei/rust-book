// Раздел 8.7: то же, что main.rs, но крейты упомянуты в другом порядке.
fn main() {
    println!("{}", dma::station_reset() + uart::station_reset());
}
