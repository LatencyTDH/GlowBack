//! # gb-optimizer
//!
//! Parameter search and distributed optimization orchestration for GlowBack.
//!
//! Provides search space definitions, parameter sweep strategies (grid, random,
//! Bayesian), trial tracking, and Ray-compatible task descriptors for distributed
//! execution.

mod ray;
mod search;
mod trial;

pub use ray::{RayClusterConfig, RayTaskDescriptor, WorkerAllocation};
pub use search::{
    BayesianSearch, GridSearch, ParameterDef, ParameterValue, RandomSearch, SearchSpace,
    SearchStrategy,
};
pub use trial::{
    ObjectiveDirection, OptimizationConfig, OptimizationState, OptimizationStatus, Trial,
    TrialResult, TrialStatus,
};
