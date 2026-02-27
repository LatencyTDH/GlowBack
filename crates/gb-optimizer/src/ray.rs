//! Ray cluster configuration and task descriptors for distributed execution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::search::ParameterValue;

/// Configuration for connecting to a Ray cluster.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RayClusterConfig {
    /// Ray head node address (e.g. "ray://localhost:10001").
    pub address: String,

    /// Namespace for this optimization run.
    pub namespace: String,

    /// Runtime environment packages (pip requirements).
    pub runtime_env: Option<RuntimeEnv>,

    /// Resource requirements per worker.
    pub worker_resources: WorkerResources,

    /// Maximum number of concurrent Ray tasks.
    pub max_concurrent_tasks: usize,
}

impl Default for RayClusterConfig {
    fn default() -> Self {
        Self {
            address: "ray://localhost:10001".to_string(),
            namespace: "glowback".to_string(),
            runtime_env: None,
            worker_resources: WorkerResources::default(),
            max_concurrent_tasks: 4,
        }
    }
}

/// Runtime environment for Ray workers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEnv {
    /// Python packages to install on workers.
    pub pip: Vec<String>,
    /// Working directory (uploaded to cluster).
    pub working_dir: Option<String>,
    /// Environment variables.
    pub env_vars: HashMap<String, String>,
}

/// Resource requirements for a single Ray worker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerResources {
    /// Number of CPUs per worker (fractional ok).
    pub num_cpus: f64,
    /// Number of GPUs per worker (0 = no GPU).
    pub num_gpus: f64,
    /// Memory in bytes (0 = no limit).
    pub memory_bytes: u64,
    /// Custom resource requirements.
    pub custom: HashMap<String, f64>,
}

impl Default for WorkerResources {
    fn default() -> Self {
        Self {
            num_cpus: 1.0,
            num_gpus: 0.0,
            memory_bytes: 0,
            custom: HashMap::new(),
        }
    }
}

/// Describes a single backtest task to be dispatched to a Ray worker.
///
/// The Python Ray integration layer converts this descriptor into a
/// `@ray.remote` function call that executes the backtest with the given
/// parameter overrides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RayTaskDescriptor {
    /// Unique task id (matches the trial id).
    pub task_id: Uuid,

    /// Optimization run this task belongs to.
    pub optimization_id: Uuid,

    /// Trial sequence number (0-indexed).
    pub trial_number: usize,

    /// Parameter overrides to inject into the base backtest config.
    pub parameters: HashMap<String, ParameterValue>,

    /// Serialized base backtest config (JSON).
    pub base_config: serde_json::Value,

    /// The metric to extract from the backtest result.
    pub objective_metric: String,

    /// Resource requirements for this specific task.
    pub resources: WorkerResources,
}

/// Allocation plan produced by the optimizer for the Ray dispatcher.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerAllocation {
    /// Total number of workers to request.
    pub num_workers: usize,

    /// Per-worker resource spec.
    pub resources: WorkerResources,

    /// Task descriptors ready to dispatch.
    pub tasks: Vec<RayTaskDescriptor>,

    /// Cluster config to use.
    pub cluster: RayClusterConfig,
}

impl WorkerAllocation {
    /// Create an allocation for a batch of tasks.
    pub fn new(cluster: RayClusterConfig, tasks: Vec<RayTaskDescriptor>) -> Self {
        let num_workers = cluster.max_concurrent_tasks.min(tasks.len());
        let resources = cluster.worker_resources.clone();
        Self {
            num_workers,
            resources,
            tasks,
            cluster,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cluster_config() {
        let config = RayClusterConfig::default();
        assert_eq!(config.address, "ray://localhost:10001");
        assert_eq!(config.namespace, "glowback");
        assert_eq!(config.max_concurrent_tasks, 4);
    }

    #[test]
    fn worker_allocation_caps_at_task_count() {
        let mut cluster = RayClusterConfig::default();
        cluster.max_concurrent_tasks = 10;

        let tasks: Vec<RayTaskDescriptor> = (0..3)
            .map(|i| RayTaskDescriptor {
                task_id: Uuid::new_v4(),
                optimization_id: Uuid::new_v4(),
                trial_number: i,
                parameters: HashMap::new(),
                base_config: serde_json::Value::Null,
                objective_metric: "sharpe_ratio".to_string(),
                resources: WorkerResources::default(),
            })
            .collect();

        let alloc = WorkerAllocation::new(cluster, tasks);
        assert_eq!(alloc.num_workers, 3); // capped at task count
        assert_eq!(alloc.tasks.len(), 3);
    }

    #[test]
    fn task_descriptor_serialization() {
        let mut params = HashMap::new();
        params.insert("lr".to_string(), ParameterValue::Float(0.01));

        let task = RayTaskDescriptor {
            task_id: Uuid::new_v4(),
            optimization_id: Uuid::new_v4(),
            trial_number: 0,
            parameters: params,
            base_config: serde_json::json!({"strategy": "ma_crossover"}),
            objective_metric: "sharpe_ratio".to_string(),
            resources: WorkerResources::default(),
        };

        let json = serde_json::to_string(&task).unwrap();
        let back: RayTaskDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(task, back);
    }

    #[test]
    fn runtime_env_round_trip() {
        let env = RuntimeEnv {
            pip: vec!["numpy".into(), "pandas".into()],
            working_dir: Some("/tmp/gb".into()),
            env_vars: {
                let mut m = HashMap::new();
                m.insert("RUST_LOG".into(), "info".into());
                m
            },
        };

        let json = serde_json::to_string(&env).unwrap();
        let back: RuntimeEnv = serde_json::from_str(&json).unwrap();
        assert_eq!(env, back);
    }
}
