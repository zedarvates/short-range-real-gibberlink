//! # Laser Module
//!
//! Modular laser communication system with safety monitoring and power management.
//!
//! ## Modules
//! - `control`: Core laser operations and modulation
//! - `safety`: Safety monitoring and compliance
//! - `power`: Power management and efficiency
//! - `alignment`: Beam alignment and tracking
//! - `types`: Common types and structures
//! - `error`: Error types for laser operations

pub mod control;
pub mod safety;
pub mod power;
pub mod alignment;
pub mod types;
pub mod error;

// Re-export main types for backward compatibility
pub use control::LaserEngine;
pub use error::LaserError;
pub use types::{LaserConfig, LaserType, ModulationScheme, ReceptionConfig, AlignmentStatus, PowerProfile, BatteryState, PowerManagementConfig, PowerStatistics, PowerBudget, BeamAlignment};
pub use safety::SafetyMonitor;
pub use power::PowerManagement;
pub use alignment::AlignmentManager;
