//! Trial tracking and optimization run management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::search::{ParameterValue, SearchSpace};

/// Unique optimization run identifier.
pub type OptimizationId = Uuid;

/// Whether we are maximizing or minimizing the objective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectiveDirection {
    Maximize,
    Minimize,
}

impl Default for ObjectiveDirection {
    fn default() -> Self {
        Self::Maximize
    }
}

/// Top-level configuration for an optimization run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptimizationConfig {
    pub id: OptimizationId,
    pub name: String,
    pub description: String,

    /// The parameter search space.
    pub search_space: SearchSpace,

    /// Which search strategy to use: "grid", "random", or "bayesian".
    pub strategy: String,

    /// Maximum number of trials to run.
    pub max_trials: usize,

    /// How many trials to run in parallel.
    pub concurrency: usize,

    /// Metric name to optimize (e.g. "sharpe_ratio", "total_return").
    pub objective_metric: String,

    /// Direction of optimization.
    pub direction: ObjectiveDirection,

    /// Base backtest configuration that trials will override with sampled
    /// parameters.  Stored as opaque JSON so the optimizer crate doesn't
    /// depend on API-layer models.
    pub base_backtest: serde_json::Value,

    /// Exploration weight for Bayesian search (ignored for grid/random).
    pub exploration_weight: f64,

    /// Number of steps per continuous dimension for grid search.
    pub grid_steps: usize,

    pub created_at: DateTime<Utc>,
}

impl OptimizationConfig {
    pub fn new(name: String, search_space: SearchSpace, strategy: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            description: String::new(),
            search_space,
            strategy: strategy.to_string(),
            max_trials: 100,
            concurrency: 4,
            objective_metric: "sharpe_ratio".to_string(),
            direction: ObjectiveDirection::Maximize,
            base_backtest: serde_json::Value::Null,
            exploration_weight: 0.3,
            grid_steps: 5,
            created_at: Utc::now(),
        }
    }

    pub fn with_max_trials(mut self, n: usize) -> Self {
        self.max_trials = n;
        self
    }

    pub fn with_concurrency(mut self, n: usize) -> Self {
        self.concurrency = n;
        self
    }

    pub fn with_objective(mut self, metric: &str, direction: ObjectiveDirection) -> Self {
        self.objective_metric = metric.to_string();
        self.direction = direction;
        self
    }

    pub fn with_base_backtest(mut self, config: serde_json::Value) -> Self {
        self.base_backtest = config;
        self
    }
}

/// Lifecycle state for an optimization run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Aggregate status of an optimization run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptimizationStatus {
    pub id: OptimizationId,
    pub config: OptimizationConfig,
    pub state: OptimizationState,
    pub trials_completed: usize,
    pub trials_failed: usize,
    pub trials_running: usize,
    pub best_trial: Option<TrialResult>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

impl OptimizationStatus {
    pub fn new(config: OptimizationConfig) -> Self {
        Self {
            id: config.id,
            config,
            state: OptimizationState::Pending,
            trials_completed: 0,
            trials_failed: 0,
            trials_running: 0,
            best_trial: None,
            started_at: None,
            finished_at: None,
            error: None,
        }
    }

    pub fn mark_running(&mut self) {
        self.state = OptimizationState::Running;
        self.started_at = Some(Utc::now());
    }

    pub fn mark_completed(&mut self) {
        self.state = OptimizationState::Completed;
        self.finished_at = Some(Utc::now());
    }

    pub fn mark_failed(&mut self, error: String) {
        self.state = OptimizationState::Failed;
        self.finished_at = Some(Utc::now());
        self.error = Some(error);
    }

    /// Update the best trial if `result` improves on the current best.
    pub fn update_best(&mut self, result: &TrialResult) {
        let dominated = match &self.best_trial {
            None => true,
            Some(current_best) => match self.config.direction {
                ObjectiveDirection::Maximize => result.objective > current_best.objective,
                ObjectiveDirection::Minimize => result.objective < current_best.objective,
            },
        };
        if dominated {
            self.best_trial = Some(result.clone());
        }
    }
}

// ---------------------------------------------------------------------------
// Individual trial
// ---------------------------------------------------------------------------

/// A single trial (one parameter combination evaluated via backtest).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trial {
    pub id: Uuid,
    pub optimization_id: OptimizationId,
    pub trial_number: usize,
    pub parameters: HashMap<String, ParameterValue>,
    pub status: TrialStatus,
    pub result: Option<TrialResult>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub worker_id: Option<String>,
    pub error: Option<String>,
}

