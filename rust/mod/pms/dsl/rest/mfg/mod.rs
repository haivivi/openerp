//! "mfg" facet — API surface for the Manufacturing App.
//! Used by factory floor operators for device provisioning,
//! batch management, firmware flashing, and license assignment.

pub mod device;
pub mod batch;
pub mod firmware;
pub mod model;

pub use device::MfgDevice;
pub use batch::MfgBatch;
pub use firmware::MfgFirmware;
pub use model::MfgModel;
