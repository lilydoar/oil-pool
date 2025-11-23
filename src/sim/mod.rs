//! Game simulation module
//!
//! Handles game state, logic, physics, and entities.

/// Game world state
pub struct World {
    /// Total number of simulation ticks elapsed
    tick_count: u64,
    /// Total simulation time elapsed in seconds
    sim_time: f64,
    /// Time scale multiplier (1.0 = normal speed, 0.0 = paused, 2.0 = 2x speed)
    time_scale: f32,
    /// Accumulator for fixed timestep simulation
    timestep_accumulator: f32,
    /// Whether the simulation is paused
    paused: bool,
    /// Random number generator seed
    rng_seed: u64,
}

impl World {
    /// Creates a new game world with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder method to set a specific RNG seed
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng_seed = seed;
        self
    }

    /// Builder method to set the time scale
    pub fn with_time_scale(mut self, scale: f32) -> Self {
        self.time_scale = scale.max(0.0);
        self
    }

    /// Builder method to set the paused state
    pub fn with_paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    /// Updates the world simulation by one tick
    pub fn tick(&mut self, delta_time: f32) {
        if self.paused {
            return;
        }

        let scaled_delta = delta_time * self.time_scale;
        self.tick_count += 1;
        self.sim_time += scaled_delta as f64;
        self.timestep_accumulator += scaled_delta;

        // TODO: Implement simulation logic
    }

    /// Returns the current tick count
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    /// Returns the total simulation time in seconds
    pub fn sim_time(&self) -> f64 {
        self.sim_time
    }

    /// Sets the time scale multiplier
    pub fn set_time_scale(&mut self, scale: f32) {
        self.time_scale = scale.max(0.0);
    }

    /// Returns the current time scale
    pub fn time_scale(&self) -> f32 {
        self.time_scale
    }

    /// Pauses the simulation
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resumes the simulation
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Toggles pause state
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Returns whether the simulation is paused
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Returns the RNG seed
    pub fn rng_seed(&self) -> u64 {
        self.rng_seed
    }

    /// Returns the timestep accumulator value
    pub fn timestep_accumulator(&self) -> f32 {
        self.timestep_accumulator
    }

    /// Consumes a fixed timestep from the accumulator
    pub fn consume_timestep(&mut self, timestep: f32) {
        self.timestep_accumulator -= timestep;
    }
}

impl Default for World {
    fn default() -> Self {
        Self {
            tick_count: 0,
            sim_time: 0.0,
            time_scale: 1.0,
            timestep_accumulator: 0.0,
            paused: false,
            rng_seed: rand::random(),
        }
    }
}
