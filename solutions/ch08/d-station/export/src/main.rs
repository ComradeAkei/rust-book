//! Выгрузка для заказчика: показания в градусах Фаренгейта.
use units::Temp;

fn main() {
    for raw in [2150, -405, 0] {
        println!("выгрузка: {}", Temp(raw).fahrenheit());
    }
}
