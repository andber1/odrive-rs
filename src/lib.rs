/// The `commands` module contains the ODrive structure, which is used to interact with the ODrive
/// protocol.
pub mod commands;

/// The `enumerations` module contains enums and constants related to different properties and
/// errors.
pub mod enumerations;

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod test_stream;

pub mod prelude {
    pub use crate::commands::ODrive;
    pub use crate::enumerations::errors::{
        AxisError, ControllerError, EncoderError, MotorError, ODriveError, ODriveResult,
    };
    pub use crate::enumerations::{AxisID, AxisState, ControlMode, EncoderMode, MotorType};
}
