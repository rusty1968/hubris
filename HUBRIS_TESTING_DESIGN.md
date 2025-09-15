# Hubris Testing Framework Design

## Overview

Hubris implements a sophisticated **on-device testing framework** that runs directly on target hardware or simulators. Unlike traditional unit tests that run on the host, Hubris tests validate the complete system including kernel interactions, hardware peripherals, and real-time behavior in the actual embedded environment.

## Architecture

### Core Components

The testing framework consists of multiple cooperating tasks that communicate via Hubris IPC:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   test-runner   │───▶│   test-suite    │───▶│   test-assist   │
│   (orchestrator)│    │   (test cases)  │    │   (IPC helper)  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
    Coordinates            Contains actual        Tests kernel IPC,
    test execution         test functions         faults, etc.
                                  │
                                  ▼
                        ┌─────────────────┐
                        │ test-idol-server│
                        │ (Idol IPC tests)│
                        └─────────────────┘
```

#### 1. **test-runner** (`test/test-runner/`)
- **Role**: Test orchestrator and coordinator
- **Priority**: Highest (0) - manages test lifecycle
- **Responsibilities**:
  - Coordinates test execution sequence
  - Manages test state and results
  - Interfaces with external test harnesses

#### 2. **test-suite** (`test/test-suite/`)
- **Role**: Contains actual test case implementations
- **Priority**: Medium (2) - executes individual tests
- **Responsibilities**:
  - Implements test case functions
  - Validates system behavior
  - Reports test results via panic/assert for failures

#### 3. **test-assist** (`test/test-assist/`)
- **Role**: IPC interaction helper
- **Priority**: High (1) - responds to test requests
- **Responsibilities**:
  - Provides target for IPC testing
  - Simulates various fault conditions
  - Tests raw kernel interactions

#### 4. **test-idol-server** (`test/test-idol-server/`)
- **Role**: Idol-mediated IPC testing
- **Priority**: High (1) - handles Idol requests
- **Responsibilities**:
  - Tests type-safe Idol IPC
  - Validates code generation
  - Tests client-server interactions

#### 5. **test-api** (`test/test-api/`)
- **Role**: Common types and operations
- **Scope**: Shared library
- **Provides**:
  - Operation enumerations (`SuiteOp`, `AssistOp`, `RunnerOp`)
  - Test result types
  - Communication protocols

### Test Applications per Hardware Platform

The framework supports multiple hardware platforms through dedicated test applications:

```
test/
├── tests-gimletlet/        # STM32H7 (Gimletlet hardware)
├── tests-stm32h7/          # Generic STM32H7 platforms
├── tests-lpc55xpresso/     # NXP LPC55 development boards
├── tests-stm32fx/          # STM32F4 platforms
├── tests-stm32g0/          # STM32G0 platforms
├── tests-gemini-bu/        # Gemini board unit
├── tests-psc/              # Power supply controller
└── tests-rot-carrier/      # RoT carrier boards
```

Each test application contains:
- **app.toml**: Task configuration with all test components
- **Cargo.toml**: Dependencies and build configuration
- Hardware-specific test configurations

## Test Definition and Execution

### Test Case Definition

Tests are defined using the `test_cases!` macro in `test-suite`:

```rust
test_cases! {
    // IPC and Communication Tests
    test_send,                    // Basic IPC send operations
    test_recv_reply,             // IPC reply handling
    test_recv_reply_fault,       // Fault handling in IPC
    test_send_never,             // Edge case handling

    // Memory Management Tests
    test_borrow_info,            // Memory lease validation
    test_borrow_read,            // Read lease operations
    test_borrow_write,           // Write lease operations

    // Kernel Feature Tests
    test_notifications,          // Notification system
    test_timer,                  // Timer functionality
    test_irq,                    // Interrupt handling

    // Fault Injection Tests
    test_fault_badmemory,        // Memory protection
    test_fault_stackoverflow,    // Stack overflow detection
    test_fault_divzero,          // Division by zero handling

    // Task Management Tests
    test_task_config,            // Task configuration validation
    test_task_restart,           // Task restart behavior
    test_task_status,            // Task status reporting
}
```

### Test Execution Flow

1. **Initialization**
   - All test tasks start and establish IPC connections
   - Test runner initializes coordination state

2. **Test Coordination**
   - External harness (Hiffy/Humility) triggers test execution
   - Test runner coordinates with test suite via `SuiteOp::RunCase`
   - Individual test cases execute in isolated contexts

3. **Test Validation**
   - Test cases use `assert!`, `panic!`, or explicit result reporting
   - Failed assertions cause task panics, indicating test failure
   - Successful completion indicates test pass

4. **Result Reporting**
   - Test results propagate through IPC back to runner
   - External tools (Humility) collect and report overall results

### Inter-Task Communication Testing

The framework extensively tests Hubris IPC mechanisms:

#### Raw IPC Tests (`test-assist`)
```rust
pub enum AssistOp {
    JustReply,              // Basic reply functionality
    SendBack,               // Echo operations
    BadMemory,              // Memory fault injection
    Panic,                  // Panic behavior
    StackOverflow,          // Stack overflow simulation
    BusError,               // Hardware fault simulation
    IllegalInstruction,     // CPU fault injection
    // ... more operations
}
```

#### Idol IPC Tests (`test-idol-server`)
- Tests type-safe code generation
- Validates serialization/deserialization
- Tests error propagation through Idol interfaces

## Integration with External Tools

### Humility Integration

Tests integrate with the **Humility** debugger for comprehensive validation:

```bash
# Build, flash, and run tests
cargo xtask test app/tests-gimletlet/app.toml

