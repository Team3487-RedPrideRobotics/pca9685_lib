use rppal::i2c;
use std::time::Duration;
use log::{debug};
use tokio::time::delay_for;

/// Mode Register 1
pub const MODE1: u8 = 0x00;
/// Mode Register 2
pub const MODE2: u8 = 0x01;
/// I2C-Bus subaddress 1
pub const SUBADR1: u8 = 0x02;
/// I2C-Bus subaddress 2
pub const SUBADR2: u8 = 0x03;
/// I2C-Bus subaddress 3
pub const SUBADR3: u8 = 0x04;
/// LED All Call I2C-bus address
pub const ALLCALLADR: u8 = 0x05;
/// Where to turn the signal on, low byte
pub const LED0_ON_L: u8 = 0x06;
/// Where to turn the signal on, high byte
pub const LED0_ON_H: u8 = 0x07;
/// Where to turn the signal off, low byte
pub const LED0_OFF_L: u8 = 0x08;
/// Where to turn the signal off, high byte
pub const LED0_OFF_H: u8 = 0x09;
/// Where to turn all signals on, low byte
pub const ALL_LED_ON_L: u8 = 0xFA;
/// Where to turn all signals on, high byte  
pub const ALL_LED_ON_H: u8 = 0xFB;
/// Where to turn all signals off, low byte
pub const ALL_LED_OFF_L: u8 = 0xFC; 
/// Where to turn all signals off, high byte
pub const ALL_LED_OFF_H: u8 = 0xFD;
/// Prescaler for PWM Output frequency
pub const PRE_SCALE: u8 = 0xFE;
/// Defines the test mode to be entered
/// **DO NOT USE**
pub const TESTMODE: u8 = 0xFF;

/// MODE1 bits
pub mod mode1 {
    
    /// Respond to LED All Call I2C-bus address
    pub const ALLCALL: u8 = 0x01;
    /// respond to I2C-bus subaddress 3
    pub const SUB3: u8 = 0x02;
    /// respond to I2C-bus subaddress 2 
    pub const SUB2: u8 = 0x04;
    /// respond to I2C-bus subaddress 1
    pub const SUB1: u8 = 0x08;
    /// Low power mode. Oscillator off
    pub const SLEEP: u8 = 0x10;
    /// Auto-Increment
    pub const AI: u8 = 0x20;
    /// Use EXTCLK pin clock
    pub const EXTCLK: u8 = 0x40;
    /// Restart Enabled
    pub const RESTART: u8 = 0x80;
}
/// MODE2 bits
pub mod mode2 {
    ///Active LOW output enable input
    pub const OUTNE_0: u8 = 0x01;
    /// Active LOW output enable input - high impedence
    pub const OUTNE_1: u8 = 0x02;
    /// totem pole structure vs open-drain
    pub const OUTDRV: u8 = 0x04;
    /// Outputs change on ACK vs STOP 
    pub const OCH: u8 = 0x08;
    /// Output logic state inverted
    pub const INVRT: u8 = 0x10;
}

/// Int. osc. frequency in datasheet
pub const FREQUENCY_OSCILLATOR: u32 = 25000000; /**<  */

/// minimum prescale value
pub const PRESCALE_MIN: u8 = 3;
/// Maximum prescale value
pub const PRESCALE_MAX: u8 = 255;

/// A Representation of a PCA9685 Chip
pub struct PCA9685 {
    address: u8,
    bus: i2c::I2c,
    oscillator_freq: u32,
}

impl PCA9685 {

    /// Creates a new PCA9865
    pub fn new(address: u8, bus: i2c::I2c) -> Result<PCA9685, i2c::Error> {
        let mut dev = PCA9685 {
            address,
            bus,
            oscillator_freq: FREQUENCY_OSCILLATOR,
        };
        if let Err(e) = dev.bus.set_slave_address(dev.address as u16) {
            return Err(e);
        }
        Ok(dev)
    }

    /// Start of the PCA9865
    pub async fn start(&mut self) -> Result<(), i2c::Error> {
        //Read Mode 1
        let mut mode = vec![0];
        if let Err(e) = self.bus.write_read(&vec![MODE1], &mut mode) {
            return Err(e)
        }
        let mode = mode.get(0).unwrap();
        debug!(target: "PCA9686_events", "Current mode {:#b}", mode);

        //Clear Sleep bit
        if let Err(e) = self.bus.write(&vec![MODE1, mode - mode1::SLEEP]) {
            return Err(e);
        }

        //Wait for at least 500us, stabilize oscillator
        delay_for(Duration::from_micros(500)).await;

        // Write a logic 1 to bit 7 to clear, if needed
        if let Err(e) = self.bus.write(&vec![MODE1, *mode]) {
            return Err(e);
        }

        //Debug Check the Mode
        let mut debug_mode = vec![0];
        if let Err(e) = self.bus.write_read(&vec![MODE1], &mut debug_mode) {
            return Err(e);
        } else {
            debug!("Mode: {:#b}", debug_mode.get(0).unwrap());
        }

        Ok(())
    }

    /// Reads the prescale directly from the chip.
    pub fn read_prescale(&mut self) -> Result<u8, i2c::Error> {
        let mut prescale_buf = vec![0];
        debug!(target: "PCA96585_events", "Reading prescale");
        if let Err(e) = self.bus.write_read(&vec![PRE_SCALE], &mut prescale_buf) {
            return Err(e);
        }
        let prescale = prescale_buf.get(0).unwrap();
        debug!(target: "PCA96585_events", "Prescale is {}", prescale);
        Ok(*prescale)
    }

}

/// Sets the prescale value for the chip from a given frequency
pub fn prescale_from_freq(clock_speed: u32, freq: u16) -> u8 {
    (((clock_speed as f32) / (4096 as f32 * freq as f32)) - 1.0) as u8
}

#[cfg(test)]
mod tests {
    #[test]
    fn freq_from_prescale() {
        use super::prescale_from_freq;
        assert_eq!(prescale_from_freq(25_000_000, 200), 0xE);
    }
}
