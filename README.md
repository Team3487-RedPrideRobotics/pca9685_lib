# pca9685_lib ![Version Badge](https://img.shields.io/crates/v/pca9685_lib?style=for-the-badge)
Raspberry Pi drivers for the PCA9685
[Documentation](https://team3487-redpriderobotics.github.io/pca9685_lib/pca9685_lib/)

This uses tokio to allow for other tasks to be run while the program runs for an (admittedly short period)
of time, 500us, but since rustaceans are writing in systems level code, I thought this might squeeze a little 
more performance. 

# Quickstart 
```rust
use rppal::i2c::{I2c};
use pca9865_lib::PCA9685;
use tokio::time::delay_for;
use std::time::Duration;

#[tokio::main]
async fn main() {
    //Create a new device with address 0x40. Note mutability.
    let mut device = PCA9685::new(0x40, I2c::new().unwrap()).unwrap();

    //Set the refresh rate to 50Hz, (re)start the chip when complete
    if let Err(_e) = device.set_prescale_fr(50, true).await {
        panic!();
    }

    //Servo fun time
    loop {
        
        //The chip divides the refresh rate into 4096 blocks
        //The first tuple is which block to turn on the pulse
        //The second tuple is which block to turn the pulse off (in this case ~two milliseconds after);
        if let Err(_e) = device.set_channel(0, (0, 0), (0x01, 0x97)) {
            panic!();
        }
        delay_for(Duration::from_secs(2)).await;
        //Set Mid
        if let Err(_e) = device.set_channel(0, (0, 0), (0x01, 0x33)) {
            panic!();
        }
        delay_for(Duration::from_secs(2)).await;
        //Set Min
        if let Err(_e) = device.set_channel(0, (0, 0), (0x00, 0xCD)) {
            panic!();
        }
        delay_for(Duration::from_secs(2)).await;
        //Set Mid
        if let Err(_e) = device.set_channel(0, (0, 0), (0x01, 0x33)) {
            panic!();
        }
        delay_for(Duration::from_secs(2)).await;
    }

}

```
