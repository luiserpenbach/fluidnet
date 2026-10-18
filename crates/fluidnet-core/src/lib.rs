//! fluidnet-core: lumped-parameter fluid network simulation.
//!
//! * `props`    real-fluid property tables (CoolProp-generated), (P, h) based
//! * `catalog`  component definitions (ports, parameters, defaults)
//! * `network`  model compilation and governing equations
//! * `flow`     restriction, friction and heat-transfer correlations
//! * `solver`   TR-BDF2 transient integrator, pseudo-transient steady solver
//! * `sim`      high-level run functions and results

pub mod catalog;
pub mod combustion;
pub mod error;
pub mod flow;
pub mod machines;
pub mod model;
pub mod network;
pub mod props;
pub mod sim;
pub mod solver;
pub mod units;

pub use error::{Error, Result};
pub use model::Model;
pub use network::Network;
pub use props::{FluidLibrary, FluidTable, Props};
pub use sim::{run_model, run_steady, run_transient, Mode, Results};
