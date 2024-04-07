// Embedded controller command port.
const EC_COMMAND_PORT: u16 = 0x66;
// Embedded controller data port.
const EC_DATA_PORT: u16 = 0x62;

// Command to get temperature.
const COMMAND_TEMP: u8 = 0x9E;
// Command to set speed.
const COMMAND_SPEED: u8 = 0x99;

// Id for fan.
const FAN_ID: u8 = 0x01;

// Min speed as percentage to run the fan.
const FAN_SPEED_MIN: u8 = 30;
// Max speed as percentage to run the fan.
const FAN_SPEED_MAX: u8 = 100;

// Minimum temperature from start to raise fan speed.
const MIN_TEMP: u8 = 70;
// Maximun temperature that starts raising fan speed even if temperature is not raising.
const MAX_TEMP: u8 = 85;

// Wait time between loops in ms.
const REFRESH_RATE: u64 = 250;

// Wait this many milliseconds to change speed when temperature is raising.
const REACTION_TIME_MS_RAISE: u64 = 1000;
// Wait this many milliseconds to change speed when temperature is lowering or staying same.
const REACTION_TIME_MS_LOWER: u64 = 2000;

// Increment as percentage to raise fan speed.
const FAN_RAISE_INCREMENT: u8 = 5;
// Increment as percentage to lower fan speed.
const FAN_LOWER_INCREMENT: u8 = 1;

// How many milliseconds to wait for command flag.
const COMMAND_FLAG_MAX_WAIT_MS: u128 = 1000;

// Command line arguments.
#[derive(Debug, clap::Parser)]
struct Cli {
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}

// Failed to set IO permission error.
#[derive(Debug)]
pub(crate) struct FailedToSetIOPermissionError {
    port: u16,
    return_value: i32,
}
impl FailedToSetIOPermissionError {
    pub fn new(port: u16, return_value: i32) -> FailedToSetIOPermissionError {
        FailedToSetIOPermissionError { port, return_value }
    }
}
impl std::error::Error for FailedToSetIOPermissionError {}
impl std::fmt::Display for FailedToSetIOPermissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Failed to set IO permission to port {} got return value {}!",
            self.port, self.return_value
        )
    }
}

// Set IO permission to port.
fn set_port_io_permission(port: u16) -> Result<(), FailedToSetIOPermissionError> {
    let return_value: i32 = unsafe { libc::ioperm(port as u64, 1, 1) };
    if return_value != 0 {
        return Err(FailedToSetIOPermissionError::new(port, return_value));
    }
    return Ok(());
}

// Initialize embedded controller.
fn ec_init() -> Result<(), FailedToSetIOPermissionError> {
    set_port_io_permission(EC_DATA_PORT)?;
    set_port_io_permission(EC_COMMAND_PORT)?;
    return Ok(());
}

// Command flag wait timed out error.
#[derive(Debug)]
pub(crate) struct CommandFlagWaitTimedOutError {
    flag: Flag,
    on: bool,
}
impl CommandFlagWaitTimedOutError {
    pub fn new(flag: Flag, on: bool) -> CommandFlagWaitTimedOutError {
        CommandFlagWaitTimedOutError { flag, on }
    }
}
impl std::error::Error for CommandFlagWaitTimedOutError {}
impl std::fmt::Display for CommandFlagWaitTimedOutError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Timed out when waiting for command flag {:?} to be {}!",
            self.flag,
            (if self.on { "on" } else { "off" })
        )
    }
}

#[derive(Debug, Clone)]
enum Flag {
    OBF,
    IBF,
}
impl Flag {
    // Flag as u8 value.
    fn flag(&self) -> u8 {
        return match self {
            Flag::OBF => 0x1,
            Flag::IBF => 0x2,
        };
    }

    // Is flag on in given output?
    fn on(&self, output: u8) -> bool {
        let flag: u8 = self.flag();
        return output & flag == flag;
    }

    // Wait for flag to be on.
    fn wait_for_on(&self) -> Result<(), CommandFlagWaitTimedOutError> {
        return self.wait(true);
    }

    // Wait for flag to be off.
    fn wait_for_off(&self) -> Result<(), CommandFlagWaitTimedOutError> {
        return self.wait(false);
    }

