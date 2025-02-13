#!/usr/bin/env -S deno run --allow-all

import OpenAI from "npm:openai";
import stripAnsi from "npm:strip-ansi";
import type { ChatCompletionCreateParams } from "npm:openai/resources/chat/completions";
import path from "node:path";
import tokenizer from "npm:gpt-tokenizer/model/gpt-4o";

const FILE_TAG = "__FILENAME__";
const START_TAG = "__FILE_CONTENT_START__";
const END_TAG = "__FILE_CONTENT_END__";
const ALL_GOOD_TAG = "DONE_ALL_TESTS_PASS_AND_COVERAGE_IS_GOOD";
const DEEPSEEK_REASONER_MAX_TOKENS = 65_536;
const OPENAI_MAX_TOKENS = 120_000;
const GEMINI_MAX_TOKENS = 1_000_000;

const BUILD_ERROR_PROMPT = `
You are given the full repository and build errors.
Your task is to fix the build errors.
Pay attention to DESIGN.md and USAGE.md files to understand the overall design and usage of the project.
Provide only and only code updates. Do not provide any other text. Your response can be multiple files.
DO NOT DELETE EXISTING IMPLEMENTATIONS. DO NOT DELETE EXISTING TESTS.
`.trim();

const CLIPPY_ERROR_PROMPT = `
You are given the full repository and clippy errors.
Your task is to fix the clippy errors while maintaining the existing functionality.
Pay attention to DESIGN.md and USAGE.md files to understand the overall design and usage of the project.
Provide only and only code updates. Do not provide any other text. Your response can be multiple files.
DO NOT DELETE EXISTING IMPLEMENTATIONS. DO NOT DELETE EXISTING TESTS.
`.trim();

const TEST_ERROR_PROMPT = `
You are given the full repository and test results.
Your task is to fix the failing tests.
Pick one test and try to fix that one failing test if multiple tests are failing.
DO NOT remove any existing implementations to make the tests pass.
Pay attention to DESIGN.md and USAGE.md files to understand the overall design and usage of the project.
Provide only and only code updates. Do not provide any other text. Your response can be multiple files.
DO NOT DELETE EXISTING IMPLEMENTATIONS. DO NOT DELETE EXISTING TESTS.
`.trim();

const COVERAGE_PROMPT = `
You are given the full repository and coverage report.
Your task is to add new tests to improve code coverage.
Code coverage should be 100%.
Pay attention to DESIGN.md and USAGE.md files to understand the overall design and usage of the project.
Provide only and only code updates. Do not provide any other text. Your response can be multiple files.
DO NOT DELETE EXISTING IMPLEMENTATIONS. DO NOT DELETE EXISTING TESTS.
Important: Add test files to the tests/ directory. Do not add tests in src/ files.
`.trim();

const RESPONSE_FORMAT = `
If all tests pass, and coverage is at 100%, return "${ALL_GOOD_TAG}".
When you return updated code, format your response as follows:
${FILE_TAG}
<relative/path/to/file>
${START_TAG}
<complete updated file content>
${END_TAG}
`;

(async function main() {
    const maxTokens = getMaxTokens();
    const { stdout: repo } = await runCommand(
        `yek --tokens ${maxTokens * 0.8}`,
    );

    // make sure coverage dir exists
    Deno.mkdirSync("coverage", { recursive: true });

    const buildResult = await runCommand("cargo build");
    if (buildResult.code !== 0) {
        console.log("Fixing build errors...");
        await fixBuildErrors(repo, buildResult);
        return;
    }

    const clippyResult = await runCommand("cargo clippy");
    if (clippyResult.code !== 0) {
        console.log("Fixing clippy errors...");
        await fixClippyErrors(repo, clippyResult);
        return;
    }

    const testResult = await runCommand("cargo nextest run");
    if (testResult.code !== 0) {
        console.log("Fixing test errors...");
        await fixTestErrors(repo, testResult);
        return;
    }

    // if custom prompt is provided, run it
    const customPrompt = Deno.env.get("AI_PROMPT");
    if (customPrompt) {
        console.log("Running custom prompt...");
        await runCustomPrompt(repo, customPrompt);
        return;
    }

    const coverageResult = await runCommand(
        "cargo llvm-cov test --ignore-run-fail",
    );
    const summary = await getChangesSummary(repo);
    console.log("Improving coverage...");

    await fixCoverage(repo, coverageResult, summary);
})();

