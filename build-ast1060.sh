#!/bin/bash

# Hubris AST1060-Starter Build Script
# This script builds the ast1060-starter application and verifies success

set -e  # Exit on any error

# Configuration
APP_NAME="ast1060-starter"
APP_CONFIG="app/${APP_NAME}/app.toml"
BUILD_DIR="target/${APP_NAME}/dist/default"
FIRMWARE_BIN="${BUILD_DIR}/final.bin"
FIRMWARE_ELF="${BUILD_DIR}/final.elf"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${CYAN}[STEP]${NC} $1"
}

# Function to check if file exists and get its size
check_file() {
    local file="$1"
    local description="$2"
    
    if [ -f "$file" ]; then
        local size=$(stat -c%s "$file" 2>/dev/null || stat -f%z "$file" 2>/dev/null)
        log_success "$description found (${size} bytes): $file"
        return 0
    else
        log_error "$description not found: $file"
        return 1
    fi
}

# Function to check build artifacts
check_build_artifacts() {
    log_step "Checking build artifacts..."
    
    local success=true
    
    # Check main firmware files
    check_file "$FIRMWARE_BIN" "Firmware binary" || success=false
    check_file "$FIRMWARE_ELF" "Firmware ELF" || success=false
    
    # Check individual task binaries
    local tasks=("jefe" "idle" "helloworld" "uart_driver" "digest_server")
    for task in "${tasks[@]}"; do
        check_file "${BUILD_DIR}/${task}" "Task binary ($task)" || success=false
    done
    
    # Check kernel
    check_file "${BUILD_DIR}/kernel" "Kernel binary" || success=false
    
    if [ "$success" = true ]; then
        log_success "All build artifacts verified successfully"
        return 0
    else
        log_error "Some build artifacts are missing"
        return 1
    fi
}

# Function to analyze task sizes
analyze_task_sizes() {
    log_step "Analyzing task sizes..."
    
    if [ -f "${BUILD_DIR}/final.elf" ]; then
        echo -e "${CYAN}Task Memory Usage:${NC}"
        
        # Use objdump or size command to analyze sections if available
        if command -v size >/dev/null 2>&1; then
            for task in jefe idle helloworld uart_driver digest_server; do
                if [ -f "${BUILD_DIR}/${task}" ]; then
                    echo -n "  $task: "
                    size "${BUILD_DIR}/${task}" 2>/dev/null | tail -n1 | awk '{printf "text=%d data=%d bss=%d total=%d bytes\n", $1, $2, $3, $1+$2+$3}' || echo "analysis failed"
                fi
            done
        else
            log_warning "size command not available, skipping detailed analysis"
        fi
        
        # Show total firmware size
        local fw_size=$(stat -c%s "$FIRMWARE_BIN" 2>/dev/null || stat -f%z "$FIRMWARE_BIN" 2>/dev/null)
        echo -e "  ${CYAN}Total firmware size: ${fw_size} bytes${NC}"
    fi
}

# Function to perform build
build_firmware() {
    log_step "Building $APP_NAME firmware..."
    
    # Check if app config exists
    if [ ! -f "$APP_CONFIG" ]; then
        log_error "Application configuration not found: $APP_CONFIG"
        return 1
    fi
    
    log_info "Using configuration: $APP_CONFIG"
    log_info "Build command: cargo xtask dist $APP_CONFIG"
    
    # Run the build
    if cargo xtask dist "$APP_CONFIG"; then
        log_success "Build completed successfully"
        return 0
    else
        log_error "Build failed"
        return 1
    fi
}

# Function to clean previous build (optional)
clean_build() {
    if [ "$1" = "--clean" ] || [ "$1" = "-c" ]; then
        log_step "Cleaning previous build..."
        if [ -d "target/${APP_NAME}" ]; then
            rm -rf "target/${APP_NAME}"
            log_info "Removed target/${APP_NAME}"
        fi
        
        # Also clean cargo cache for this app
        cargo clean --package ast1060-starter 2>/dev/null || true
        cargo clean --package task-helloworld 2>/dev/null || true
        cargo clean --package digest-server 2>/dev/null || true
        log_info "Cleaned cargo cache"
    fi
}

# Function to show build summary
show_summary() {
    log_step "Build Summary"
    
    echo -e "${CYAN}Application:${NC} $APP_NAME"
    echo -e "${CYAN}Configuration:${NC} $APP_CONFIG"
    echo -e "${CYAN}Build Directory:${NC} $BUILD_DIR"
    
    if [ -f "$FIRMWARE_BIN" ]; then
        local fw_size=$(stat -c%s "$FIRMWARE_BIN" 2>/dev/null || stat -f%z "$FIRMWARE_BIN" 2>/dev/null)
        echo -e "${CYAN}Firmware Size:${NC} $fw_size bytes"
        echo -e "${CYAN}Firmware Path:${NC} $FIRMWARE_BIN"
    fi
    
    echo -e "${GREEN}Build completed successfully!${NC}"
    echo
    echo -e "${YELLOW}Next steps:${NC}"
    echo "  • Test with QEMU: ./run-qemu-debug.sh"
    echo "  • Flash to hardware: use $FIRMWARE_BIN"
    echo "  • Debug with GDB: gdb $FIRMWARE_ELF"
}

# Main execution
main() {
    echo -e "${BLUE}=== Hubris AST1060-Starter Build Script ===${NC}"
    echo
    
    # Handle command line arguments
    clean_build "$1"
    
    # Change to script directory
    cd "$(dirname "$0")"
    
    # Verify we're in the right directory
    if [ ! -f "Cargo.toml" ] || [ ! -d "app/ast1060-starter" ]; then
        log_error "Not in Hubris repository root. Please run from the main Hubris directory."
        exit 1
    fi
    
    # Show environment info
    log_info "Rust version: $(rustc --version 2>/dev/null || echo 'Not available')"
    log_info "Cargo version: $(cargo --version 2>/dev/null || echo 'Not available')"
    log_info "Working directory: $(pwd)"
    echo
    
    # Perform the build
    if build_firmware; then
        echo
        
        # Verify build artifacts
        if check_build_artifacts; then
            echo
            
            # Analyze task sizes
            analyze_task_sizes
            echo
            
            # Show summary
            show_summary
            
            exit 0
        else
            log_error "Build verification failed"
            exit 1
        fi
    else
        log_error "Build failed"
        exit 1
    fi
}

# Show usage information
show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo
    echo "Options:"
    echo "  -c, --clean    Clean previous build artifacts before building"
    echo "  -h, --help     Show this help message"
    echo
    echo "Examples:"
    echo "  $0              # Build normally"
    echo "  $0 --clean     # Clean and build"
}

# Handle help option
if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    show_usage
    exit 0
fi

# Run main function
main "$@"
