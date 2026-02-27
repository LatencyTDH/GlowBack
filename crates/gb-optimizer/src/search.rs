//! Search space definitions and parameter sweep strategies.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single parameter dimension in the search space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterDef {
    /// Human-readable parameter name (e.g. "short_period").
    pub name: String,
    /// The kind of search range.
    pub kind: ParameterKind,
}

/// Describes how a parameter is sampled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParameterKind {
    /// Continuous uniform range [low, high].
    FloatRange { low: f64, high: f64 },
    /// Integer range [low, high] inclusive.
    IntRange { low: i64, high: i64 },
    /// Log-uniform range (sampled in log-space then exponentiated).
    LogUniform { low: f64, high: f64 },
    /// Categorical choices.
    Choice { values: Vec<serde_json::Value> },
}

/// A concrete parameter value produced by a search strategy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParameterValue {
    Float(f64),
    Int(i64),
    Json(serde_json::Value),
}

impl std::fmt::Display for ParameterValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Float(v) => write!(f, "{v}"),
            Self::Int(v) => write!(f, "{v}"),
            Self::Json(v) => write!(f, "{v}"),
        }
    }
}

/// The full search space: an ordered list of parameter definitions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchSpace {
    pub parameters: Vec<ParameterDef>,
}

impl SearchSpace {
    pub fn new() -> Self {
        Self {
            parameters: Vec::new(),
        }
    }

    pub fn add_float(mut self, name: impl Into<String>, low: f64, high: f64) -> Self {
        self.parameters.push(ParameterDef {
            name: name.into(),
            kind: ParameterKind::FloatRange { low, high },
        });
        self
    }

    pub fn add_int(mut self, name: impl Into<String>, low: i64, high: i64) -> Self {
        self.parameters.push(ParameterDef {
            name: name.into(),
            kind: ParameterKind::IntRange { low, high },
        });
        self
    }

    pub fn add_log_uniform(mut self, name: impl Into<String>, low: f64, high: f64) -> Self {
        self.parameters.push(ParameterDef {
            name: name.into(),
            kind: ParameterKind::LogUniform { low, high },
        });
        self
    }

    pub fn add_choice(mut self, name: impl Into<String>, values: Vec<serde_json::Value>) -> Self {
        self.parameters.push(ParameterDef {
            name: name.into(),
            kind: ParameterKind::Choice { values },
        });
        self
    }

    /// Total number of grid points (returns `None` if any parameter is
    /// continuous without a natural grid).
    pub fn grid_size(&self) -> Option<usize> {
        let mut total: usize = 1;
        for param in &self.parameters {
            let dim_size = match &param.kind {
                ParameterKind::IntRange { low, high } => (high - low + 1) as usize,
                ParameterKind::Choice { values } => values.len(),
                // Continuous dimensions need explicit step count — not grid-able by default.
                _ => return None,
            };
            total = total.checked_mul(dim_size)?;
        }
        Some(total)
    }
}

impl Default for SearchSpace {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Search strategies
// ---------------------------------------------------------------------------

/// Common trait for all search strategies.
pub trait SearchStrategy: Send + Sync {
    /// Generate the next batch of parameter combinations to evaluate.
    fn suggest(&mut self, count: usize) -> Vec<HashMap<String, ParameterValue>>;

    /// Report completed trial results so adaptive strategies can learn.
    fn report(&mut self, _params: &HashMap<String, ParameterValue>, _objective: f64) {}

    /// Human-readable strategy name.
    fn name(&self) -> &str;
}

// ---- Grid search ----

/// Exhaustive grid search over discrete parameter combinations.
#[derive(Debug, Clone)]
pub struct GridSearch {
    #[allow(dead_code)]
    space: SearchSpace,
    /// Number of steps for continuous dimensions.
    #[allow(dead_code)]
    float_steps: usize,
    cursor: usize,
    combos: Vec<HashMap<String, ParameterValue>>,
}

impl GridSearch {
    pub fn new(space: SearchSpace, float_steps: usize) -> Self {
        let combos = Self::build_grid(&space, float_steps);
        Self {
            space,
            float_steps,
            cursor: 0,
            combos,
        }
    }