// ------------------ Utils -------------------

function getMaxTokens() {
    const provider = Deno.env.get("AI_PROVIDER") || "ollama";
    switch (provider) {
        case "openai":
            return OPENAI_MAX_TOKENS;
        case "gemini":
            return GEMINI_MAX_TOKENS;
        case "deepseek":
            return DEEPSEEK_REASONER_MAX_TOKENS;
        default:
            return OPENAI_MAX_TOKENS;
    }
}

/**
 * Generate a prompt from a list of inputs. Trims from the top to fit the max tokens.
 * @param inputs Sorted from least to most important
 * @param maxTokens
 * @returns
 */
function generatePrompt(
    repo: string,
    inputs: [title: string, prompt: string][],
    maxTokens: number = getMaxTokens(),
) {
    const lines = [
        ["Repository", repo],
        ...inputs,
        ["Response format", RESPONSE_FORMAT],
    ]
        .flatMap(([title, prompt]) => [`# ${title}:`, prompt, "", "", ""])
        .map((line) => stripAnsi(line));

    while (tokenizer.encode(lines.join("\n")).length > maxTokens) {
        lines.shift();
    }

    return lines.join("\n");
}

async function runCustomPrompt(repo: string, prompt: string) {
    const request = generatePrompt(repo, [["Instructions", prompt]]);

    const aiContent = await callAi(request);
    await applyChanges(aiContent);
}

async function fixBuildErrors(
    repo: string,
    buildResult: { code: number; stderr: string },
) {
    const request = generatePrompt(repo, [
        ["Build errors", buildResult.stderr],
        ["Instructions", BUILD_ERROR_PROMPT],
    ]);

    const aiContent = await callAi(request);
    await applyChanges(aiContent);
}

async function fixClippyErrors(
    repo: string,
    clippyResult: { code: number; stderr: string },
) {
    const request = generatePrompt(repo, [
        ["Clippy errors", clippyResult.stderr],
        ["Instructions", CLIPPY_ERROR_PROMPT],
    ]);

    const aiContent = await callAi(request);
    await applyChanges(aiContent);
}

async function fixTestErrors(
    repo: string,
    testResult: { code: number; stderr: string },
) {
    const request = generatePrompt(repo, [
        ["Test errors", testResult.stderr],
        ["Instructions", TEST_ERROR_PROMPT],
    ]);

    const aiContent = await callAi(request);
    await applyChanges(aiContent);
}

async function fixCoverage(
    repo: string,
    coverageResult: { code: number; stderr: string },
    summary: string,
) {
    const request = generatePrompt(repo, [
        ["Coverage report", coverageResult.stderr],
        ["Changes summary", summary],
        ["Instructions", COVERAGE_PROMPT],
    ]);

    const aiContent = await callAi(request);
    await applyChanges(aiContent);
}

async function applyChanges(aiContent: string) {
    const updatedFiles = parseUpdatedFiles(aiContent);
    for (const f of updatedFiles) {
        await writeFileContent(f.filename, f.content);
    }
    await runCommand("cargo fmt");
    await runCommand("cargo clippy --fix --allow-dirty");
}

function getOpenAiClient() {
    const provider = Deno.env.get("AI_PROVIDER") || "ollama";

    console.log("Using AI provider:", provider);

    switch (provider) {
        case "ollama": {
            const apiKey = "";
            return new OpenAI({
                apiKey,
                baseURL: "http://127.0.0.1:11434/v1",
            });
        }
        case "gemini": {
            const apiKey = Deno.env.get("GEMINI_API_KEY");
            if (!apiKey) throw new Error("Missing GEMINI_API_KEY env var.");
            return new OpenAI({
                apiKey,
                baseURL:
                    "https://generativelanguage.googleapis.com/v1beta/openai/",
            });
        }
        case "openai": {
            const apiKey = Deno.env.get("OPENAI_API_KEY");
            if (!apiKey) throw new Error("Missing OPENAI_API_KEY env var.");
            return new OpenAI({ apiKey });
        }
        case "deepseek": {
            const apiKey = Deno.env.get("DEEPSEEK_API_KEY");
            if (!apiKey) throw new Error("Missing DEEPSEEK_API_KEY env var.");
            return new OpenAI({
                apiKey,
                baseURL: "https://api.deepseek.com/v1",
            });
        }
        default: {
            throw new Error(`Unknown AI provider: ${provider}`);
        }
    }
}