    // Wait for flag to be given on status.
    fn wait(&self, on: bool) -> Result<(), CommandFlagWaitTimedOutError> {
        // Command port with readonly access.
        let mut port_command: x86_64::instructions::port::PortGeneric<
            u8,
            x86_64::instructions::port::ReadOnlyAccess,
        > = x86_64::instructions::port::PortReadOnly::<u8>::new(EC_COMMAND_PORT);

        // Init start time.
        let start: std::time::Instant = std::time::Instant::now();

        // Wait for flag to be given on status.
        while self.on(unsafe { port_command.read() }) != on {
            // If max timeout has been reached
            if COMMAND_FLAG_MAX_WAIT_MS < start.elapsed().as_millis() {
                // return error.
                return Err(CommandFlagWaitTimedOutError::new(self.clone(), on));
            }
        }

        // Flag was set to asked on status.
        return Ok(());
    }
}

// Write given value to given port.
fn write_to_port(port: u16, value: u8) -> Result<(), CommandFlagWaitTimedOutError> {
    // Wait for input buffer flag to be off.
    Flag::IBF.wait_for_off()?;

    // Write the value to port.
    unsafe { x86_64::instructions::port::PortWriteOnly::<u8>::new(port).write(value) };

    Ok(())
}

// Send command to embedded controller.
fn send_command(command: u8) -> Result<(), CommandFlagWaitTimedOutError> {
    return write_to_port(EC_COMMAND_PORT, command);
}

// Write data to embedded controller.
fn write_data(data: u8) -> Result<(), CommandFlagWaitTimedOutError> {
    return write_to_port(EC_DATA_PORT, data);
}

// Set data speed.
fn set_fan_speed(speed: u8) -> Result<(), CommandFlagWaitTimedOutError> {
    send_command(COMMAND_SPEED)?;
    write_data(FAN_ID)?;
    return write_data(
        ((std::cmp::min(speed, 100) as f32 / 100 as f32) * 255 as f32).floor() as u8,
    );
}

// Flush embedded controller.
fn ec_flush() {
    // Init readonly access to command and data ports.
    let mut port_command: x86_64::instructions::port::PortGeneric<
        u8,
        x86_64::instructions::port::ReadOnlyAccess,
    > = x86_64::instructions::port::PortReadOnly::<u8>::new(EC_COMMAND_PORT);
    let mut port_data: x86_64::instructions::port::PortGeneric<
        u8,
        x86_64::instructions::port::ReadOnlyAccess,
    > = x86_64::instructions::port::PortReadOnly::<u8>::new(EC_DATA_PORT);

    // While output buffer flag is on
    while Flag::OBF.on(unsafe { port_command.read() }) {
        // read data.
        unsafe { port_data.read() };
    }
}

// Read byte from enbedded controller.
fn read_byte() -> Result<u8, CommandFlagWaitTimedOutError> {
    // Wait for output buffer flag to be on.
    Flag::OBF.wait_for_on()?;

    // Return read byte.
    Ok(unsafe { x86_64::instructions::port::PortReadOnly::<u8>::new(EC_DATA_PORT).read() })
}

// Get local temperature.
fn get_local_temp() -> Result<u8, CommandFlagWaitTimedOutError> {
    ec_flush();
    send_command(COMMAND_TEMP)?;
    write_data(FAN_ID)?;
    return read_byte();
}

// Collects all of the errors that can occur when creating a new connection.
#[derive(Debug)]
pub(crate) enum RunError {
    FailedToSetIOPermissionError(FailedToSetIOPermissionError),
    CommandFlagWaitTimedOutError(CommandFlagWaitTimedOutError),
}
impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RunError::FailedToSetIOPermissionError(e) => {
                write!(f, "Failed to set IO permission error:\n{}", e)
            }
            RunError::CommandFlagWaitTimedOutError(e) => {
                write!(f, "Command flag wait timed out error:\n{}", e)
            }
        }
    }
}
impl From<FailedToSetIOPermissionError> for RunError {
    fn from(err: FailedToSetIOPermissionError) -> Self {
        RunError::FailedToSetIOPermissionError(err)
    }
}
impl From<CommandFlagWaitTimedOutError> for RunError {
    fn from(err: CommandFlagWaitTimedOutError) -> Self {
        RunError::CommandFlagWaitTimedOutError(err)
    }
}

