//! Задача B: система частиц строками (AoS) и столбцами (SoA).

// ANCHOR: aos
/// Частица: 48 байт. Энергии нужны скорость и масса — 16 байт из 48,
/// шагу по времени — положение и скорость, 24 байта. Раскладка задана
/// (`repr(C)`), как в C: промахи кэша считаются по байтам частицы.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub mass: f32,
    pub charge: f32,
    pub id: u64,
    pub flags: u32,
    pub age: u32,
}

const _: () = assert!(size_of::<Particle>() == 48);

/// `n` частиц с разными скоростями и массами.
pub fn cloud(n: usize) -> Vec<Particle> {
    (0..n)
        .map(|i| {
            let f = i as f32;
            Particle {
                pos: [f, -f, 0.5 * f],
                vel: [(i % 7) as f32 - 3.0, (i % 5) as f32 - 2.0, (i % 3) as f32 - 1.0],
                mass: 1.0 + (i % 11) as f32,
                charge: if i % 2 == 0 { 1.0 } else { -1.0 },
                id: i as u64,
                flags: 0,
                age: 0,
            }
        })
        .collect()
}

/// Кинетическая энергия системы: сумма m·v²/2 в порядке частиц.
#[inline(never)]
pub fn energy_aos(ps: &[Particle]) -> f64 {
    let mut e = 0.0f64;
    for p in ps {
        let v2 = p.vel[0] * p.vel[0] + p.vel[1] * p.vel[1] + p.vel[2] * p.vel[2];
        e += f64::from(0.5 * p.mass * v2);
    }
    e
}

/// Шаг по времени: положение сдвигается на скорость × `dt`.
#[inline(never)]
pub fn step_aos(ps: &mut [Particle], dt: f32) {
    for p in ps {
        for k in 0..3 {
            p.pos[k] += p.vel[k] * dt;
        }
    }
}
// ANCHOR_END: aos

// ANCHOR: soa
/// Те же частицы столбцами. Поля выберите сами: тест обращается к
/// частицам только через методы.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Particles {}

impl Particles {
    pub fn from_aos(ps: &[Particle]) -> Particles {
        let _ = ps;
        todo!("30.B: from_aos")
    }

    pub fn to_aos(&self) -> Vec<Particle> {
        todo!("30.B: to_aos")
    }

    /// Та же сумма в том же порядке, что [`energy_aos`]: результат должен
    /// совпасть до бита. `#[inline(never)]` оставьте: cachegrind считает
    /// промахи по функциям.
    #[inline(never)]
    pub fn energy(&self) -> f64 {
        todo!("30.B: energy")
    }

    /// Шаг по времени, как [`step_aos`].
    #[inline(never)]
    pub fn step(&mut self, dt: f32) {
        let _ = dt;
        todo!("30.B: step")
    }
}
// ANCHOR_END: soa
