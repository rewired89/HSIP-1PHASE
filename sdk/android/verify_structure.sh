#!/bin/bash
# Verify Android SDK structure

echo "==================================="
echo "Android SDK Structure Verification"
echo "==================================="
echo ""

cd hsip

# Check if required files exist
FILES=(
    "build.gradle"
    "src/main/AndroidManifest.xml"
    "src/main/java/com/hsip/HSIPVerifier.kt"
    "src/main/java/com/hsip/Types.kt"
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

# Check build.gradle structure
echo "Checking build.gradle..."
if grep -q "kotlin" build.gradle && \
   grep -q "org.bouncycastle" build.gradle; then
    echo "✅ build.gradle has correct dependencies"
else
    echo "❌ build.gradle missing required dependencies"
    all_exist=false
fi
echo ""

# Check if Kotlin files have basic structure
echo "Checking Kotlin files..."
if grep -q "class HSIPVerifier" src/main/java/com/hsip/HSIPVerifier.kt && \
   grep -q "fun verify" src/main/java/com/hsip/HSIPVerifier.kt; then
    echo "✅ HSIPVerifier.kt has core verification logic"
else
    echo "❌ HSIPVerifier.kt missing required functions"
    all_exist=false
fi

if grep -q "data class SignedMessage" src/main/java/com/hsip/Types.kt && \
   grep -q "data class VerificationResult" src/main/java/com/hsip/Types.kt; then
    echo "✅ Types.kt has required types"
else
    echo "❌ Types.kt missing required types"
    all_exist=false
fi
echo ""

# Summary
if [ "$all_exist" = true ]; then
    echo "==================================="
    echo "✅ Android SDK structure is VALID"
    echo "==================================="
    echo ""
    echo "Next steps:"
    echo "1. Test compilation: ./gradlew build"
    echo "2. Integrate into Android app"
    exit 0
else
    echo "==================================="
    echo "❌ Android SDK structure has ERRORS"
    echo "==================================="
    exit 1
fi