// Run fan controller.
fn run() -> Result<(), RunError> {
    // Init logger from command line arguments.
    env_logger::Builder::new()
        .filter_level(<Cli as clap::Parser>::parse().verbose.log_level_filter())
        .init();

    // Speed of fan when last set.
    let mut fan_speed_last: u8 = 0;
    // Set to this fan speed.
    let mut fan_speed: u8 = FAN_SPEED_MIN;

    // Init enbedded controller.
    ec_init()?;

    // Set fan speed.
    set_fan_speed(fan_speed)?;

    // Sleep milli second amount.
    let sleep_time: std::time::Duration = std::time::Duration::from_millis(REFRESH_RATE);
    // Wait this many loops to change speed when temperature is raising.
    let reaction_loops_raise: u64 = REACTION_TIME_MS_RAISE / REFRESH_RATE;
    // Wait this many loops to change speed when temperature is lowering or staying same.
    let reaction_loops_lower: u64 = REACTION_TIME_MS_LOWER / REFRESH_RATE;

    // Temperature.
    let mut temp: u8 = get_local_temp()?;
    // Last loop temperature.
    let mut temp_last: u8 = temp.clone();

    // How many loops has temperature been raising.
    let mut raising_loops: u64 = 0;
    // How many loops has temperature been lowering or staying the same.
    let mut lowering_or_staying_loops: u64 = 0;

    // Infinite loop.
    loop {
        // Get temperature.
        temp = get_local_temp()?;
        // Print out temperature.
        log::info!("Temperature: {} C", temp);

        // If temperature is over the max
        if MAX_TEMP < temp
            // or
            || (
                // min temperature has been reached
                MIN_TEMP < temp
                // and 
                &&
                // temperature is raising.
                temp_last < temp
            )
        {
            // Output information about it.
            log::info!("Raising or over max!");

            // Increase raising loops.
            raising_loops += 1;

            // If has been raising more than reaction time gives.
            if reaction_loops_raise < raising_loops {
                // Raise fan speed by defined increment.
                fan_speed = std::cmp::min(fan_speed + FAN_RAISE_INCREMENT, FAN_SPEED_MAX);

                // Zero loop counters.
                lowering_or_staying_loops = 0;
                raising_loops = 0;
            }
        }
        // Fan speed is not raising or over the max.
        else {
            // Inform about it.
            log::info!("Lowering or staying the same.");
            // Increase loop counter.
            lowering_or_staying_loops += 1;

            // If has been lowering or staying the same, more than reaction time gives.
            if reaction_loops_lower < lowering_or_staying_loops {
                // Lower fan speed by defined increment.
                if FAN_LOWER_INCREMENT < fan_speed {
                    fan_speed = std::cmp::max(fan_speed - FAN_LOWER_INCREMENT, FAN_SPEED_MIN);
                } else {
                    fan_speed = FAN_SPEED_MIN;
                }

                // Zero loop counters.
                lowering_or_staying_loops = 0;
                raising_loops = 0;
            }
        }

        // If fan speed changed.
        if fan_speed != fan_speed_last {
            // Output about the change.
            log::info!("Changing fan speed {} % => {} %", fan_speed_last, fan_speed);

            // Zero loop counters.
            lowering_or_staying_loops = 0;
            raising_loops = 0;

            // Set fan speed.
            set_fan_speed(fan_speed)?;

            // Save fan speed as last fan speed.
            fan_speed_last = fan_speed;
            // Update last loop temperature from this loop temperature.
            temp_last = temp;
        }
        // Fan speed did not change.
        else {
            log::info!("Keeping fan speed at {} %", fan_speed);
        }

        // Sleep set milli seconds.
        std::thread::sleep(sleep_time);
    }
}

fn main() {
    match run() {
        Ok(()) => std::process::exit(0),
        Err(error) => {
            eprintln!("Got error {}", error);
            std::process::exit(1);
        }
    }
}