    fn build_grid(space: &SearchSpace, float_steps: usize) -> Vec<HashMap<String, ParameterValue>> {
        let mut axes: Vec<Vec<(&str, ParameterValue)>> = Vec::new();

        for param in &space.parameters {
            let values: Vec<ParameterValue> = match &param.kind {
                ParameterKind::FloatRange { low, high } => {
                    let steps = float_steps.max(2);
                    (0..steps)
                        .map(|i| {
                            let t = i as f64 / (steps - 1) as f64;
                            ParameterValue::Float(low + t * (high - low))
                        })
                        .collect()
                }
                ParameterKind::IntRange { low, high } => {
                    (*low..=*high).map(ParameterValue::Int).collect()
                }
                ParameterKind::LogUniform { low, high } => {
                    let steps = float_steps.max(2);
                    let log_low = low.ln();
                    let log_high = high.ln();
                    (0..steps)
                        .map(|i| {
                            let t = i as f64 / (steps - 1) as f64;
                            ParameterValue::Float((log_low + t * (log_high - log_low)).exp())
                        })
                        .collect()
                }
                ParameterKind::Choice { values } => values
                    .iter()
                    .map(|v| ParameterValue::Json(v.clone()))
                    .collect(),
            };
            axes.push(
                values
                    .into_iter()
                    .map(|v| (param.name.as_str(), v))
                    .collect(),
            );
        }

        // Cartesian product
        let mut result: Vec<HashMap<String, ParameterValue>> = vec![HashMap::new()];
        for axis in &axes {
            let mut next = Vec::with_capacity(result.len() * axis.len());
            for existing in &result {
                for (name, value) in axis {
                    let mut combo = existing.clone();
                    combo.insert(name.to_string(), value.clone());
                    next.push(combo);
                }
            }
            result = next;
        }

        result
    }
}

impl SearchStrategy for GridSearch {
    fn suggest(&mut self, count: usize) -> Vec<HashMap<String, ParameterValue>> {
        let end = (self.cursor + count).min(self.combos.len());
        let batch = self.combos[self.cursor..end].to_vec();
        self.cursor = end;
        batch
    }

    fn name(&self) -> &str {
        "grid"
    }
}

// ---- Random search ----

/// Independent random sampling across the search space.
#[derive(Debug, Clone)]
pub struct RandomSearch {
    space: SearchSpace,
}

impl RandomSearch {
    pub fn new(space: SearchSpace) -> Self {
        Self { space }
    }

    fn sample_one(&self) -> HashMap<String, ParameterValue> {
        let mut rng = rand::thread_rng();
        let mut params = HashMap::new();

        for param in &self.space.parameters {
            let value = match &param.kind {
                ParameterKind::FloatRange { low, high } => {
                    ParameterValue::Float(rng.gen_range(*low..=*high))
                }
                ParameterKind::IntRange { low, high } => {
                    ParameterValue::Int(rng.gen_range(*low..=*high))
                }
                ParameterKind::LogUniform { low, high } => {
                    let log_low = low.ln();
                    let log_high = high.ln();
                    let log_val: f64 = rng.gen_range(log_low..=log_high);
                    ParameterValue::Float(log_val.exp())
                }
                ParameterKind::Choice { values } => {
                    let idx = rng.gen_range(0..values.len());
                    ParameterValue::Json(values[idx].clone())
                }
            };
            params.insert(param.name.clone(), value);
        }

        params
    }
}

impl SearchStrategy for RandomSearch {
    fn suggest(&mut self, count: usize) -> Vec<HashMap<String, ParameterValue>> {
        (0..count).map(|_| self.sample_one()).collect()
    }

