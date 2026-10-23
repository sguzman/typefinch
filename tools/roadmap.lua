#!/usr/bin/env lua5.4
-- Reproducible roadmap renderer and evidence auditor. No Python in CI.
local json = require("dkjson")
local function read(path)
    local file, err = io.open(path, "rb")
    assert(file, err)
    local content = assert(file:read("*a"))
    file:close()
    return content
end
local function write(path, content)
    local file, err = io.open(path, "wb")
    assert(file, err)
    assert(file:write(content))
    assert(file:close())
end
local ledger = assert(json.decode(read("docs/progress.json")))
local tasks, lookup, ids = {}, {}, {}
local function round(n) return math.floor(n + 0.5) end
local function mean(items)
    if #items == 0 then return 0 end
    local sum = 0
    for _, item in ipairs(items) do sum = sum + item.progress end
    return round(sum / #items)
end
local function bar(n)
    local blocks = math.min(10, round(n / 10))
    return string.rep("█", blocks) .. string.rep("░", 10 - blocks)
end
for _, milestone in ipairs(ledger.milestones) do
    for _, phase in ipairs(milestone.phases) do
        for _, task in ipairs(phase.tasks) do
            assert(not ids[task.id], "duplicate task ID " .. task.id)
            ids[task.id] = true
            lookup[task.id] = task
            table.insert(tasks, task)
            assert(#task.checks > 0, "missing acceptance criteria " .. task.id)
            local accepted = 0
            for _, check in ipairs(task.checks) do
                assert(type(check.passed) == "boolean", "nonboolean criterion in " .. task.id)
                if check.passed then
                    accepted = accepted + 1
                    assert(type(check.evidence) == "string" and #check.evidence > 0,
                        "missing evidence in " .. task.id)
                end
            end
            assert(task.progress == round(100 * accepted / #task.checks),
                "stale task percentage " .. task.id)
        end
    end
end
local lines = {
    "# 🗺️ Typefinch roadmap", "",
    "**Source of truth:** [progress.json](progress.json). Percentages are evidence-backed fractions of atomic acceptance checks, averaged into phase/milestone totals. No unverified task reaches 100%.",
    "", string.format("**Overall:** `%s %d%%` across %d tasks. This is tracked progress, not a claim of runtime readiness.", bar(mean(tasks)), mean(tasks), #tasks), ""
}
for _, milestone in ipairs(ledger.milestones) do
    local mtasks = {}
    for _, phase in ipairs(milestone.phases) do
        for _, task in ipairs(phase.tasks) do table.insert(mtasks, task) end
    end
    local amount = mean(mtasks)
    table.insert(lines, string.format("## %s %s — %s — `%s %d%%`",
        milestone.emoji, milestone.id, milestone.name, bar(amount), amount))
    table.insert(lines, "")
    table.insert(lines, "**Acceptance gate:** " .. milestone.gate)
    table.insert(lines, "")
    for _, phase in ipairs(milestone.phases) do
        amount = mean(phase.tasks)
        table.insert(lines, string.format("### %s %s — %s — `%s %d%%`",
            phase.emoji, phase.id, phase.name, bar(amount), amount))
        table.insert(lines, "")
        for _, task in ipairs(phase.tasks) do
            table.insert(lines, string.format("- %s **%s** %s — `%s %d%%`",
                task.emoji, task.id, task.name, bar(task.progress), task.progress))
            for _, check in ipairs(task.checks) do
                local mark = check.passed and "x" or " "
                local extra = check.passed and (" — " .. check.evidence) or ""
                table.insert(lines, string.format("  - [%s] %s%s", mark, check.text, extra))
            end
        end
        table.insert(lines, "")
    end
    table.insert(lines, "**Feature inventory** (computed from associated task progress):")
    table.insert(lines, "")
    for _, feature in ipairs(milestone.features) do
        local ftasks = {}
        for _, id in ipairs(feature.task_ids) do
            assert(lookup[id], "unrecognized feature task " .. id)
            table.insert(ftasks, lookup[id])
        end
        amount = mean(ftasks)
        table.insert(lines, string.format("- %s %s — `%s %d%%`",
            feature.emoji, feature.name, bar(amount), amount))
    end
    table.insert(lines, "")
end
local content = table.concat(lines, "\n") .. "\n"
if arg[1] == "--check" then
    assert(read("docs/ROADMAP.md") == content,
        "ROADMAP.md does not match ledger; run lua5.4 tools/roadmap.lua")
    print(string.format("Verified %d tasks, evidence and generated roadmap (%d%%)", #tasks, mean(tasks)))
else
    write("docs/ROADMAP.md", content)
    print(string.format("Rendered Typefinch roadmap: %d tasks, %d%%", #tasks, mean(tasks)))
end
