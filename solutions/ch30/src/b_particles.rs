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
/// Те же частицы столбцами: по вектору на каждое поле.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Particles {
    pub pos: [Vec<f32>; 3],
    pub vel: [Vec<f32>; 3],
    pub mass: Vec<f32>,
    pub charge: Vec<f32>,
    pub id: Vec<u64>,
    pub flags: Vec<u32>,
    pub age: Vec<u32>,
}

impl Particles {
    pub fn from_aos(ps: &[Particle]) -> Particles {
        let col = |f: &dyn Fn(&Particle) -> f32| ps.iter().map(f).collect::<Vec<f32>>();
        Particles {
            pos: [col(&|p| p.pos[0]), col(&|p| p.pos[1]), col(&|p| p.pos[2])],
            vel: [col(&|p| p.vel[0]), col(&|p| p.vel[1]), col(&|p| p.vel[2])],
            mass: col(&|p| p.mass),
            charge: col(&|p| p.charge),
            id: ps.iter().map(|p| p.id).collect(),
            flags: ps.iter().map(|p| p.flags).collect(),
            age: ps.iter().map(|p| p.age).collect(),
        }
    }

    pub fn to_aos(&self) -> Vec<Particle> {
        (0..self.mass.len())
            .map(|i| Particle {
                pos: [self.pos[0][i], self.pos[1][i], self.pos[2][i]],
                vel: [self.vel[0][i], self.vel[1][i], self.vel[2][i]],
                mass: self.mass[i],
                charge: self.charge[i],
                id: self.id[i],
                flags: self.flags[i],
                age: self.age[i],
            })
            .collect()
    }

    /// Та же сумма в том же порядке, что [`energy_aos`]: результат
    /// совпадает до бита.
    #[inline(never)]
    pub fn energy(&self) -> f64 {
        let [vx, vy, vz] = &self.vel;
        let n = self.mass.len();
        let (vx, vy, vz, m) = (&vx[..n], &vy[..n], &vz[..n], &self.mass[..n]);
        let mut e = 0.0f64;
        for i in 0..n {
            let v2 = vx[i] * vx[i] + vy[i] * vy[i] + vz[i] * vz[i];
            e += f64::from(0.5 * m[i] * v2);
        }
        e
    }

    #[inline(never)]
    pub fn step(&mut self, dt: f32) {
        for k in 0..3 {
            for (x, v) in self.pos[k].iter_mut().zip(&self.vel[k]) {
                *x += v * dt;
            }
        }
    }
}
// ANCHOR_END: soa
