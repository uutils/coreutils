#!/bin/bash

echo "Testing embedded locale functionality to prevent crates.io regression..."
INSTALL_DIR="$(pwd)/test-install-dir"
rm -rf "$INSTALL_DIR"
mkdir -p "$INSTALL_DIR"
utilities_to_test=("cp" "mv" "ln")

for util in "${utilities_to_test[@]}"; do
  echo "Testing $util..."
  cargo install --path "src/uu/$util" --root "$INSTALL_DIR" --force --quiet
  binary_path="$INSTALL_DIR/bin/$util"
  help_output=$("$binary_path" --help 2>&1)

  # Check for regression indicators
  if echo "$help_output" | grep -q "common-usage"; then
    echo "✗ CRITICAL REGRESSION: $util shows untranslated 'common-usage' key"
    echo "Help output:"
    echo "$help_output"
    exit 1
  fi

  # Verify proper "Usage:" label
  if echo "$help_output" | grep -q "Usage:"; then
    echo "✓ $util shows properly translated 'Usage:' label"
  else
    echo "✗ CRITICAL REGRESSION: $util missing 'Usage:' label"
    echo "Help output:"
    echo "$help_output"
    exit 1
  fi
done

echo "All tests passed!"
