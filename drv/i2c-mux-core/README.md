# Generic I2C Mux Implementation

This demonstrates the factored I2C mux drivers for Hubris/STM32 platforms.

## STM32 Integration

```rust
use drv_i2c_mux_core::{pca9548::Pca9548, I2cMuxConfig, I2cMuxDriver};
use crate::mux_adapters::{Stm32I2cDevice, Stm32GpioPin};

let generic_driver = Pca9548;
let config = I2cMuxConfig {
    address: 0x70,
    reset_pin: Some(gpio_pin),
};
let i2c_device = Stm32I2cDevice { controller, ctrl };

// Configure the mux
generic_driver.configure(&mut config)?;

// Enable segment 3
generic_driver.enable_segment(&i2c_device, &config, Some(Segment::S3))?;
```

## Benefits of Factorization

1. **Single Implementation**: Each mux device (PCA9548, PCA9545) has only one implementation
2. **Testability**: Generic implementations can be unit tested with mock I2C/GPIO
3. **Maintainability**: Bug fixes and features only need to be implemented once
4. **Consistency**: Same behavior across all STM32/Hubris platforms