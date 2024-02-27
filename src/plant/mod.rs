pub struct PendulumPlant {
    pub length: f64,
    pub gravity: f64,
    pub damping: f64,
    pub angle: f64,
    pub angular_velocity: f64,
    pub last_time: Option<u64>,
}

impl Default for PendulumPlant {
    fn default() -> Self {
        Self {
            length: 10.,
            gravity: 9.81,
            damping: 0.5,
            angle: 0.,
            angular_velocity: 0.,
            last_time: None,
        }
    }
}

impl PendulumPlant {
    pub fn update(&mut self, time: u64, torque: f64) {
        let dt = if let Some(last_time) = self.last_time {
            (time - last_time) as f64
        } else {
            1.
        };

        self.last_time = Some(time);

        let angular_acceleration = (-self.gravity / self.length * self.angle.sin()
            - self.damping * self.angular_velocity
            + torque)
            / self.length;

        self.angular_velocity += angular_acceleration * dt;
        self.angle += self.angular_velocity * dt;
    }
}
