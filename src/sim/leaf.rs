//! Leaf placement simulation
//!
//! Places leaves organically along invisible "vines" using Perlin noise for natural distribution.

use noise::{NoiseFn, Perlin};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use super::Simulation;

/// Invisible line along which leaves grow
#[derive(Debug, Clone, Copy)]
pub struct Vine {
    pub start: [f32; 2],
    pub end: [f32; 2],
}

impl Vine {
    pub fn new(start: [f32; 2], end: [f32; 2]) -> Self {
        Self { start, end }
    }

    /// Get point along vine at position t (0.0 = start, 1.0 = end)
    pub fn point_at(&self, t: f32) -> [f32; 2] {
        [
            self.start[0] + (self.end[0] - self.start[0]) * t,
            self.start[1] + (self.end[1] - self.start[1]) * t,
        ]
    }

    /// Get perpendicular direction (normalized)
    pub fn perpendicular(&self) -> [f32; 2] {
        let dx = self.end[0] - self.start[0];
        let dy = self.end[1] - self.start[1];
        let len = (dx * dx + dy * dy).sqrt();
        if len > 0.0 {
            [-dy / len, dx / len] // 90° rotation
        } else {
            [0.0, 1.0]
        }
    }

    /// Get direction along vine (for leaf rotation)
    pub fn direction_angle(&self) -> f32 {
        let dx = self.end[0] - self.start[0];
        let dy = self.end[1] - self.start[1];
        dy.atan2(dx)
    }

    /// Get length of the vine
    fn length(&self) -> f32 {
        let dx = self.end[0] - self.start[0];
        let dy = self.end[1] - self.start[1];
        (dx * dx + dy * dy).sqrt()
    }
}

/// Individual leaf instance (pure sim data)
#[derive(Debug, Clone, Copy)]
pub struct Leaf {
    pub position: [f32; 2],
    pub size: f32,
    pub aspect: f32,
    pub rotation: f32,
    pub growth: f32,
    pub color_variant: u8,
}

/// Configuration for leaf simulation
#[derive(Debug, Clone)]
pub struct LeafConfig {
    pub spawn_rate: f32,
    pub growth_rate: f32,
    pub base_size: f32,
    pub size_variation: f32,
    pub max_offset: f32,
    pub noise_seed: u32,
}

impl Default for LeafConfig {
    fn default() -> Self {
        Self {
            spawn_rate: 2.0,
            growth_rate: 1.0,
            base_size: 8.0,
            size_variation: 0.3,
            max_offset: 20.0,
            noise_seed: 42,
        }
    }
}

/// Leaf placement simulation
pub struct LeafSimulation {
    vines: Vec<Vine>,
    leaves: Vec<Leaf>,
    config: LeafConfig,
    spawn_accumulator: f32,
    noise: Perlin,
    rng: StdRng,
    max_leaves: usize,
    active: bool,
    spawn_counter: usize, // Tracks total spawns for noise time evolution
}

impl LeafSimulation {
    pub fn new() -> Self {
        Self::with_config(LeafConfig::default())
    }

    pub fn with_config(config: LeafConfig) -> Self {
        let rng = StdRng::seed_from_u64(config.noise_seed as u64);
        let noise = Perlin::new(config.noise_seed);

        Self {
            vines: Vec::new(),
            leaves: Vec::new(),
            config,
            spawn_accumulator: 0.0,
            noise,
            rng,
            max_leaves: 500,
            active: true,
            spawn_counter: 0,
        }
    }

    // Vine management
    pub fn add_vine(&mut self, vine: Vine) {
        self.vines.push(vine);
    }

    pub fn add_vine_line(&mut self, start: [f32; 2], end: [f32; 2]) {
        self.add_vine(Vine::new(start, end));
    }

    pub fn clear_vines(&mut self) {
        self.vines.clear();
    }

    pub fn vines(&self) -> &[Vine] {
        &self.vines
    }

    // Leaf access
    pub fn leaves(&self) -> &[Leaf] {
        &self.leaves
    }

    // Control
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn set_max_leaves(&mut self, max: usize) {
        self.max_leaves = max;
    }

    // Leaf generation helpers
    fn select_random_vine(&mut self) -> Option<usize> {
        if self.vines.is_empty() {
            return None;
        }

        // Weight by length - longer vines more likely to be selected
        let total_length: f32 = self.vines.iter().map(|v| v.length()).sum();
        if total_length <= 0.0 {
            // All zero-length vines, just pick random
            return Some(self.rng.random_range(0..self.vines.len()));
        }

        let target = self.rng.random::<f32>() * total_length;
        let mut accumulated = 0.0;
        for (idx, vine) in self.vines.iter().enumerate() {
            accumulated += vine.length();
            if accumulated >= target {
                return Some(idx);
            }
        }

        // Fallback to last vine (shouldn't reach here)
        Some(self.vines.len() - 1)
    }

