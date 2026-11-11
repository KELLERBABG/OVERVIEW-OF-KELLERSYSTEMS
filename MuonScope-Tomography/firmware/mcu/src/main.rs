//! Embedded MCU Firmware Entrypoint (no_std)
//! Target: ARM Cortex-M7 (STM32H7 / RP2040 / ESP32-S3)

#![no_std]
#![no_main]

use panic_halt as _;
use cortex_m_rt::entry;

mod sipm_dac_pid;

use sipm_dac_pid::SipmBiasPidController;

#[entry]
fn main() -> ! {
    // Breakdown voltage: 53.0 V at 25 deg C, +4.5 V overvoltage
    let mut bias_controller = SipmBiasPidController::new(53.0, 4.5, -0.054);

    loop {
        // Simulated 100 Hz PID cycle reading PT1000 temperature sensor
        let measured_temp_c = 24.5;
        let dac_code = bias_controller.update_dac_code(measured_temp_c);

        // Send dac_code to 16-bit precision SPI DAC...
        let _ = dac_code;
    }
}
