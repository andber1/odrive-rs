use std::io::{Error, Read, Write};
use std::io;
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::enumerations::{AxisState, Axis};

#[cfg(test)]
mod tests;

/// The `ODrive` struct manages a connection with an ODrive motor over the ASCII protocol.
/// It acts as a newtype around a connection stream.
/// This has been tested using serial types from `serialport-rs`.
#[derive(Debug, Default, Ord, PartialOrd, Eq, PartialEq, Clone)]
pub struct ODrive<T> {
    io_stream: T
}

impl<T> ODrive<T> {
    /// Although any type can be passed in here, it is suggested that the supplied type `T` be
    /// `Read + Write`. Doing so will unlock the full API.
    pub fn new(io_stream: T) -> Self {
        Self { io_stream }
    }
}

/// An implementation of `Write` has been provided as an escape hatch to enable the usage of
/// operations not yet supported by this library.
impl<T> Write for ODrive<T> where T: Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.io_stream.write(buf)
    }

    fn flush(&mut self) -> Result<(), Error> {
        self.io_stream.flush()
    }
}

/// An implementation of `Write` has been provided as an escape hatch to enable the usage of
/// operations not yet supported by this library. Be advised that using this implementation may
/// place the connection into an inconsistent state.
impl<T> Read for ODrive<T> where T: Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        self.io_stream.read(buf)
    }
}

impl<T> ODrive<T> where T: Read {
    /// Reads the next message sent by the ODrive as a string.
    /// If their is no message, this function should return an empty string.
    pub fn read_string(&mut self) -> io::Result<String> {
        let mut string = String::with_capacity(20);
        let duration = Instant::now();
        loop {
            let mut buffer = [0; 1];
            while self.io_stream.read(&mut buffer).unwrap_or_default() == 0 {
                if duration.elapsed().as_millis() >= 1_000 {
                    return Ok(string);
                }
            }
            let ch = buffer[0];
            if ch as char == '\n' {
                break;
            }

            string.push(ch as char);
        }

        Ok(string.trim().to_owned())
    }

    /// Reads the next message as a float. This will return zero if the message is not a valid
    /// float.
    pub fn read_float(&mut self) -> io::Result<f32> {
        Ok(self.read_string()?.parse().unwrap_or_default())
    }

    /// Reads the next message as an int. This will return zero if the message is not a valid int.
    pub fn read_int(&mut self) -> io::Result<i32> {
        Ok(self.read_string()?.parse().unwrap_or_default())
    }
}

impl<T> ODrive<T> where T: Write {
    /// Move the motor to a position. Use this command if you have a real-time controller which
    /// is streaming setpoints and tracking a trajectory.
    /// `axis` The motor to be used for the operation.
    /// `position` is the desired position, in encoder counts.
    /// `velocity_feed_forward` is the velocity feed forward term, in encoder counts per second.
    /// `current_feed_forward` is the current feed forward term, in amps.
    /// If `None` is supplied for a feed forward input, zero will be provided as a default.
    pub fn set_position_p(&mut self, axis: Axis, position: f32, velocity_feed_forward: Option<f32>, current_feed_forward: Option<f32>) -> io::Result<()> {
        let velocity_feed_forward = velocity_feed_forward.unwrap_or_default();
        let current_feed_forward = current_feed_forward.unwrap_or_default();
        writeln!(self.io_stream, "p {} {} {} {}", axis as u8, position, velocity_feed_forward, current_feed_forward)?;
        self.flush()
    }

    /// Move the motor to a position. Use this command if you are sending one setpoint at a time.
    /// `axis` The motor to be used for the operation.
    /// `position` is the desired position, in encoder counts.
    /// `velocity_limit` is the velocity limit, in encoder counts per second.
    /// `current_limit` is the current limit, in amps.
    /// If `None` is supplied for a limit, zero will be provided as a default.
    pub fn set_position_q(&mut self, axis: Axis, position: f32, velocity_limit: Option<f32>, current_limit: Option<f32>) -> io::Result<()> {
        let velocity_feed_forward = velocity_limit.unwrap_or_default();
        let current_feed_forward = current_limit.unwrap_or_default();
        writeln!(self.io_stream, "q {} {} {} {}", axis as u8, position, velocity_feed_forward, current_feed_forward)?;
        self.flush()
    }

    /// Specifies a velocity setpoint for the motor.
    /// `axis` The motor to be used for the operation.
    /// `velocity` is the velocity setpoint, in encoder counts per second.
    /// `current_feed_forward` is the current feed forward term, in amps.
    /// If `None` is supplied for a feed forward input, zero will be provided as a default.
    pub fn set_velocity(&mut self, axis: Axis, position: f32, current_feed_forward: Option<f32>) -> io::Result<()> {
        let current_feed_forward = current_feed_forward.unwrap_or_default();
        writeln!(self.io_stream, "v {} {} {}", axis as u8, position, current_feed_forward)?;
        self.flush()
    }

    /// Specifies a velocity setpoint for the motor.
    /// `axis` The motor to be used for the operation.
    /// `current` is the current to be supplied, in amps.
    pub fn set_current(&mut self, axis: Axis, current: f32) -> io::Result<()> {
        writeln!(self.io_stream, "c {} {}", axis as u8, current)?;
        self.flush()
    }

    /// Moves a motor to a given position
    /// For general movement, this is the best command.
    /// `axis` The motor to be used for the operation.
    /// `position` is the desired position, in encoder counts.
    pub fn set_trajectory(&mut self, axis: Axis, position: f32) -> io::Result<()> {
        writeln!(self.io_stream, "t {} {}", axis as u8, position)?;
        self.flush()
    }
}

impl<T> ODrive<T> where T: Read + Write {
    pub fn get_velocity(&mut self, axis: Axis) -> io::Result<f32> {
        writeln!(self.io_stream, "r axis{} .encoder.vel_estimate", axis as u8)?;
        self.flush()?;
        self.read_float()
    }

    pub fn run_state(&mut self, axis: Axis, requested_state: AxisState, wait: bool) -> io::Result<bool> {
        let mut timeout_ctr = 100;
        writeln!(self.io_stream, "w axis{}.requested_state {}", axis as u8, requested_state as u8)?;
        self.flush()?;
        if wait {
            while {
                sleep(Duration::from_millis(100));
                writeln!(self.io_stream, "r axis{}.current_state", axis as u8)?;
                self.flush()?;
                timeout_ctr -= 1;
                self.read_int().unwrap_or_default() != AxisState::Idle as i32 && timeout_ctr > 0
            } {}
        }

        Ok(timeout_ctr > 0)
    }
}