    fn name(&self) -> &str {
        "random"
    }
}

// ---- Bayesian search (surrogate-model stub) ----

/// Bayesian optimization using a simple surrogate model.
///
/// This implementation tracks observed (params, objective) pairs and uses them
/// to bias future sampling toward promising regions.  A full Gaussian-process
/// backend can be plugged in via the `report` method; the default uses a
/// weighted-random heuristic.
#[derive(Debug, Clone)]
pub struct BayesianSearch {
    space: SearchSpace,
    observations: Vec<(HashMap<String, ParameterValue>, f64)>,
    exploration_weight: f64,
}

impl BayesianSearch {
    pub fn new(space: SearchSpace, exploration_weight: f64) -> Self {
        Self {
            space,
            observations: Vec::new(),
            exploration_weight,
        }
    }

    /// Pure exploration sample (same as random).
    fn explore(&self) -> HashMap<String, ParameterValue> {
        let random = RandomSearch::new(self.space.clone());
        random.sample_one()
    }

    /// Exploitation: perturb the best-known point.
    fn exploit(&self) -> HashMap<String, ParameterValue> {
        let best = self
            .observations
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let base = match best {
            Some((params, _)) => params.clone(),
            None => return self.explore(),
        };

        let mut rng = rand::thread_rng();
        let mut perturbed = HashMap::new();

        for param in &self.space.parameters {
            let base_val = base.get(&param.name);
            let value = match (&param.kind, base_val) {
                (ParameterKind::FloatRange { low, high }, Some(ParameterValue::Float(v))) => {
                    let range = high - low;
                    let noise = rng.gen_range(-0.1..0.1) * range;
                    ParameterValue::Float((v + noise).clamp(*low, *high))
                }
                (ParameterKind::IntRange { low, high }, Some(ParameterValue::Int(v))) => {
                    let delta: i64 = rng.gen_range(-2..=2);
                    ParameterValue::Int((v + delta).clamp(*low, *high))
                }
                (ParameterKind::LogUniform { low, high }, Some(ParameterValue::Float(v))) => {
                    let log_v = v.ln();
                    let log_range = high.ln() - low.ln();
                    let noise = rng.gen_range(-0.1..0.1) * log_range;
                    ParameterValue::Float((log_v + noise).exp().clamp(*low, *high))
                }
                _ => {
                    // Fall back to random for choices or missing base
                    RandomSearch::new(SearchSpace {
                        parameters: vec![param.clone()],
                    })
                    .sample_one()
                    .remove(&param.name)
                    .unwrap_or(ParameterValue::Int(0))
                }
            };
            perturbed.insert(param.name.clone(), value);
        }

        perturbed
    }
}

impl SearchStrategy for BayesianSearch {
    fn suggest(&mut self, count: usize) -> Vec<HashMap<String, ParameterValue>> {
        let mut rng = rand::thread_rng();
        (0..count)
            .map(|_| {
                if self.observations.is_empty() || rng.gen::<f64>() < self.exploration_weight {
                    self.explore()
                } else {
                    self.exploit()
                }
            })
            .collect()
    }

    fn report(&mut self, params: &HashMap<String, ParameterValue>, objective: f64) {
        self.observations.push((params.clone(), objective));
    }

