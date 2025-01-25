# Only in CI
if [ "$GITHUB_ACTIONS" ]; then
    git config --global user.email "github-actions[bot]@users.noreply.github.com"
    git config --global user.name "github-actions[bot]"
    echo "SHORT_DATE=$(date +%Y%m%d)" >>$GITHUB_ENV
fi

attempts=$MAX_ATTEMPTS

success=0

for i in $(seq 1 $attempts); do
    echo "=== Attempt $i/$attempts ===" | tee -a attempts.txt

    # Capture both stdout and stderr
    test_output=$(cargo test -- --test-threads=1 2>&1)
    test_exit_code=${PIPESTATUS[0]}

    # Trim output from "failures:"
    test_output=$(echo "$test_output" | sed -n '/failures:/,/failures:/p' | sed '1d; $d')

    # Save full test output
    echo "$test_output" >>attempts.txt
    echo -e "\n\n" >>attempts.txt

    # Exit if tests pass
    if [ $test_exit_code -eq 0 ]; then
        success=1
        if [ "$GITHUB_ACTIONS" ]; then
            echo "ATTEMPTS=$i" >>$GITHUB_ENV
        fi
        break
    fi

    # Feed DeepSeek with history but only the last 250KB
    askds --serialize="yek --max-size=100KB | cat" --test-file-pattern="tests/*.rs" --fix --auto-apply --system-prompt=./prompts/fix-tests.txt "$(tail -c 250000 attempts.txt)"

    # Check for changes
    if ! git diff --quiet; then
        git add .
        git commit -m "fix attempt $i (tokenizer)"
        echo "Applied fixes for tokenizer tests" | tee -a attempts.txt
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
