/// Animation system for UltraWM.
///
/// Uses a vsync-aligned loop driven by DwmFlush() (LeopardWM-proven approach).
/// Animates: window positions, focus ring changes, workspace transitions.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct Spring {
    pub stiffness: f32,
    pub damping: f32,
    pub mass: f32,
}

impl Default for Spring {
    fn default() -> Self {
        Self {
            stiffness: 180.0,
            damping: 20.0,
            mass: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SpringValue {
    pub current: f32,
    pub target: f32,
    pub velocity: f32,
    pub spring: Spring,
}

impl SpringValue {
    pub fn new(value: f32) -> Self {
        Self {
            current: value,
            target: value,
            velocity: 0.0,
            spring: Spring::default(),
        }
    }

    pub fn with_spring(mut self, spring: Spring) -> Self {
        self.spring = spring;
        self
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Step the spring simulation by dt seconds. Returns the new value.
    pub fn step(&mut self, dt: f32) -> f32 {
        let displacement = self.current - self.target;
        let spring_force = -self.spring.stiffness * displacement;
        let damping_force = -self.spring.damping * self.velocity;
        let acceleration = (spring_force + damping_force) / self.spring.mass;

        self.velocity += acceleration * dt;
        self.current += self.velocity * dt;

        // Snap if very close
        if self.current.abs() < 0.5 && self.velocity.abs() < 0.5 {
            self.current = self.target;
            self.velocity = 0.0;
        }

        self.current
    }

    pub fn is_settled(&self) -> bool {
        self.current.abs() < 0.5 && self.velocity.abs() < 0.5
    }
}

#[derive(Debug)]
pub struct AnimationLoop {
    pub last_tick: Instant,
    pub running: bool,
}

impl AnimationLoop {
    pub fn new() -> Self {
        Self {
            last_tick: Instant::now(),
            running: true,
        }
    }

    pub fn tick(&mut self) -> Duration {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick);
        self.last_tick = now;
        dt
    }

    /// Spin the vsync loop to let animations settle.
    pub fn vsync_wait(&self) {
        // Vsync-aligned wait — placeholder for DwmFlush or similar
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