impl Trial {
    pub fn new(
        optimization_id: OptimizationId,
        trial_number: usize,
        parameters: HashMap<String, ParameterValue>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            optimization_id,
            trial_number,
            parameters,
            status: TrialStatus::Pending,
            result: None,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
            worker_id: None,
            error: None,
        }
    }

    pub fn mark_running(&mut self, worker_id: Option<String>) {
        self.status = TrialStatus::Running;
        self.started_at = Some(Utc::now());
        self.worker_id = worker_id;
    }

    pub fn mark_completed(&mut self, result: TrialResult) {
        self.status = TrialStatus::Completed;
        self.finished_at = Some(Utc::now());
        self.result = Some(result);
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = TrialStatus::Failed;
        self.finished_at = Some(Utc::now());
        self.error = Some(error);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrialStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Result of a single trial.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrialResult {
    pub trial_id: Uuid,
    pub objective: f64,
    pub metrics: HashMap<String, f64>,
    pub parameters: HashMap<String, ParameterValue>,
    pub duration_seconds: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::SearchSpace;

    fn sample_config() -> OptimizationConfig {
        let space =
            SearchSpace::new()
                .add_int("short_period", 5, 15)
                .add_float("position_size", 0.5, 1.0);

        OptimizationConfig::new("test_opt".into(), space, "random")
            .with_max_trials(50)
            .with_concurrency(4)
            .with_objective("sharpe_ratio", ObjectiveDirection::Maximize)
    }

    #[test]
    fn optimization_status_lifecycle() {
        let config = sample_config();
        let mut status = OptimizationStatus::new(config);

        assert_eq!(status.state, OptimizationState::Pending);
        assert!(status.started_at.is_none());

        status.mark_running();
        assert_eq!(status.state, OptimizationState::Running);
        assert!(status.started_at.is_some());

        status.mark_completed();
        assert_eq!(status.state, OptimizationState::Completed);
        assert!(status.finished_at.is_some());
    }

    #[test]
    fn best_trial_tracking_maximize() {
        let config = sample_config();
        let mut status = OptimizationStatus::new(config);

        let result_a = TrialResult {
            trial_id: Uuid::new_v4(),
            objective: 1.5,
            metrics: HashMap::new(),
            parameters: HashMap::new(),
            duration_seconds: Some(10),
        };
        status.update_best(&result_a);
        assert_eq!(status.best_trial.as_ref().unwrap().objective, 1.5);

        let result_b = TrialResult {
            trial_id: Uuid::new_v4(),
            objective: 2.0,
            metrics: HashMap::new(),
            parameters: HashMap::new(),
            duration_seconds: Some(8),
        };
        status.update_best(&result_b);
        assert_eq!(status.best_trial.as_ref().unwrap().objective, 2.0);

        // Worse result should not replace
        let result_c = TrialResult {
            trial_id: Uuid::new_v4(),
            objective: 1.0,
            metrics: HashMap::new(),
            parameters: HashMap::new(),
            duration_seconds: Some(12),
        };
        status.update_best(&result_c);
        assert_eq!(status.best_trial.as_ref().unwrap().objective, 2.0);
    }

    #[test]
    fn best_trial_tracking_minimize() {
        let space = SearchSpace::new().add_float("x", 0.0, 1.0);
        let config = OptimizationConfig::new("min_test".into(), space, "random")
            .with_objective("max_drawdown", ObjectiveDirection::Minimize);
        let mut status = OptimizationStatus::new(config);

        let result_high = TrialResult {
            trial_id: Uuid::new_v4(),
            objective: 0.15,
            metrics: HashMap::new(),
            parameters: HashMap::new(),
            duration_seconds: None,
        };
        status.update_best(&result_high);
        assert_eq!(status.best_trial.as_ref().unwrap().objective, 0.15);

        let result_low = TrialResult {
            trial_id: Uuid::new_v4(),
            objective: 0.05,
            metrics: HashMap::new(),
            parameters: HashMap::new(),
            duration_seconds: None,
        };
        status.update_best(&result_low);
        assert_eq!(status.best_trial.as_ref().unwrap().objective, 0.05);
    }

    #[test]
    fn trial_lifecycle() {
        let opt_id = Uuid::new_v4();
        let mut params = HashMap::new();
        params.insert("short_period".into(), ParameterValue::Int(10));

        let mut trial = Trial::new(opt_id, 1, params.clone());
        assert_eq!(trial.status, TrialStatus::Pending);

        trial.mark_running(Some("worker-0".into()));
        assert_eq!(trial.status, TrialStatus::Running);
        assert_eq!(trial.worker_id.as_deref(), Some("worker-0"));

        let result = TrialResult {
            trial_id: trial.id,
            objective: 1.8,
            metrics: HashMap::new(),
            parameters: params,
            duration_seconds: Some(5),
        };
        trial.mark_completed(result);
        assert_eq!(trial.status, TrialStatus::Completed);
        assert!(trial.finished_at.is_some());
        assert_eq!(trial.result.as_ref().unwrap().objective, 1.8);
    }

    #[test]
    fn trial_failure() {
        let mut trial = Trial::new(Uuid::new_v4(), 0, HashMap::new());
        trial.mark_running(None);
        trial.mark_failed("backtest panicked".into());
        assert_eq!(trial.status, TrialStatus::Failed);
        assert_eq!(trial.error.as_deref(), Some("backtest panicked"));
    }
}
