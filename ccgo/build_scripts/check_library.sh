#!/bin/bash
#
# check_library.sh - Check library file architecture and platform
#
# Usage: ./check_library.sh <library_file>
#

if [ $# -eq 0 ]; then
    echo "Usage: $0 <library_file>"
    echo "Example: $0 /path/to/libccgonow.a"
    exit 1
fi

LIB_FILE="$1"

if [ ! -f "$LIB_FILE" ]; then
    echo "ERROR: File not found: $LIB_FILE"
    exit 1
fi

echo "=========================================="
echo "Library File: $LIB_FILE"
echo "=========================================="

# 1. Basic file info
echo ""
echo "--- File Type ---"
file "$LIB_FILE"

# 2. Platform-specific checks
echo ""
echo "--- Detailed Architecture Info ---"

# Check if it's a Mach-O file (macOS/iOS)
if file "$LIB_FILE" | grep -q "Mach-O"; then
    echo "Platform: macOS/iOS (Mach-O)"

    if command -v lipo &> /dev/null; then
        echo ""
        echo "Architectures:"
        lipo -info "$LIB_FILE"
    fi

    if command -v otool &> /dev/null; then
        echo ""
        echo "Header Info:"
        otool -hv "$LIB_FILE" | head -20
    fi

# Check if it's an ELF file (Linux/Android)
elif file "$LIB_FILE" | grep -q "ELF"; then
    echo "Platform: Linux/Android (ELF)"

    if command -v readelf &> /dev/null; then
        echo ""
        echo "ELF Header:"
        readelf -h "$LIB_FILE" | grep -E "(Class|Machine|OS/ABI)"

        # Detect Android-specific markers
        if readelf -n "$LIB_FILE" 2>/dev/null | grep -q "Android"; then
            echo "Detected: Android library"
        fi
    fi

    # Determine architecture from file output
    if file "$LIB_FILE" | grep -q "ARM aarch64"; then
        echo "Architecture: ARM 64-bit (arm64-v8a / aarch64)"
    elif file "$LIB_FILE" | grep -q "ARM"; then
        echo "Architecture: ARM 32-bit (armeabi-v7a)"
    elif file "$LIB_FILE" | grep -q "x86-64"; then
        echo "Architecture: x86_64 (64-bit Intel/AMD)"
    elif file "$LIB_FILE" | grep -q "Intel 80386"; then
        echo "Architecture: x86 (32-bit Intel)"
    fi

# Check if it's a PE file (Windows)
elif file "$LIB_FILE" | grep -q "PE32\|MS Windows"; then
    echo "Platform: Windows (PE)"

    if file "$LIB_FILE" | grep -q "PE32+"; then
        echo "Architecture: x86_64 (64-bit)"
    else
        echo "Architecture: x86 (32-bit)"
    fi

    if command -v objdump &> /dev/null; then
        echo ""
        echo "PE Header:"
        objdump -f "$LIB_FILE"
    fi

# Check if it's an archive file
elif file "$LIB_FILE" | grep -q "ar archive"; then
    echo "Type: Static library archive (.a)"

    # Extract and check first object file
    TEMP_DIR=$(mktemp -d)
    cd "$TEMP_DIR"
    ar x "$LIB_FILE" 2>/dev/null || true

    FIRST_OBJ=$(ls *.o 2>/dev/null | head -1)
    if [ -n "$FIRST_OBJ" ]; then
        echo ""
        echo "Checking first object file: $FIRST_OBJ"
        file "$FIRST_OBJ"

        if file "$FIRST_OBJ" | grep -q "ELF"; then
            if command -v readelf &> /dev/null; then
                readelf -h "$FIRST_OBJ" | grep -E "(Class|Machine|OS/ABI)"
            fi
        elif file "$FIRST_OBJ" | grep -q "Mach-O"; then
            if command -v otool &> /dev/null; then
                otool -hv "$FIRST_OBJ" | head -10
            fi
        fi
    fi

    cd - > /dev/null
    rm -rf "$TEMP_DIR"
else
    echo "Unknown format"
fi

# 3. Symbol table info (if available)
echo ""
echo "--- Symbol Table (first 10 symbols) ---"
if command -v nm &> /dev/null; then
    nm "$LIB_FILE" 2>/dev/null | head -10 || echo "No symbols available (stripped?)"
else
    echo "nm command not available"
fi

# 4. File size
echo ""
echo "--- File Size ---"
if command -v du &> /dev/null; then
    du -h "$LIB_FILE"
fi

echo ""
echo "=========================================="
