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
    echo "=== Attempt $i/$attempts ===" | tee -a attempts.txt

    # Run tests and print output to console, capture to temp file
    cargo test -- --test-threads=1 2>&1 | tee test_output.tmp
    test_exit_code=${PIPESTATUS[0]}

    # Trim output to only include failures section
    test_output=$(sed -n '/failures:/,/failures:/p' test_output.tmp | sed '1d; $d')
    rm test_output.tmp

    # Append trimmed test results to attempts.txt
    echo "$test_output" >>attempts.txt
    echo -e "\n\n" >>attempts.txt

    # Exit loop if tests passed
    if [ $test_exit_code -eq 0 ]; then
        success=1
        if [ "$GITHUB_ACTIONS" ]; then
            echo "ATTEMPTS=$i" >>$GITHUB_ENV
        fi
        break
    fi

    # Run askds and print output to console and log
    echo "=== askds Output ===" | tee -a attempts.txt
    askds --serialize="yek --max-size=100KB | cat" --test-file-pattern="tests/*.rs" --fix --auto-apply --system-prompt=./prompts/fix-tests.txt "$(tail -c 250000 attempts.txt)" 2>&1 | tee -a attempts.txt
    echo "=== End askds Output ===" | tee -a attempts.txt

    # Commit changes if any
    if ! git diff --quiet; then
        git add .
        git commit -m "fix attempt $i (${BRANCH})"
        echo "Applied fixes for ${BRANCH} tests" | tee -a attempts.txt
    else
        echo "No changes in attempt $i" | tee -a attempts.txt
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