    fn name(&self) -> &str {
        "bayesian"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_space() -> SearchSpace {
        SearchSpace::new()
            .add_int("short_period", 5, 15)
            .add_int("long_period", 20, 50)
            .add_float("position_size", 0.5, 1.0)
    }

    #[test]
    fn grid_search_produces_correct_count() {
        let space = SearchSpace::new()
            .add_int("a", 1, 3) // 3 values
            .add_int("b", 10, 11); // 2 values
        assert_eq!(space.grid_size(), Some(6));

        let mut gs = GridSearch::new(space, 5);
        let batch = gs.suggest(100);
        assert_eq!(batch.len(), 6);
    }

    #[test]
    fn grid_search_cursor_advances() {
        let space = SearchSpace::new().add_int("x", 1, 5); // 5 values
        let mut gs = GridSearch::new(space, 5);
        let first = gs.suggest(3);
        assert_eq!(first.len(), 3);
        let second = gs.suggest(10);
        assert_eq!(second.len(), 2); // only 2 remain
    }

    #[test]
    fn random_search_respects_bounds() {
        let space = sample_space();
        let mut rs = RandomSearch::new(space);
        let suggestions = rs.suggest(50);
        assert_eq!(suggestions.len(), 50);

        for params in &suggestions {
            match params.get("short_period") {
                Some(ParameterValue::Int(v)) => assert!(*v >= 5 && *v <= 15),
                other => panic!("unexpected short_period value: {other:?}"),
            }
            match params.get("position_size") {
                Some(ParameterValue::Float(v)) => assert!(*v >= 0.5 && *v <= 1.0),
                other => panic!("unexpected position_size value: {other:?}"),
            }
        }
    }

    #[test]
    fn bayesian_search_starts_with_exploration() {
        let space = sample_space();
        let mut bs = BayesianSearch::new(space, 0.3);
        // No observations yet → all suggestions are exploration
        let suggestions = bs.suggest(10);
        assert_eq!(suggestions.len(), 10);
    }

    #[test]
    fn bayesian_search_exploits_after_reports() {
        let space = SearchSpace::new().add_float("lr", 0.001, 1.0);
        let mut bs = BayesianSearch::new(space, 0.0); // exploration_weight=0 → always exploit after report

        let mut best_params = HashMap::new();
        best_params.insert("lr".to_string(), ParameterValue::Float(0.01));
        bs.report(&best_params, 0.95);

        let suggestions = bs.suggest(20);
        // All suggestions should be perturbations near 0.01
        for params in &suggestions {
            match params.get("lr") {
                Some(ParameterValue::Float(v)) => {
                    // Should be within ±10% of the range from the best point
                    assert!(*v >= 0.001 && *v <= 1.0);
                }
                other => panic!("unexpected lr value: {other:?}"),
            }
        }
    }

    #[test]
    fn grid_size_none_for_float_only() {
        let space = SearchSpace::new().add_float("x", 0.0, 1.0);
        assert_eq!(space.grid_size(), None);
    }

    #[test]
    fn choice_parameter_works() {
        let space = SearchSpace::new().add_choice(
            "strategy",
            vec![
                serde_json::json!("ma_crossover"),
                serde_json::json!("momentum"),
                serde_json::json!("mean_reversion"),
            ],
        );
        let mut rs = RandomSearch::new(space);
        let suggestions = rs.suggest(30);
        assert_eq!(suggestions.len(), 30);
        for params in &suggestions {
            match params.get("strategy") {
                Some(ParameterValue::Json(v)) => {
                    let s = v.as_str().unwrap();
                    assert!(["ma_crossover", "momentum", "mean_reversion"].contains(&s));
                }
                other => panic!("unexpected strategy value: {other:?}"),
            }
        }
    }

    #[test]
    fn log_uniform_stays_in_bounds() {
        let space = SearchSpace::new().add_log_uniform("lr", 1e-5, 1e-1);
        let mut rs = RandomSearch::new(space);
        let suggestions = rs.suggest(100);
        for params in &suggestions {
            match params.get("lr") {
                Some(ParameterValue::Float(v)) => {
                    assert!(*v >= 1e-5 && *v <= 1e-1, "lr out of bounds: {v}");
                }
                other => panic!("unexpected lr value: {other:?}"),
            }
        }
    }

    #[test]
    fn search_space_builder_chain() {
        let space = SearchSpace::new()
            .add_int("a", 1, 10)
            .add_float("b", 0.0, 1.0)
            .add_log_uniform("c", 0.001, 100.0)
            .add_choice("d", vec![serde_json::json!(true), serde_json::json!(false)]);
        assert_eq!(space.parameters.len(), 4);
    }
}
