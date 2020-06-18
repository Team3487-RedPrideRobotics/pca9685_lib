use rppal::i2c;
use std::time::Duration;

use async_std::task;

/// Mode Register 1
pub const PCA9685_MODE1: u8 = 0x00;
/// Mode Register 2
pub const PCA9685_MODE2: u8 = 0x01;
/// I2C-Bus subaddress 1
pub const PCA9865_SUBADR1: u8 = 0x02;
/// I2C-Bus subaddress 2
pub const PCA9865_SUBADR2: u8 = 0x03;
/// I2C-Bus subaddress 3
pub const PCA9865_SUBADR3: u8 = 0x04;
/// LED All Call I2C-bus address
pub const PCA9685_ALLCALLADR: u8 = 0x05;
/// Where to turn the signal on, low byte
pub const PCA9685_LED0_ON_L: u8 = 0x06;
/// Where to turn the signal on, high byte
pub const PCA9685_LED0_ON_H: u8 = 0x07;
/// Where to turn the signal off, low byte
pub const PCA9685_LED0_OFF_L: u8 = 0x08;
/// Where to turn the signal off, high byte
pub const PCA9685_LED0_OFF_H: u8 = 0x09;
/// Where to turn all signals on, low byte
pub const PCA9685_ALLLED_ON_L: u8 = 0xFA;
/// Where to turn all signals on, high byte  
pub const PCA9685_ALLLED_ON_H: u8 = 0xFB;
/// Where to turn all signals off, low byte
pub const PCA9685_ALLLED_OFF_L: u8 = 0xFC; 
/// Where to turn all signals off, high byte
pub const PCA9685_ALLLED_OFF_H: u8 = 0xFD;
/// Prescaler for PWM Output frequency
pub const PCA9685_PRESCALE: u8 = 0xFE;
/// Defines the test mode to be entered
pub const PCA9685_TESTMODE: u8 = 0xFF;

// MODE1 bits
/// Respond to LED All Call I2C-bus address
pub const MODE1_ALLCAL: u8 = 0x01;
/// respond to I2C-bus subaddress 3
pub const MODE1_SUB3: u8 = 0x02;
/// respond to I2C-bus subaddress 2 
pub const MODE1_SUB2: u8 = 0x04;
/// respond to I2C-bus subaddress 1
pub const MODE1_SUB1: u8 = 0x08;
/// Low power mode. Oscillator off
pub const MODE1_SLEEP: u8 = 0x10;
/// Auto-Increment enabled
pub const MODE1_AI: u8 = 0x20;
/// Use EXTCLK pin clock
pub const MODE1_EXTCLK: u8 = 0x40;
/// Restart Enabled
pub const MODE1_RESTART: u8 = 0x80;
// MODE2 bits
///Active LOW output enable input
pub const MODE2_OUTNE_0: u8 = 0x01;
/// Active LOW output enable input - high impedence
pub const MODE2_OUTNE_1: u8 = 0x02;
/// totem pole structure vs open-drain
pub const MODE2_OUTDRV: u8 = 0x04;
/// Outputs change on ACK vs STOP 
pub const MODE2_OCH: u8 = 0x08;
/// Output logic state inverted
pub const MODE2_INVRT: u8 = 0x10;

/// Int. osc. frequency in datasheet
pub const FREQUENCY_OSCILLATOR: u32 = 25000000; /**<  */

