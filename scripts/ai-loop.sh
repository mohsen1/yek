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

success=0

for i in $(seq 1 $attempts); do
    echo "=== Attempt $i/$attempts ==="

    # Run tests and print output to console
    test_output=$(cargo test -- --test accepts_model_from_config --test-threads=1 2>&1)
    test_exit_code=$?
    echo "$test_output" >test_output.tmp

    # Append last attempt to test output if it exists
    if [ -f last_attempt.txt ]; then
        echo "## Last time we tried this but we failed:"
        cat last_attempt.txt >>test_output.tmp
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

    # Run askds to fix the tests
    askds \
        --hide-ui \
        --fix \
        --auto-apply \
        --serialize="yek --max-size=100KB | cat" \
        --test-file-pattern='tests/*.rs' \
        --source-file-pattern='src/**/*.rs' \
        --system-prompt=./prompts/fix-tests.txt \
        --run="cat test_output.tmp" || true

    rm last_attempt.txt
    cargo fmt
    cargo clippy --fix --allow-dirty

    # Commit changes if any
    if ! git diff --quiet; then
        git add .
        git commit -m "fix attempt $i (${BRANCH})"
        echo "Applied fixes for ${BRANCH} tests"
    else
        echo "No changes in attempt $i"
        cp test_output.tmp last_attempt.txt
        continue
    fi
done

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
