use log::{debug, info};
use rppal::i2c;
use std::time::Duration;
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
pub const FREQUENCY_OSCILLATOR: u32 = 25_000_000;

/// minimum prescale value
pub const PRESCALE_MIN: u8 = 0x03;

/// Maximum prescale value
pub const PRESCALE_MAX: u8 = 0xFF;

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
        dev.bus.set_slave_address(dev.address as u16)?;
        Ok(dev)
    }

    /// Start the PCA9865.
    /// The chip needs a little time to start.
    pub async fn start(&mut self) -> Result<(), i2c::Error> {
        info!(target: "PCA9685_events", "Starting chip");

        //Read Mode 1
        let mut mode = vec![0];
        self.bus.write_read(&vec![MODE1], &mut mode)?;
        let mode = mode.get(0).unwrap();
        debug!(target: "PCA9685_events", "Current mode {:#b}", mode);

        //Clear Sleep bit
        debug!(target: "PCA9685_events", "Writing to mode 1: {:#b}", mode-mode1::SLEEP);
        self.bus.write(&vec![MODE1, mode - mode1::SLEEP])?;

        //Wait for at least 500us, stabilize oscillator
        delay_for(Duration::from_micros(500)).await;

        // Write a logic 1 to bit 7 to clear, if needed
        self.bus.write(&vec![MODE1, (*mode-mode1::SLEEP)])?;

        //Debug Check the Mode
        let mut debug_mode = vec![0];
        if let Err(e) = self.bus.write_read(&vec![MODE1], &mut debug_mode) {
            return Err(e);
        } else {
            debug!(target: "PCA9685_events", "Mode: {:#b}", debug_mode.get(0).unwrap());
        }
        info!(target: "PCA9685_events", "Started Chip!");
        Ok(())
    }

    /// Put the chip into sleep
    pub fn sleep(&mut self) -> Result<(), i2c::Error> {
        info!(target: "PCA9685_events", "Going to sleep");
        //Get the current mode
        let mut mode = vec![0];
        self.bus.write_read(&vec![MODE1], &mut mode)?;
        let mode = mode.get(0).unwrap();
        debug!(target: "PCA9685_events", "Current mode {:#b}", mode);
        
        //If chip is not in sleep
        if mode & mode1::SLEEP == 0 {
            //Go to sleep
            let mut buf = vec![0];
            let mode_sleep =  mode + mode1::SLEEP;
            debug!(target: "PCA9685_events", "Writing sleep mode {:#b}", mode);
            if let Err(e) = self.bus.write_read(&vec![MODE1,mode_sleep], &mut buf) {
                return Err(e);
            } else {
                debug!(target: "PCA9685_events", "Mode: {:#b}", buf.get(0).unwrap());
            }
        }
        info!(target: "PCA9685_events","Put the chip to sleep!");
        Ok(())
    }

    /// Set the prescale value from a given frequency
    /// # Warnings
    /// - In order to change the prescale, the chip must be put into sleep.
    /// Make sure that anything important be safetied before use.
    /// 
    /// - This function tries to be as close as possible to the given frequency.
    pub async fn set_prescale_fr(&mut self, frequency: u16) -> Result<(), i2c::Error> {
        self.sleep()?;
        //Get the old prescale for debug purposes
        let mut prescale_buf = vec![0];
        if let Err(e) = self.bus.write_read(&vec![PRE_SCALE], &mut prescale_buf) {
            return Err(e);
        } {
            let old_prescale = prescale_buf.get(0).unwrap();
            debug!(target: "PCA9685_events", "Old Prescale is {:#X}", old_prescale);
        }
        
        //Get the new prescale
        let mut prescale_val = prescale_from_freq(self.oscillator_freq, frequency);
        match prescale_val {
            v if v > PRESCALE_MAX => prescale_val = PRESCALE_MAX,
            v if v < PRESCALE_MIN => prescale_val = PRESCALE_MIN,
            _ => {}
        }

        //Set the Prescale
        if let Err(e) = self.bus.write_read(&vec![PRE_SCALE, prescale_val], &mut prescale_buf) {
            return Err(e);
        } else {
            info!(target: "PCA9685_events","New Prescale is {:#X}", prescale_buf.get(0).unwrap());
        }
        
        //Start the chip again
        self.start().await?;

        Ok(())
    }

    /// Reads the prescale directly from the chip.
    pub fn read_prescale(&mut self) -> Result<u8, i2c::Error> {
        let mut prescale_buf = vec![0];
        debug!(target: "PCA9685_events", "Reading prescale");
        self.bus.write_read(&vec![PRE_SCALE], &mut prescale_buf)?;
        let prescale = prescale_buf.get(0).unwrap();
        debug!(target: "PCA9685_events", "Prescale is {}", prescale);
        Ok(*prescale)
    }

    /// Set the pulse-widths for a channel.
    /// Channels range from 0 - 15.
    /// Since the device uses 12bit accuracy,
    /// the tuples are arranged as (most_significant, least_significant)
    /// # Panics
    /// The channel must be less than 16.
    pub fn set_channel(&mut self, channel: u8, period_on: (u8, u8), period_off: (u8, u8)) -> Result< (), i2c::Error> {
        if channel  > 16 {
            panic!("Channel must be less than 16");
        }
        
        //Write to the four registers
        self.bus.write(&vec![LED0_ON_L + 4*channel, period_on.1])?;
        self.bus.write(&vec![LED0_ON_H, period_on.0])?;
        self.bus.write(&vec![LED0_OFF_L, period_off.1])?;
        self.bus.write(&vec![LED0_OFF_H, period_off.0])?;

        Ok(())
    }

    /// Set to use External Clock
    /// # Warnings
    /// - In order to use the EXTCLK pin, the chip must be put to sleep.
    /// - In order to reset this mode, you have to run a power cycle (or software reset).
    /// - Max frequency is 50 Mhz
    /// - **Untested**
    pub async fn set_external_clock(&mut self, clock_speed: u32) -> Result<(), i2c::Error> {
        // Go to sleep
        self.sleep()?;

        //Get the current mode
        let mut mode = vec![0];
        self.bus.write_read(&vec![MODE1], &mut mode)?;
        let mode = mode.get(0).unwrap();
        debug!(target: "PCA9685_events", "Current mode {:#b}", mode);

        //Write logic 1 to sleep & EXTCLK,
        self.bus.write(&vec![MODE1, mode + mode1::EXTCLK])?;

        //Wake up
        self.start().await?;

        self.oscillator_freq = clock_speed;
        Ok(())
    }

    /// Set the output mode of the chip
    /// Options: Open-Drain or Totem pole.
    /// # Default
    /// Totem pole
    /// # Warnings
    /// - LEDS with built in zener diodes should only be 
    /// driven in open drain mode.
    /// - **Untested**
    pub fn set_output_mode(&mut self, open_drain: bool) -> Result<(), i2c::Error> {
        //Get the old mode2
        let mut mode = vec![0];
        self.bus.write_read(&vec![MODE2], &mut mode)?;
        let mode = mode.get(0).unwrap();
        //If open drain mode
        if open_drain {
            //Since mode2::OUTDRV is default 1, if 1
            if mode & mode2::OUTDRV == mode2::OUTDRV {
                //Change to 0
                self.bus.write(&vec![MODE2, mode - mode2::OUTDRV])?;
                info!(target: "PCA9685_events", "Set to Open-Drain");
            }
        } else {
            if mode & mode2::OUTDRV == 0 {
                //Change to 1
                self.bus.write(&vec![MODE2, mode + mode2::OUTDRV])?;
                info!(target: "PCA9685_events", "Set to Totem Pole");
            }
        }

        Ok(())
    }

    /// Gets the prescale value
    pub fn get_prescale(&mut self) -> Result<u8, i2c::Error> {
        let mut buf = vec![0];
        self.bus.write_read(&vec![PRE_SCALE], &mut buf)?;

        Ok(*buf.get(0).unwrap())

    }

}

/// Gets a prescale value for the chip from a given frequency
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
