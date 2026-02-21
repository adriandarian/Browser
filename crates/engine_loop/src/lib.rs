use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct FrameTiming {
    pub frame_index: u64,
    pub dt_seconds: f32,
    pub fps: f32,
    pub fixed_updates: u32,
}

#[derive(Debug)]
pub struct Scheduler {
    fixed_step: Duration,
    max_updates_per_frame: u32,
    accumulator: Duration,
    frame_index: u64,
    second_accumulator: Duration,
    frames_this_second: u32,
    fps: f32,
}

impl Scheduler {
    pub fn new(tick_hz: u32) -> Self {
        let tick_hz = tick_hz.max(1);
        Self {
            fixed_step: Duration::from_secs_f64(1.0 / f64::from(tick_hz)),
            max_updates_per_frame: 8,
            accumulator: Duration::ZERO,
            frame_index: 0,
            second_accumulator: Duration::ZERO,
            frames_this_second: 0,
            fps: 0.0,
        }
    }

    pub fn with_max_updates_per_frame(mut self, max_updates_per_frame: u32) -> Self {
        self.max_updates_per_frame = max_updates_per_frame.max(1);
        self
    }

    pub fn fixed_step(&self) -> Duration {
        self.fixed_step
    }

    pub fn advance(&mut self, dt: Duration) -> FrameTiming {
        self.accumulator = self.accumulator.saturating_add(dt);

        let mut updates = 0;
        while self.accumulator >= self.fixed_step && updates < self.max_updates_per_frame {
            self.accumulator -= self.fixed_step;
            updates += 1;
        }

        self.frame_index = self.frame_index.wrapping_add(1);
        self.frames_this_second = self.frames_this_second.saturating_add(1);
        self.second_accumulator = self.second_accumulator.saturating_add(dt);

        if self.second_accumulator >= Duration::from_secs(1) {
            let secs = self.second_accumulator.as_secs_f32();
            if secs > 0.0 {
                self.fps = self.frames_this_second as f32 / secs;
            }
            self.frames_this_second = 0;
            self.second_accumulator = Duration::ZERO;
        }

        FrameTiming {
            frame_index: self.frame_index,
            dt_seconds: dt.as_secs_f32(),
            fps: self.fps,
            fixed_updates: updates,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_fixed_updates() {
        let mut scheduler = Scheduler::new(60);
        let timing = scheduler.advance(Duration::from_millis(16));
        assert_eq!(timing.fixed_updates, 0);

        let timing = scheduler.advance(Duration::from_millis(17));
        assert_eq!(timing.fixed_updates, 1);
    }

    #[test]
    fn reports_non_zero_fps() {
        let mut scheduler = Scheduler::new(60);
        let mut last = FrameTiming {
            frame_index: 0,
            dt_seconds: 0.0,
            fps: 0.0,
            fixed_updates: 0,
        };

        for _ in 0..65 {
            last = scheduler.advance(Duration::from_millis(16));
        }

        assert!(last.fps > 0.0);
    }
}