    fn sample_vine_position(&self, vine_idx: usize) -> f32 {
        let time_factor = self.spawn_counter as f64 * 0.1;
        let noise_val = self.noise.get([vine_idx as f64, time_factor]);
        // Map [-1, 1] → [0.1, 0.9]
        (noise_val as f32 + 1.0) * 0.4 + 0.1
    }

    fn sample_perpendicular_offset(&self, vine_idx: usize, vine_pos: f32) -> f32 {
        let noise_val = self
            .noise
            .get([vine_idx as f64 * 10.0, vine_pos as f64 * 20.0]);
        noise_val as f32 * self.config.max_offset
    }

    fn sample_rotation(&mut self, base_angle: f32) -> f32 {
        base_angle + self.rng.random_range(-0.3..0.3)
    }

    fn generate_leaf(&mut self) -> Option<Leaf> {
        let vine_idx = self.select_random_vine()?;
        let vine = &self.vines[vine_idx];

        // Sample position along vine
        let vine_pos = self.sample_vine_position(vine_idx);
        let base_position = vine.point_at(vine_pos);

        // Sample perpendicular offset
        let offset_amount = self.sample_perpendicular_offset(vine_idx, vine_pos);
        let perp = vine.perpendicular();
        let position = [
            base_position[0] + perp[0] * offset_amount,
            base_position[1] + perp[1] * offset_amount,
        ];

        // Sample size with variation
        let size = self.config.base_size
            * self
                .rng
                .random_range(1.0 - self.config.size_variation..=1.0 + self.config.size_variation);

        // Sample aspect ratio (elliptical)
        let aspect = self.rng.random_range(0.5..0.7);

        // Sample rotation based on vine direction
        let base_angle = vine.direction_angle();
        let rotation = self.sample_rotation(base_angle);

        // Random color variant
        let color_variant = self.rng.random_range(0..4);

        Some(Leaf {
            position,
            size,
            aspect,
            rotation,
            growth: 0.0, // Starts at 0, will grow over time
            color_variant,
        })
    }
}

impl Default for LeafSimulation {
    fn default() -> Self {
        Self::new()
    }
}

impl Simulation for LeafSimulation {
    fn tick(&mut self, delta_time: f32) {
        if !self.active {
            return;
        }

        // Early exit if no vines
        if self.vines.is_empty() {
            return;
        }

        // Phase 1: Grow existing leaves
        for leaf in &mut self.leaves {
            if leaf.growth < 1.0 {
                leaf.growth = (leaf.growth + self.config.growth_rate * delta_time).min(1.0);
            }
        }

        // Phase 2: Spawn new leaves (fixed timestep)
        self.spawn_accumulator += delta_time;
        let spawn_interval = 1.0 / self.config.spawn_rate;

        while self.spawn_accumulator >= spawn_interval {
            self.spawn_accumulator -= spawn_interval;

            if self.leaves.len() < self.max_leaves
                && let Some(new_leaf) = self.generate_leaf()
            {
                self.leaves.push(new_leaf);
                self.spawn_counter += 1;
            }
        }
    }

    fn reset(&mut self) {
        self.leaves.clear();
        self.spawn_accumulator = 0.0;
        self.spawn_counter = 0;
        // Note: vines are preserved
    }

