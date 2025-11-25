//! Game simulation module
//!
//! Handles game state, logic, physics, and entities.

// pub mod tictactoe;

use std::any::Any;

/// Trait that all game simulations must implement
///
/// This allows the World to contain and manage multiple different game systems
/// in a pluggable way. Each simulation is responsible for its own state and logic.
pub trait Simulation {
    /// Updates the simulation by one tick
    ///
    /// # Arguments
    /// * `delta_time` - Time elapsed since last tick in seconds
    fn tick(&mut self, delta_time: f32);

    /// Resets the simulation to its initial state
    fn reset(&mut self);

    /// Returns the name/identifier of this simulation
    fn name(&self) -> &str;

    /// Returns true if the simulation is currently active
    fn is_active(&self) -> bool {
        true
    }

    /// Allows downcasting to concrete types for specific operations
    ///
    /// This enables type-safe access to simulation-specific methods
    fn as_any(&self) -> &dyn Any;

    /// Mutable version of as_any for type-safe mutable access
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

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
    /// Collection of all active simulations
    simulations: Vec<Box<dyn Simulation>>,
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

        // Update all active simulations
        for sim in &mut self.simulations {
            if sim.is_active() {
                sim.tick(scaled_delta);
            }
        }
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

    /// Adds a simulation to the world
    pub fn add_simulation(&mut self, sim: Box<dyn Simulation>) {
        self.simulations.push(sim);
    }

    /// Returns a reference to all simulations
    pub fn simulations(&self) -> &[Box<dyn Simulation>] {
        &self.simulations
    }

    /// Returns a mutable reference to all simulations
    pub fn simulations_mut(&mut self) -> &mut [Box<dyn Simulation>] {
        &mut self.simulations
    }

    /// Gets a reference to a specific simulation by name
    pub fn get_simulation(&self, name: &str) -> Option<&Box<dyn Simulation>> {
        self.simulations.iter().find(|s| s.name() == name)
    }

    /// Gets a mutable reference to a specific simulation by name
    pub fn get_simulation_mut(&mut self, name: &str) -> Option<&mut Box<dyn Simulation>> {
        self.simulations.iter_mut().find(|s| s.name() == name)
    }

    /// Gets a typed reference to a specific simulation
    ///
    /// # Example
    /// ```ignore
    /// if let Some(ttt) = world.get_simulation_typed::<TicTacToe>("tictactoe") {
    ///     // Use TicTacToe-specific methods
    /// }
    /// ```
    pub fn get_simulation_typed<T: 'static>(&self, name: &str) -> Option<&T> {
        self.get_simulation(name)
            .and_then(|s| s.as_any().downcast_ref::<T>())
    }

    /// Gets a mutable typed reference to a specific simulation
    pub fn get_simulation_typed_mut<T: 'static>(&mut self, name: &str) -> Option<&mut T> {
        self.get_simulation_mut(name)
            .and_then(|s| s.as_any_mut().downcast_mut::<T>())
    }

    /// Resets all simulations to their initial state
    pub fn reset_all_simulations(&mut self) {
        for sim in &mut self.simulations {
            sim.reset();
        }
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
            simulations: Vec::new(),
        }
    }
}
