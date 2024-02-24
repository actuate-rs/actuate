pub struct Gain {
    kp: f64,
}

impl Gain {
    pub fn new(kp: f64) -> Self {
        Self { kp }
    }
}

impl Gain {
    pub fn gain(&self, value: &mut f64) {
        *value *= self.kp;
    }
}
