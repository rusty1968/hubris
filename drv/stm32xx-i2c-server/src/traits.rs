//! Trait definitions for composable I2C service behaviors

use heapless::FnvIndexMap;

/// Generic I2C service layer - platform independent
pub trait I2cServiceLayer {
    type Error: Clone + core::fmt::Debug;
    type DriverInterface: I2cDriverInterface;
    type ControllerId: Copy + Clone + core::fmt::Debug + Eq + core::hash::Hash;
    type Request: Clone + core::fmt::Debug;
    type Response: Clone + core::fmt::Debug;
    
    /// Handle incoming IPC request
    fn handle_request(&mut self, request: Self::Request) -> Result<Self::Response, Self::Error>;
    
    /// Initialize the service layer
    fn initialize(&mut self) -> Result<(), Self::Error>;
    
    /// Get available controller IDs
    fn available_controller_ids(&self) -> &[Self::ControllerId];
    
    /// Shutdown gracefully
    fn shutdown(&mut self) -> Result<(), Self::Error>;
}

/// Low-level driver interface abstraction
pub trait I2cDriverInterface {
    type Error: Clone + core::fmt::Debug;
    type ControllerId: Copy + Clone + core::fmt::Debug + Eq + core::hash::Hash;
    
    /// Perform I2C write-read transaction
    fn write_read(&mut self, 
                  controller_id: Self::ControllerId,
                  addr: u8, 
                  write_data: &[u8], 
                  read_data: &mut [u8]) -> Result<usize, Self::Error>;
    
    /// Check if controller is ready
    fn is_ready(&self, controller_id: Self::ControllerId) -> bool;
    
    /// Reset controller to known state
    fn reset(&mut self, controller_id: Self::ControllerId) -> Result<(), Self::Error>;
}

/// Bus recovery behavior - platform agnostic interface
pub trait BusRecovery {
    type Error: Clone + core::fmt::Debug;
    type ControllerId: Copy + Clone + core::fmt::Debug + Eq + core::hash::Hash;
    type RecoveryStats: Clone + core::fmt::Debug;
    
    /// Attempt to recover a stuck I2C bus
    fn recover_bus(&mut self, controller_id: Self::ControllerId) -> Result<(), Self::Error>;
    
    /// Check if bus appears to be stuck
    fn is_bus_stuck(&self, controller_id: Self::ControllerId) -> Result<bool, Self::Error>;
    
    /// Get recovery statistics
    fn recovery_stats(&self, controller_id: Self::ControllerId) -> Self::RecoveryStats;
}

/// Power management behavior - platform agnostic interface  
pub trait PowerManagement {
    type Error: Clone + core::fmt::Debug;
    type ControllerId: Copy + Clone + core::fmt::Debug + Eq + core::hash::Hash;
    type PowerMode: Copy + Clone + core::fmt::Debug;
    
    /// Transition to specified power mode
    fn set_power_mode(&mut self, mode: Self::PowerMode) -> Result<(), Self::Error>;
    
    /// Get current power mode
    fn power_mode(&self) -> Self::PowerMode;
    
    /// Configure controller for low power mode
    fn prepare_controller_for_sleep(&mut self, controller_id: Self::ControllerId) -> Result<(), Self::Error>;
    
    /// Restore controller after wake from low power
    fn restore_controller_from_sleep(&mut self, controller_id: Self::ControllerId) -> Result<(), Self::Error>;
    
    /// Configure wakeup sources
    fn configure_wakeup(&mut self, controller_id: Self::ControllerId, enable: bool) -> Result<(), Self::Error>;
}

/// Multiplexer management behavior
pub trait MultiplexerManagement {
    type Error: Clone + core::fmt::Debug;
    type ControllerId: Copy + Clone + core::fmt::Debug + Eq + core::hash::Hash;
    type MuxAddress: Copy + Clone + core::fmt::Debug;
    type Segment: Copy + Clone + core::fmt::Debug;
    
    /// Select multiplexer segment
    fn select_mux_segment(&mut self, 
                         controller_id: Self::ControllerId,
                         mux: Self::MuxAddress, 
                         segment: Self::Segment) -> Result<(), Self::Error>;
    
    /// Get current mux state
    fn current_mux_state(&self, controller_id: Self::ControllerId) -> Option<(Self::MuxAddress, Self::Segment)>;
    
    /// Reset all multiplexers on controller
    fn reset_muxes(&mut self, controller_id: Self::ControllerId) -> Result<(), Self::Error>;
}

/// Hardware feature detection and capability management
pub trait HardwareCapabilities {
    type Features: Clone + core::fmt::Debug;
    type Error: Clone + core::fmt::Debug;
    type ControllerId: Copy + Clone + core::fmt::Debug + Eq + core::hash::Hash;
    
    /// Detect hardware capabilities at runtime
    fn detect_capabilities(&mut self) -> Result<(), Self::Error>;
    
    /// Get capabilities for specific controller
    fn controller_capabilities(&self, controller_id: Self::ControllerId) -> Option<&Self::Features>;
    
    /// Check if feature is supported
    fn supports_feature(&self, controller_id: Self::ControllerId, feature: &str) -> bool;
    
    /// Get all available controller IDs
    fn available_controllers(&self) -> &[Self::ControllerId];
}

/// Error handling and diagnostics behavior
pub trait ErrorHandling {
    type Error: Clone + core::fmt::Debug;
    type ControllerId: Copy + Clone + core::fmt::Debug + Eq + core::hash::Hash;
    type ErrorStats: Clone + core::fmt::Debug;
    
    /// Handle hardware error interrupt
    fn handle_error(&mut self, controller_id: Self::ControllerId) -> Result<(), Self::Error>;
    
    /// Get error statistics
    fn error_stats(&self, controller_id: Self::ControllerId) -> Self::ErrorStats;
    
    /// Clear error counters
    fn clear_error_stats(&mut self, controller_id: Self::ControllerId);
    
    /// Get last error details
    fn last_error(&self, controller_id: Self::ControllerId) -> Option<Self::Error>;
}

/// Bus recovery statistics
#[derive(Clone, Debug, Default)]
pub struct BusRecoveryStats {
    pub recovery_count: u32,
    pub successful_recoveries: u32,
    pub failed_recoveries: u32,
    pub last_recovery_time: Option<u64>,
}

/// Transaction timing statistics
#[derive(Clone, Debug, Default)]
pub struct TransactionStats {
    pub total_transactions: u32,
    pub successful_transactions: u32,
    pub failed_transactions: u32,
    pub average_duration_us: u32,
    pub max_duration_us: u32,
}