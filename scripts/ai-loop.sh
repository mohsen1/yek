#!/bin/bash
# Only in CI
if [ "$GITHUB_ACTIONS" ]; then
    git config --global user.email "github-actions[bot]@users.noreply.github.com"
    git config --global user.name "github-actions[bot]"
    echo "SHORT_DATE=$(date +%Y%m%d_%H%M)" >>$GITHUB_ENV
fi

# Default to 40 attempts if not set
attempts=${MAX_ATTEMPTS:-40}
BRANCH=${BRANCH:-tokenizer}

# Get stack trace for debugging
export RUST_BACKTRACE=1

success=0

for i in $(seq 1 $attempts); do
    echo "=== Attempt $i/$attempts ==="

    # Run tests and print output to console
    test_output=$(cargo test --test-threads=1 2>&1)
    test_exit_code=$?
    echo "$test_output" >test_output.txt

    # Process failures if tests failed
    if [ $test_exit_code -ne 0 ]; then
        # Extract the relevant error information
        grep -A 300 "failures:" test_output.txt >failures.txt
        if [ -s failures.txt ]; then
            mv failures.txt test_output.txt
        fi
    fi

    # Exit loop if tests passed
    if [ $test_exit_code -eq 0 ]; then
        success=1
        if [ "$GITHUB_ACTIONS" ]; then
            echo "ATTEMPTS=$i" >>$GITHUB_ENV
        fi
        echo "Tests passed!!"
        break
    fi

    # Chop test_output.txt to 100KB (from bottom) to avoid Context Window overflow
    tail -c 100KB test_output.txt >test_output.tmp && mv test_output.tmp test_output.txt

    echo "Running askds to fix the tests"
    echo "test_output.txt: size $(du -sh test_output.txt | awk '{print $1}')"

    # Run askds to fix the tests
    node ../askds/dist/index.js \
        --hide-ui \
        --timeout=480 \
        --fix \
        --auto-apply \
        --serialize="yek --max-size=100KB | cat" \
        --test-file-pattern='tests/*.rs' \
        --source-file-pattern='src/**/*.rs' \
        --system-prompt=./prompts/fix-tests.txt \
        --run="cat test_output.txt" || true

    # Run clippy and fmt to fix the code style
    cargo clippy --all-targets --fix --allow-dirty || true
    cargo fmt || true

    # Commit changes if any
    if ! git diff --quiet; then
        git add .
        git commit -m "fix attempt $i (${BRANCH})"
        echo "Applied fixes for ${BRANCH} tests"
        rm -f last_attempt.txt
    else
        echo "No changes in attempt $i"
        cp test_output.txt last_attempt.txt
        continue
    fi
done

rm -f last_attempt.txt

if [ $success -ne 1 ]; then
    if [ "$GITHUB_ACTIONS" ]; then
        echo "ATTEMPTS=$attempts" >>$GITHUB_ENV
        echo "::error::Failed after $attempts attempts"
        exit 1
    else
        echo "Failed after $attempts attempts"
        exit 1
    fi
fi

# Ensure formatting and linting passes before proceeding
if ! cargo fmt --check; then
    echo "Error: Code formatting check failed. Run 'cargo fmt' to fix formatting issues."
    exit 1
fi
if ! cargo clippy --all-targets -- -D warnings; then
    echo "Error: Clippy lints found. Fix the reported issues before continuing."
    exit 1
fi

cargo fmt || exit 1
cargo clippy --fix --allow-dirty --allow-staged || exit 1