async function writeFileContent(filePath: string, content: string) {
    console.log("Writing updated content to:", filePath);
    // make sure directories exists first
    const dir = path.dirname(filePath);
    Deno.mkdirSync(dir, { recursive: true });
    await Deno.writeTextFile(filePath, content);
}

async function runCommand(
    command: string,
): Promise<{ code: number; stdout: string; stderr: string }> {
    const [cmd, ...args] = command.split(/\s+/);
    console.log(`$ ${command}`);
    const proc = new Deno.Command(cmd, {
        args,
        stdout: "piped",
        stderr: "piped",
        stdin: "inherit",
        env: {
            ...Deno.env.toObject(),
            RUSTFLAGS: "-Cinstrument-coverage",
            LLVM_PROFILE_FILE: "coverage/merged-%p-%m.profraw",
            CARGO_TERM_COLOR: "always",
            RUST_BACKTRACE: "1",
            FORCE_COLOR: "1",
        },
    });
    const output = await proc.output();
    return {
        code: output.code,
        stdout: new TextDecoder().decode(output.stdout),
        stderr: new TextDecoder().decode(output.stderr),
    };
}

async function callAi(
    text: string,
    { printOutput = true }: { printOutput?: boolean } = {},
) {
    const openai = getOpenAiClient();
    const modelName = Deno.env.get("AI_MODEL") || "mistral-small";
    const encoder = new TextEncoder();
    const chatParams: ChatCompletionCreateParams = {
        model: modelName,
        stream: true,
        messages: [{ role: "user", content: text }],
    };

    const res = await openai.chat.completions.create(chatParams);
    const contents = [];
    for await (const chunk of res) {
        const content = chunk.choices[0].delta.content ?? "";
        contents.push(content);
        if (printOutput) {
            Deno.stdout.writeSync(encoder.encode(content));
        }
    }

    return contents.join("");
}

async function getChangesSummary(repo: string) {
    const baseBranch = Deno.env.get("BASE_BRANCH") || "main";
    const { stdout: changes } = await runCommand(`git diff ${baseBranch}`);
    if (!changes) return "No changes";
    console.log("Asking AI to summarize changes...");
    const summaryAndThinking = await callAi(
        [
            `Repository:`,
            repo,
            `Changes:`,
            changes,
            `Instructions: Summerize the changes made so far in the repo. In bullet points. Short and concise.`,
        ].join("\n"),
    );

    // remove the thinking part
    const summary = summaryAndThinking.replace(/<think>\n.*?\n<\/think>/s, "");
    return summary;
}

function parseUpdatedFiles(
    content: string,
): Array<{ filename: string; content: string }> {
    // Quick and simple parse for multiple updates in one message

    const results: Array<{ filename: string; content: string }> = [];

    if (!content.includes(FILE_TAG)) return results;

    // Split on the special file tag
    const chunks = content.split(FILE_TAG);
    for (const chunk of chunks) {
        const trimmedChunk = chunk.trim();
        if (!trimmedChunk) continue;

        if (!trimmedChunk.includes(START_TAG)) continue;
        if (!trimmedChunk.includes(END_TAG)) continue;

        const lines = trimmedChunk.split("\n");
        const filename = lines[0].trim();
        const rest = lines.slice(1).join("\n").trim();

        const startIdx = rest.indexOf(START_TAG);
        const endIdx = rest.indexOf(END_TAG);
        if (startIdx < 0 || endIdx < 0) continue;

        let fileContent = rest
            .substring(startIdx + START_TAG.length, endIdx)
            .trim();
        // Remove any triple backticks
        if (fileContent.startsWith("```")) {
            fileContent = fileContent.replace(/^```[^\n]*\n?/, "");
        }
        if (fileContent.endsWith("```")) {
            fileContent = fileContent.replace(/```$/, "");
        }

        results.push({ filename, content: fileContent.trim() });
    }
    return results;
}