/// minimum prescale value
pub const PCA9685_PRESCALE_MIN: u8 = 3;
/// Maximum prescale value
pub const PCA9685_PRESCALE_MAX: u8 = 255;

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
    /// Restarts the deice and sets a prescale
    /// 0 is for using a default prescale.
    /// # Panics
    /// If the prescale is less than the minium, or greater than the max, 
    /// this function will panic.
    pub async fn begin(&mut self, prescale: u8 ) -> Result<(), i2c::Error> {
        let prescale = match prescale {
            p if p == 0 => {
                return self.set_pwm_freq(1000.0).await;
            }
            p if p < PCA9685_PRESCALE_MIN => panic!("Prescale is less than it should be!"),
            p if p > PCA9685_PRESCALE_MAX => panic!("Prescale is greater than it should be!"),
            _ => {
                if let Err(e) = self.set_ext_clock(prescale).await {
                    return Err(e);
                }
            }
        };

        

        self.set_osc_frequency(self.oscillator_freq);
        self.reset().await
    }
    /// Send a reset command to the chip
    pub async fn reset(&mut self) -> Result<(), rppal::i2c::Error> {
        let res = self.bus.write(&vec![PCA9685_MODE1, MODE1_RESTART]);
        if let Err(e) = res {
            return Err(e);
        }

        task::sleep(Duration::from_millis(10)).await;
        Ok(())
    }
    /// Put the chip into sleep mode
    pub async fn sleep(&mut self) -> Result<(), rppal::i2c::Error> {
        let mut buf = vec![0 as u8];
        if let Err(e) = self.bus.write_read(&vec![PCA9685_MODE1], &mut buf) {
            return Err(e);
        }

        let awake = buf.get(0).unwrap();
        let sleep = awake | MODE1_SLEEP;

        if let Err(e) = self.bus.write(&vec![PCA9685_MODE1, sleep]) {
            return Err(e);
        }

        task::sleep(Duration::from_millis(5)).await;

        Ok(())
    }
    /// Awaken the chip out of sleep mode
    pub async fn wakeup(&mut self) -> Result<(), rppal::i2c::Error> {
        let mut buf = vec![0 as u8];
        if let Err(e) = self.bus.write_read(&vec![PCA9685_MODE1], &mut buf) {
            return Err(e);
        }

        let sleep = buf.get(0).unwrap();
        let awake = sleep & !MODE1_SLEEP;

        if let Err(e) = self.bus.write(&vec![PCA9685_MODE1, awake]) {
            return Err(e);
        }

        Ok(())
    }
    /// Sets the pwm output of a pin based on the input of microseconds
    /// 
    /// **Imprecise:** This function is not 100% accurate due to the nature of the chip.
    pub fn write_micros(&mut self, channel: u8, micros: u16) -> Result<(), i2c::Error> {

        let pulse = micros;

        let (prescale, result) = self.get_prescale();

        if let Err(e) = result {
            return Err(e);
        }

        let prescale = prescale + 1;

        let pulselength = 1_000_000 * prescale;
        let pulselength = pulselength / self.oscillator_freq as usize;
        
        let pulse = pulse / pulselength as u16;

        if let Err(e) = self.set_pwm(channel, 0, pulse as u16) {
            return Err(e);
        }

        Ok(())
    }
    /// Sets the PWM frequency for the entire chip, up to ~1.6 KHz
    /// # Panics
    /// This function will panic if the frequency is not within 1 < freq < 3052
    /// 
    /// **Imprecise:** This function is not 100% accurate due to the nature of the chip.
    pub async fn set_pwm_freq(&mut self, freq: f32) -> Result<(), rppal::i2c::Error> {
        match freq {
            f if f < 1.0 => panic!("Frequency Cannot Be Lower than 1!"),
            f if f > 3052.0 => panic!("Datasheet Max is 3052hz"),
            _ => {},
        }

        let prescale_val = ((self.oscillator_freq as f32 / (freq * 4096.0)) + 0.5) - 1.0;
        let prescale = match prescale_val {
            p if p < PCA9685_PRESCALE_MIN as f32 => PCA9685_PRESCALE_MIN as f32,
            p if p > PCA9685_PRESCALE_MAX as f32 => PCA9685_PRESCALE_MAX as f32,
            _ => prescale_val
        };

        let mut buf = vec![0 as u8];
        if let Err(e) = self.bus.write_read(&vec![PCA9685_MODE1], &mut buf) {
            return Err(e);
        }

        let oldmode = buf.get(0).unwrap();
        let newmode = (oldmode * !MODE1_RESTART) | MODE1_SLEEP;

        if let Err(e) = self.bus.write(&vec![
            PCA9685_MODE1, newmode
        ]) {
            return Err(e);
        }

        if let Err(e) = self.bus.write(&vec![
            PCA9685_PRESCALE, prescale as u8
        ]) {
            return Err(e);
        }

        if let Err(e) = self.bus.write(&vec![
            PCA9685_MODE1, *oldmode
        ]) {
            return Err(e);
        }

        task::sleep(Duration::from_millis(5)).await;

        Ok(())
        
    }
    /// Helper to set pin PWM Output. Sets pin wihtout having to deal with
    /// on/off tick placement and properly handles a zero value as completely off and
    /// 4095 as completely on
    pub fn set_pin(&mut self, channel: u8, mut on_tick: u16, invert: bool) -> Result<usize,rppal::i2c::Error> {
        if on_tick > 4095 {
            on_tick = 4095;
        }

        if invert {
            if on_tick == 0 {
                self.set_pwm(channel, 4096, 0)
            } else if on_tick == 4095 {
                self.set_pwm(channel, 0, 4096)
            } else {
                self.set_pwm(channel, 0, 4095-on_tick)
            }
        } else {
            if on_tick == 4095 {
                self.set_pwm(channel, 4096, 0)
            } else if on_tick == 0 {
                self.set_pwm(channel, 0, 4096)
            } else {
                self.set_pwm(channel, 0, on_tick)
            }
        }

    }
    /// Set the PWM output of one of the pins on the chip
    /// # Panics
    /// This function will panic if channel is > 15
    pub fn set_pwm(&mut self, channel: u8, on: u16, off: u16) -> Result<usize, rppal::i2c::Error> {
        let on_bytes = on.to_be_bytes();
        let off_bytes = off.to_be_bytes();

        if channel > 15 {
            panic!("There are only 16 channels on the chip.");
        }

        self.bus.write(&vec![
            PCA9685_LED0_ON_L + 4 * channel,
            on_bytes[1],
            on_bytes[0],
            off_bytes[1],
            off_bytes[0],
        ])
    }

    /// Getter for the internally tracked oscillator used for freq calculations
    pub fn get_osc_frequency(&self) -> u32 {
        self.oscillator_freq
    }

    /// Sets the internally tracked oscillator used for frequency calculations.
    pub fn set_osc_frequency(&mut self, freq: u32) {
        self.oscillator_freq = freq;
    }
    /// Just returns the private prescale value. (PCA9685 doesn't have introspection)
    pub fn get_prescale(&mut self) -> (usize, i2c::Result<()>) {
        let mut buf = vec![0 as u8];
        let result = self.bus.write_read( &vec![PCA9685_PRESCALE],&mut buf[..]);
        ((*buf.get(0).unwrap()).into(), result)
    }

    ///Sets EXTCLK pin to use the external clock.
    pub async fn set_ext_clock(&mut self, prescale: u8) -> Result<(), rppal::i2c::Error> {
        let mut buf = vec![0 as u8];
        let result = self.bus.write_read(&vec![PCA9685_MODE1], &mut buf);
        
        if let Err(e) = result {
            return Err(e);
        }

        let oldmode = buf.get(0).unwrap();
        let newmode = (oldmode & !MODE1_RESTART) | MODE1_SLEEP;

        if let Err(e) = self.bus.write(&vec![
            PCA9685_MODE1,
            newmode
        ]){
            return Err(e);
        }

        if let Err(e) = self.bus.write(&vec![
            PCA9685_PRESCALE,
            prescale
        ]) {
            return Err(e);
        }

        task::sleep(Duration::from_millis(5)).await;

        if let Err(e) = self.bus.write(&vec![
            PCA9685_MODE1,
            (newmode & !MODE1_SLEEP) | MODE1_RESTART | MODE1_AI
        ]) {
            return Err(e);
        }

        Ok(())
    }
    /// Sets the output mode of the PCA9685 to either open drain or push pull / totempole
    /// # Warning
    /// LEDs with integrated zener diodes should
    /// only be driven in open drain mode.
    pub fn set_output_mode(&mut self, totempole: bool) -> Result<(), rppal::i2c::Error> {
        let mut buf = vec![0 as u8];
        if let Err(e) = self.bus.write_read(
            &vec![PCA9685_MODE2], &mut buf
        ) {
            return Err(e);
        }

        let oldmode = buf.get(0).unwrap();
        let newmode: u8;
        if totempole {
            newmode = oldmode | MODE2_OUTDRV;
        } else {
            newmode = oldmode & !MODE2_OUTDRV;
        }

        if let Err(e) = self.bus.write(&vec![
            PCA9685_MODE2, newmode
        ]) {
            return Err(e);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
