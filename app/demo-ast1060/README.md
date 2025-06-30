# AST1060 Demo Application

This is a demonstration application for the AST1060 SoC platform running on Hubris. It showcases the key capabilities of the AST1060 including hardware-accelerated cryptographic operations.

## Features Demonstrated

### Hardware Acceleration
- **HACE (Hash and Crypto Engine)**: Hardware-accelerated SHA-256 and HMAC-SHA256 operations
- **Performance Testing**: Comparison between hardware and software implementations
- **Incremental Operations**: Streaming hash/HMAC operations for large data

### System Integration
- **Multi-task Architecture**: Demonstrates Hubris task communication and coordination
- **IPC Communication**: Inter-Process Communication between tasks using IDL
- **Timer Management**: Periodic operations and system heartbeat
- **Error Handling**: Robust error handling and recovery

### Test Scenarios
1. **Basic Hash Operations**: Single-shot SHA-256 hashing
2. **Basic HMAC Operations**: Single-shot HMAC-SHA256 with keys
3. **Incremental Hash**: Multi-chunk streaming hash operations
4. **Incremental HMAC**: Multi-chunk streaming HMAC operations
5. **Performance Benchmarks**: Timing comparisons and throughput tests

## Architecture

The demo consists of several tasks:

- **`jefe`**: System supervisor task
- **`hmac_hash`**: HMAC+Hash server providing hardware acceleration
- **`demo_task`**: Main demo task that exercises the crypto functionality
- **`ping`/`pong`**: Simple IPC demonstration tasks
- **`hiffy`**: Interactive debugging and testing interface
- **`idle`**: Low-priority idle task

## Building

To build the demo for AST1060:

```bash
# From the hubris root directory
cargo xtask build --bin demo-ast1060 --image a
```

## Running

Deploy the built image to your AST1060 development board according to your board's programming procedure.

### Expected Output

When running, you should see output similar to:

```
AST1060 Demo Task Starting!
Testing HMAC+Hash hardware acceleration...
Testing basic SHA256 hash...
Hash computed successfully: [8f, 43, 7d, 2b, 5c, 3e, 1f, 9a]
Testing basic HMAC-SHA256...
HMAC computed successfully: [a1, b2, c3, d4, e5, f6, 78, 90]
Testing incremental SHA256 hash...
SHA256 initialized
Updated with 7 bytes
Updated with 7 bytes
Updated with 12 bytes
Updated with 5 bytes
Incremental hash completed: [12, 34, 56, 78, 9a, bc, de, f0]
Testing incremental HMAC-SHA256...
HMAC-SHA256 initialized
HMAC updated with 12 bytes
HMAC updated with 5 bytes
HMAC updated with 3 bytes
HMAC updated with 7 bytes
Incremental HMAC completed: [fe, dc, ba, 98, 76, 54, 32, 10]
Initial tests complete:
  Hash operations: 2
  HMAC operations: 2
  Test passes: 4
  Test failures: 0
```

## Testing with Hiffy

The demo includes a Hiffy task that allows interactive testing:

```bash
# Connect to the target and access Hiffy console
humility hiffy
```

From the Hiffy console, you can manually test individual operations:

```hiffy
# Test basic hash
hash = call("hmac_hash", "digest_sha256", (20, [0x48, 0x65, 0x6c, 0x6c, 0x6f, ...]))

# Test HMAC
hmac = call("hmac_hash", "hmac_sha256", (20, 10, [data...], [key...]))

# Check task status
status = call("demo_task", "get_counters")
```

## Hardware Requirements

- AST1060 SoC development board
- Sufficient flash memory (>64KB recommended)
- UART interface for console output
- JTAG/SWD interface for debugging (optional)

## Development Notes

### Adding New Tests

To add new cryptographic tests:

1. Add test functions to `task/demo-ast1060/src/main.rs`
2. Call them from the main loop or timer handler
3. Update counters and logging as appropriate

### Modifying Hardware Configuration

The hardware peripheral configuration is in:
- `boards/ast1060-dev.toml` - Board-specific configuration
- `chips/ast1060/chip.toml` - SoC peripheral definitions

### Performance Tuning

Key areas for performance optimization:
- Buffer sizes in the HMAC+Hash server
- Task priorities and stack sizes
- Timer intervals for periodic operations

## Troubleshooting

### Common Issues

1. **Build Errors**: Ensure all dependencies are properly configured in Cargo.toml files
2. **Hardware Access**: Verify peripheral addresses match your AST1060 variant
3. **Task Communication**: Check that task slots are properly configured in app.toml
4. **Memory Usage**: Monitor flash and RAM usage, increase limits if needed

### Debug Output

Enable additional debug output by modifying the demo task to include more detailed logging. The task uses standard `println!` macros which output via the UART console.

### Hardware Verification

To verify the HACE controller is working:
1. Check that the driver initializes without errors
2. Compare hash outputs with known test vectors
3. Verify performance improvements over software implementations

## License

This demo application follows the same licensing as the Hubris operating system.
