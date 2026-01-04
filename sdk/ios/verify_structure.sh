#!/bin/bash
# Verify iOS SDK structure

echo "==================================="
echo "iOS SDK Structure Verification"
echo "==================================="
echo ""

# Check if required files exist
FILES=(
    "Package.swift"
    "Sources/HSIP/HSIPVerifier.swift"
    "Sources/HSIP/Types.swift"
    "Sources/HSIP/HSIP.swift"
    "README.md"
)

echo "Checking files..."
all_exist=true
for file in "${FILES[@]}"; do
    if [ -f "$file" ]; then
        echo "✅ $file"
    else
        echo "❌ $file - MISSING"
        all_exist=false
    fi
done
echo ""

# Check Package.swift structure
echo "Checking Package.swift..."
if grep -q "name: \"HSIP\"" Package.swift && \
   grep -q "platforms:" Package.swift && \
   grep -q ".iOS(.v13)" Package.swift; then
    echo "✅ Package.swift has correct structure"
else
    echo "❌ Package.swift missing required fields"
    all_exist=false
fi
echo ""

# Check if Swift files have basic structure
echo "Checking Swift files..."
if grep -q "class HSIPVerifier" Sources/HSIP/HSIPVerifier.swift && \
   grep -q "func verify" Sources/HSIP/HSIPVerifier.swift; then
    echo "✅ HSIPVerifier.swift has core verification logic"
else
    echo "❌ HSIPVerifier.swift missing required functions"
    all_exist=false
fi

if grep -q "struct SignedMessage" Sources/HSIP/Types.swift && \
   grep -q "struct VerificationResult" Sources/HSIP/Types.swift; then
    echo "✅ Types.swift has required types"
else
    echo "❌ Types.swift missing required types"
    all_exist=false
fi
echo ""

# Summary
if [ "$all_exist" = true ]; then
    echo "==================================="
    echo "✅ iOS SDK structure is VALID"
    echo "==================================="
    echo ""
    echo "Next steps:"
    echo "1. Test compilation: swift build"
    echo "2. Integrate into iOS app"
    exit 0
else
    echo "==================================="
    echo "❌ iOS SDK structure has ERRORS"
    echo "==================================="
    exit 1
fi