# Run tests without flashing (reuse existing image)
cargo xtask test app/tests-gimletlet/app.toml --noflash
```

**Humility** provides:
- Test orchestration from host
- Result collection and reporting
- Debugging support for test failures
- Hardware-in-the-loop test execution

### Hiffy Integration

Tests can be triggered through **Hiffy** (Hubris Interactive Function Interface):

```rust
// Hiffy function to run a specific test case
pub(crate) fn run_a_test(
    stack: &[Option<u32>],
    _data: &[u8],
    rval: &mut [u8],
) -> Result<usize, Failure> {
    let test_id = stack[0].unwrap();

    // Trigger test execution via IPC
    let (rc, _len) = sys_send(
        TEST_TASK.get_task_id(),
        SuiteOp::RunCase as u16,
        test_id.as_bytes(),
        &mut [],
        &[],
    );

    // Process and return results
    // ...
}
```

## Hardware and Driver Testing

### Peripheral Driver Validation

The framework supports testing hardware drivers through:

1. **Hardware Loopback Testing**
   - Connect hardware signals for loopback validation
   - Test actual peripheral register access
   - Validate timing and electrical characteristics

2. **Mock Device Testing**
   - Simulate device responses for protocol validation
   - Test error handling without hardware dependencies
   - Validate state machines and edge cases

3. **Integration Testing**
   - Test complete driver stacks (driver + server + client)
   - Validate IPC interfaces between components
   - Test real-world usage patterns

### Example: I2C Driver Testing Pattern

For testing I2C drivers (like AST1060), the pattern would be:

```rust
test_cases! {
    // Basic Operations
    test_i2c_write_read,         // Basic write-read transactions
    test_i2c_write_read_block,   // SMBus block operations
    test_i2c_timing_config,      // Speed configuration

    // Error Handling
    test_i2c_bus_error,          // Bus error recovery
    test_i2c_timeout,            // Transaction timeouts
    test_i2c_invalid_address,    // Address validation

    // Mux Operations
    test_i2c_mux_switching,      // Mux segment selection
    test_i2c_mux_isolation,      // Segment isolation

    // Hardware Integration
    test_i2c_interrupt_handling, // IRQ processing
    test_i2c_concurrent_access,  // Multi-controller usage

    // Trace and Debug
    test_i2c_trace_logging,      // Debug trace validation
}
```

## Fault Injection and Error Testing

### Memory Protection Testing
- **Invalid memory access**: Tests MPU configuration
- **Stack overflow**: Validates stack protection
- **Buffer overruns**: Tests memory lease system

### Hardware Fault Simulation
- **Bus errors**: Simulates peripheral access failures
- **Illegal instructions**: Tests fault handler behavior
- **Interrupt handling**: Validates IRQ processing

### IPC Error Testing
- **Invalid task IDs**: Tests task validation
- **Malformed messages**: Tests protocol robustness
- **Resource exhaustion**: Tests system limits

## Advanced Testing Features

### Task Configuration Testing
```rust
// Tests the task_config! macro system
task_config::task_config! {
    foo: &'static str,
    bar: u32,
    baz: &'static [u8],
    tup: &'static [(u32, bool)],
}

// Validates configuration is properly loaded
fn test_task_config() {
    assert_eq!(config::foo(), "Hello, world");
    assert_eq!(config::bar(), 42);
    // ...
}
```

### Interrupt Testing
```rust
// Tests can configure interrupts for validation
[tasks.suite]
uses = ["spi1"]                    # Use SPI peripheral
notifications = ["test-irq"]        # Declare notification
interrupts = {"spi1.irq" = "test-irq"}  # Map interrupt
```

### Real-Time Behavior Testing
- **Timing validation**: Tests meet real-time constraints
- **Priority handling**: Validates task scheduling
- **Notification delivery**: Tests event processing

## Benefits of On-Device Testing

### Complete System Validation
- **Hardware integration**: Tests actual peripheral behavior
- **Kernel interactions**: Validates system calls and IPC
- **Memory management**: Tests MPU and memory protection
- **Real-time behavior**: Validates timing and scheduling

### Hardware-Specific Testing
- **Peripheral registers**: Tests actual hardware register access
- **Interrupt handling**: Validates IRQ processing with real hardware
- **Timing constraints**: Tests real-world timing requirements
- **Power management**: Tests sleep/wake behavior

### Comprehensive Coverage
- **Integration testing**: Full system stack validation
- **Fault injection**: Comprehensive error handling
- **Edge case testing**: Real-world failure scenarios
- **Performance validation**: Actual hardware performance

## Best Practices for Test Development

### Test Structure
1. **Isolate test cases**: Each test should be independent
2. **Clear failure modes**: Use descriptive assertions and panics
3. **Hardware abstraction**: Design tests to work across platforms
4. **Resource cleanup**: Ensure tests don't leak resources

### Error Handling
1. **Explicit validation**: Test both success and failure paths
2. **Fault injection**: Include deliberate error testing
3. **Recovery testing**: Validate system recovery from faults
4. **Boundary testing**: Test edge cases and limits

### Documentation
1. **Test purpose**: Document what each test validates
2. **Hardware requirements**: Specify any hardware dependencies
3. **Expected behavior**: Clear description of pass/fail criteria
4. **Debugging info**: Include trace logging for diagnosis

This comprehensive testing framework ensures that Hubris systems are thoroughly validated in their actual deployment environment, providing confidence in both the kernel implementation and the specific application drivers and tasks.