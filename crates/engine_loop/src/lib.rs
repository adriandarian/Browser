use std::time::{Duration, Instant};

use tracing::trace;

#[derive(Debug, Clone, Copy)]
pub struct FrameMetrics {
    pub dt_seconds: f32,
    pub fps: f32,
    pub frame_number: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct TickStep {
    pub tick_count: u32,
    pub metrics: FrameMetrics,
}

#[derive(Debug)]
pub struct Scheduler {
    tick_interval: Duration,
    max_ticks_per_frame: u32,
    last_frame_start: Instant,
    accumulator: Duration,
    frame_number: u64,
}

impl Scheduler {
    pub fn with_hz(tick_hz: f32) -> Self {
        let safe_hz = tick_hz.max(1.0);
        let tick_interval = Duration::from_secs_f32(1.0 / safe_hz);
        Self {
            tick_interval,
            max_ticks_per_frame: 8,
            last_frame_start: Instant::now(),
            accumulator: Duration::ZERO,
            frame_number: 0,
        }
    }

    pub fn begin_frame(&mut self) -> TickStep {
        let now = Instant::now();
        let dt = now.saturating_duration_since(self.last_frame_start);
        self.last_frame_start = now;

        self.accumulator = self.accumulator.saturating_add(dt);

        let mut tick_count = 0;
        while self.accumulator >= self.tick_interval && tick_count < self.max_ticks_per_frame {
            self.accumulator = self.accumulator.saturating_sub(self.tick_interval);
            tick_count += 1;
        }

        if tick_count == self.max_ticks_per_frame && self.accumulator >= self.tick_interval {
            trace!("scheduler clamped fixed updates for frame");
            self.accumulator = Duration::ZERO;
        }

        self.frame_number = self.frame_number.wrapping_add(1);

        let dt_seconds = dt.as_secs_f32().max(f32::EPSILON);
        let fps = 1.0 / dt_seconds;
        let metrics = FrameMetrics {
            dt_seconds,
            fps,
            frame_number: self.frame_number,
        };

        TickStep {
            tick_count,
            metrics,
        }
    }

    pub fn tick_interval(&self) -> Duration {
        self.tick_interval
    }
}
