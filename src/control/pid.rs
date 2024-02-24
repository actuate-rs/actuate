pub struct PidController {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
    pub total_error: f64,
    pub last_error: f64,
    pub last_time: Option<u64>,
}

impl Default for PidController {
    fn default() -> Self {
        Self {
            kp: 0.5,
            ki: 0.1,
            kd: 0.2,
            total_error: 0.,
            last_error: 0.,
            last_time: None,
        }
    }
}

impl PidController {
    pub fn control(&mut self, time: u64, value: &mut f64, target: &f64) {
        let elapsed = match self.last_time {
            Some(last_time) => time - last_time,
            None => 1,
        };
        let elapsed_ms = (elapsed as f64).max(1.0);

        let error = *target - *value;
        let error_delta = (error - self.last_error) / elapsed_ms;
        self.total_error += error * elapsed_ms;
        self.last_error = error;
        self.last_time = Some(time);

        let p = self.kp * error;
        let i = self.ki * self.total_error;
        let d = self.kd * error_delta;
        *value = p + i + d;
    }
}
