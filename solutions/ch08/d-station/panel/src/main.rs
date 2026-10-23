//! Табло станции: показания в градусах Цельсия.
use units::Temp;

fn main() {
    for raw in [2150, -405, 0] {
        println!("табло: {}", Temp(raw).celsius());
    }
}