    fn name(&self) -> &str {
        "leaf"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a standard test vine
    fn test_vine() -> Vine {
        Vine::new([100.0, 100.0], [200.0, 100.0])
    }

    #[test]
    fn test_initialization() {
        let sim = LeafSimulation::new();
        assert_eq!(sim.leaves().len(), 0);
        assert_eq!(sim.vines().len(), 0);
        assert_eq!(sim.name(), "leaf");
        assert!(sim.is_active());
    }

    #[test]
    fn test_add_vine() {
        let mut sim = LeafSimulation::new();
        sim.add_vine(test_vine());
        assert_eq!(sim.vines().len(), 1);
    }

    #[test]
    fn test_add_multiple_vines() {
        let mut sim = LeafSimulation::new();
        sim.add_vine(test_vine());
        sim.add_vine_line([0.0, 0.0], [100.0, 100.0]);
        assert_eq!(sim.vines().len(), 2);
    }

    #[test]
    fn test_clear_vines() {
        let mut sim = LeafSimulation::new();
        sim.add_vine(test_vine());
        sim.add_vine(test_vine());
        assert_eq!(sim.vines().len(), 2);
        sim.clear_vines();
        assert_eq!(sim.vines().len(), 0);
    }

    #[test]
    fn test_vine_point_at() {
        let vine = Vine::new([0.0, 0.0], [100.0, 0.0]);
        assert_eq!(vine.point_at(0.0), [0.0, 0.0]);
        assert_eq!(vine.point_at(0.5), [50.0, 0.0]);
        assert_eq!(vine.point_at(1.0), [100.0, 0.0]);
    }

    #[test]
    fn test_vine_perpendicular() {
        let vine = Vine::new([0.0, 0.0], [100.0, 0.0]);
        let perp = vine.perpendicular();
        // Horizontal line → vertical perpendicular
        assert!((perp[0]).abs() < 0.001);
        assert!((perp[1] - 1.0).abs() < 0.001 || (perp[1] + 1.0).abs() < 0.001);
    }

    #[test]
    fn test_vine_direction_angle() {
        let vine = Vine::new([0.0, 0.0], [100.0, 0.0]);
        let angle = vine.direction_angle();
        assert!((angle - 0.0).abs() < 0.001); // Horizontal = 0 radians

        let vine2 = Vine::new([0.0, 0.0], [0.0, 100.0]);
        let angle2 = vine2.direction_angle();
        assert!((angle2 - std::f32::consts::FRAC_PI_2).abs() < 0.001); // Vertical = π/2
    }

    #[test]
    fn test_zero_length_vine() {
        let vine = Vine::new([50.0, 50.0], [50.0, 50.0]);
        let perp = vine.perpendicular();
        assert_eq!(perp, [0.0, 1.0]); // Default perpendicular
        assert_eq!(vine.length(), 0.0);
    }

    #[test]
    fn test_spawning_over_time() {
        let mut sim = LeafSimulation::new();
        sim.add_vine(test_vine());

        // Simulate 5 seconds at 60 FPS (2 leaves/sec = ~10 leaves)
        for _ in 0..300 {
            sim.tick(1.0 / 60.0);
        }

        let count = sim.leaves().len();
        assert!(
            (8..=12).contains(&count),
            "Expected ~10 leaves, got {}",
            count
        );
    }

    #[test]
    fn test_growth_progression() {
        let mut sim = LeafSimulation::with_config(LeafConfig {
            spawn_rate: 100.0, // Spawn immediately
            growth_rate: 1.0,
            ..Default::default()
        });
        sim.add_vine(test_vine());

        // Spawn leaves
        sim.tick(0.1);
        assert!(!sim.leaves().is_empty());

        let initial_growth = sim.leaves()[0].growth;

        // Grow for 0.5 seconds
        for _ in 0..30 {
            sim.tick(1.0 / 60.0);
        }

        let after_growth = sim.leaves()[0].growth;
        assert!(after_growth > initial_growth);
        assert!((0.4..=0.6).contains(&after_growth));
    }

    #[test]
    fn test_growth_clamps_at_one() {
        let mut sim = LeafSimulation::with_config(LeafConfig {
            spawn_rate: 1.0,
            growth_rate: 1.0,
            ..Default::default()
        });
        sim.add_vine(test_vine());

        // Spawn and grow way past 1.0
        sim.tick(0.1); // Spawn
        for _ in 0..120 {
            sim.tick(1.0 / 60.0); // 2 seconds of growth
        }

        assert_eq!(sim.leaves()[0].growth, 1.0);
    }

    #[test]
    fn test_max_leaves_cap() {
        let mut sim = LeafSimulation::new();
        sim.set_max_leaves(10);
        sim.add_vine(test_vine());

        sim.config.spawn_rate = 100.0; // Very high

        // Try to spawn many leaves
        for _ in 0..100 {
            sim.tick(0.1);
        }

        assert_eq!(sim.leaves().len(), 10);
    }

    #[test]
    fn test_no_spawning_without_vines() {
        let mut sim = LeafSimulation::new();
        // No vines added

        for _ in 0..100 {
            sim.tick(0.1);
        }

        assert_eq!(sim.leaves().len(), 0);
    }

    #[test]
    fn test_inactive_no_growth_or_spawn() {
        let mut sim = LeafSimulation::new();
        sim.add_vine(test_vine());
        sim.set_active(false);

        for _ in 0..100 {
            sim.tick(0.1);
        }

        assert_eq!(sim.leaves().len(), 0);
    }

    #[test]
    fn test_set_active() {
        let mut sim = LeafSimulation::new();
        assert!(sim.is_active());
        sim.set_active(false);
        assert!(!sim.is_active());
        sim.set_active(true);
        assert!(sim.is_active());
    }

    #[test]
    fn test_reset_clears_leaves_keeps_vines() {
        let mut sim = LeafSimulation::new();
        sim.add_vine(test_vine());

        // Spawn some leaves
        sim.config.spawn_rate = 10.0;
        sim.tick(1.0);
        assert!(!sim.leaves().is_empty());

        // Reset
        sim.reset();

        assert_eq!(sim.leaves().len(), 0);
        assert_eq!(sim.vines().len(), 1); // Vines preserved
    }

    #[test]
    fn test_determinism_same_seed() {
        let config = LeafConfig {
            noise_seed: 12345,
            spawn_rate: 5.0,
            ..Default::default()
        };

        let mut sim1 = LeafSimulation::with_config(config.clone());
        let mut sim2 = LeafSimulation::with_config(config);

        sim1.add_vine(test_vine());
        sim2.add_vine(test_vine());

        // Run identical simulations
        for _ in 0..60 {
            sim1.tick(1.0 / 60.0);
            sim2.tick(1.0 / 60.0);
        }

        // Should have same number of leaves
        assert_eq!(sim1.leaves().len(), sim2.leaves().len());

        // Should have same positions (within floating point tolerance)
        for (l1, l2) in sim1.leaves().iter().zip(sim2.leaves().iter()) {
            assert!((l1.position[0] - l2.position[0]).abs() < 0.001);
            assert!((l1.position[1] - l2.position[1]).abs() < 0.001);
        }
    }

    #[test]
    fn test_different_seeds_different_results() {
        let config1 = LeafConfig {
            noise_seed: 111,
            spawn_rate: 5.0,
            ..Default::default()
        };
        let config2 = LeafConfig {
            noise_seed: 222,
            spawn_rate: 5.0,
            ..Default::default()
        };

        let mut sim1 = LeafSimulation::with_config(config1);
        let mut sim2 = LeafSimulation::with_config(config2);

        sim1.add_vine(test_vine());
        sim2.add_vine(test_vine());

        // Run simulations
        for _ in 0..60 {
            sim1.tick(1.0 / 60.0);
            sim2.tick(1.0 / 60.0);
        }

        // Should have different positions
        let mut found_difference = false;
        for (l1, l2) in sim1.leaves().iter().zip(sim2.leaves().iter()) {
            if (l1.position[0] - l2.position[0]).abs() > 0.001 {
                found_difference = true;
                break;
            }
        }
        assert!(
            found_difference,
            "Different seeds should produce different results"
        );
    }

    #[test]
    fn test_leaf_properties_in_range() {
        let mut sim = LeafSimulation::new();
        sim.add_vine(test_vine());
        sim.config.spawn_rate = 10.0;

        sim.tick(1.0);

        for leaf in sim.leaves() {
            assert!(leaf.size > 0.0);
            assert!(leaf.aspect >= 0.5 && leaf.aspect <= 0.7);
            assert_eq!(leaf.growth, 0.0); // Just spawned
            assert!(leaf.color_variant < 4);
        }
    }

    #[test]
    fn test_zero_delta_time_no_change() {
        let mut sim = LeafSimulation::new();
        sim.add_vine(test_vine());

        sim.tick(0.0);
        assert_eq!(sim.leaves().len(), 0);
    }

    #[test]
    fn test_weighted_vine_selection() {
        let mut sim = LeafSimulation::new();
        // Add one very long vine and one very short vine
        sim.add_vine_line([0.0, 0.0], [1000.0, 0.0]); // Long
        sim.add_vine_line([0.0, 0.0], [1.0, 0.0]); // Short

        sim.config.spawn_rate = 100.0;
        sim.tick(1.0); // Spawn many leaves

        // Most leaves should be on the long vine (around position 0-1000)
        let on_long_vine = sim.leaves().iter().filter(|l| l.position[0] > 10.0).count();

        // Expect at least 90% on the long vine (it's 1000x longer)
        assert!(
            on_long_vine as f32 / sim.leaves().len() as f32 > 0.9,
            "Expected most leaves on long vine, got {} / {}",
            on_long_vine,
            sim.leaves().len()
        );
    }
}
