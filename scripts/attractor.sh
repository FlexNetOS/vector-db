#!/usr/bin/env bash
#
# Attractor pipeline runner for ruvector (the brain side).
#
# Reads .attractor/integration.dot and walks the canonical
# Identify -> Implement -> Validate -> Optimize -> Distill loop with no
# human in the loop. Each node delegates to .attractor/nodes/<name>.sh,
# which is allowed to be a stub during phase-by-phase rollout.
#
# Usage:
#   scripts/attractor.sh validate          # parse the DOT, report node order
#   scripts/attractor.sh dry-run           # print what each node would do
#   scripts/attractor.sh run                # execute one iteration end-to-end
#   scripts/attractor.sh node <identify|implement|validate|optimize|distill>
#                                          # invoke a single node directly
#                                          # (this is what the DOT's `command`
#                                          # attribute resolves to)
#   scripts/attractor.sh --help
#
# Exit codes:
#   0   success
#   1   user-error (bad args, missing DOT)
#   2   validate node failed (recorded as fail trajectory; optimize skipped,
#       distill still runs so ReasoningBank learns from the failure)
#   3   identify or implement failed (fail-fast: trajectory is broken before
#       it can be evaluated, so distill is skipped to avoid corrupting the
#       bank); OR optimize / distill failed (the trajectory ran end-to-end
#       but a post-validate stage errored; distill captures whatever it can)
#
# This script is intentionally dependency-light: it does not require
# graphviz unless `validate --strict` is requested. Node scripts may
# pull in cargo, jq, curl, etc. as needed.

set -euo pipefail

readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly DOT_FILE="$ROOT/.attractor/integration.dot"
readonly NODES_DIR="$ROOT/.attractor/nodes"
readonly RUNS_DIR="$ROOT/.attractor/runs"
readonly NODE_ORDER=(identify implement validate optimize distill)

# ---- Logging helpers ------------------------------------------------------

# Logs go to stderr (>&2), so detect a TTY on fd 2, not fd 1. Otherwise
# `attractor.sh run > out.json` would strip color from the still-visible
# stderr, and `attractor.sh run 2> err.log` would leak ANSI codes into a
# redirected log file.
if [ -t 2 ] && [ -z "${NO_COLOR:-}" ]; then
    BOLD=$'\e[1m'; DIM=$'\e[2m'; RED=$'\e[31m'; GREEN=$'\e[32m'
    YELLOW=$'\e[33m'; CYAN=$'\e[36m'; NC=$'\e[0m'
else
    BOLD=""; DIM=""; RED=""; GREEN=""; YELLOW=""; CYAN=""; NC=""
fi

log()    { printf "%s\n" "$*" >&2; }
info()   { printf "${CYAN}[attractor]${NC} %s\n" "$*" >&2; }
ok()     { printf "${GREEN}[attractor]${NC} %s\n" "$*" >&2; }
warn()   { printf "${YELLOW}[attractor]${NC} %s\n" "$*" >&2; }
err()    { printf "${RED}[attractor]${NC} %s\n" "$*" >&2; }

usage() {
    sed -n '2,/^$/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

# ---- Sanity ---------------------------------------------------------------

require_dot_file() {
    if [ ! -f "$DOT_FILE" ]; then
        err "missing pipeline graph: $DOT_FILE"
        exit 1
    fi
}

# Parse node order from the DOT. We trust the canonical NODE_ORDER for
# execution order (the DOT is the human-readable spec; a topological
# walk would also be valid but adds a graphviz dep).
list_nodes() {
    for node in "${NODE_ORDER[@]}"; do
        printf "%s\n" "$node"
    done
}

# ---- Subcommands ----------------------------------------------------------

cmd_validate() {
    require_dot_file
    info "validating $DOT_FILE"

    # Cheap syntactic check: ensure each node label is mentioned in the DOT.
    local missing=()
    for node in "${NODE_ORDER[@]}"; do
        # Use POSIX [[:space:]] instead of GNU `\s` so the validator works
        # on BSD/macOS grep, busybox, and other non-GNU runners.
        if ! grep -qE "^[[:space:]]*${node}[[:space:]]*\[" "$DOT_FILE"; then
            missing+=("$node")
        fi
    done
    if [ "${#missing[@]}" -gt 0 ]; then
        err "DOT is missing required nodes: ${missing[*]}"
        exit 2
    fi

    # Strict mode: also ask graphviz to parse it. Skipped if dot(1)
    # is not on PATH so this works on bare CI runners.
    if [ "${1:-}" = "--strict" ]; then
        if command -v dot >/dev/null 2>&1; then
            if ! dot -Tcanon "$DOT_FILE" >/dev/null 2>&1; then
                err "graphviz failed to parse $DOT_FILE"
                exit 2
            fi
            ok "graphviz parse OK"
        else
            warn "graphviz (dot) not installed; skipping --strict parse"
        fi
    fi

    ok "DOT contains all 5 canonical nodes: ${NODE_ORDER[*]}"
}

cmd_dry_run() {
    require_dot_file
    info "dry-run topology for $(basename "$ROOT")"
    local i=0
    for node in "${NODE_ORDER[@]}"; do
        i=$((i + 1))
        local script="$NODES_DIR/${node}.sh"
        if [ -x "$script" ]; then
            printf "  ${BOLD}%d. %s${NC}  -> %s\n" "$i" "$node" "$script" >&2
        else
            printf "  ${BOLD}%d. %s${NC}  -> ${DIM}(stub: %s missing or non-exec)${NC}\n" "$i" "$node" "$script" >&2
        fi
    done
    ok "5 nodes scheduled"
}

cmd_node() {
    require_dot_file
    local node="${1:-}"
    if [ -z "$node" ]; then
        err "usage: scripts/attractor.sh node <identify|implement|validate|optimize|distill>"
        exit 1
    fi
    local found=0
    for known in "${NODE_ORDER[@]}"; do
        if [ "$known" = "$node" ]; then found=1; break; fi
    done
    if [ "$found" -eq 0 ]; then
        err "unknown node: $node (expected one of: ${NODE_ORDER[*]})"
        exit 1
    fi
    local script="$NODES_DIR/${node}.sh"
    if [ ! -x "$script" ]; then
        err "missing or non-executable node script: $script"
        exit 1
    fi
    exec "$script"
}

# JSON-string-escape stdin → stdout. The python3 path handles every
# control character per RFC 8259 (json.dumps); the awk fallback covers
# the cases that actually show up in node contracts (\, ", \t, \r,
# embedded newlines) but NOT the rarer C0 controls like \b or \f.
# Practical impact: nil — node scripts emit JSON, not raw form-feeds.
#
# CRITICAL: pass `$0` as a printf *argument*, never as the format
# string. If a node emits `%s` or `%d`, putting it into the format
# string would crash awk ("not enough arguments") and `set -euo
# pipefail` would propagate the failure up, killing the run.
json_escape() {
    python3 -c 'import json,sys;sys.stdout.write(json.dumps(sys.stdin.read()))' 2>/dev/null \
        || awk 'BEGIN{printf "\""} {gsub(/\\/,"\\\\");gsub(/"/,"\\\"");gsub(/\t/,"\\t");gsub(/\r/,"\\r");printf "%s", (NR>1?"\\n":"")$0} END{printf "\""}'
}

cmd_run() {
    require_dot_file
    mkdir -p "$RUNS_DIR"
    local stamp; stamp="$(date -u +%Y%m%dT%H%M%SZ)"
    local log_file="$RUNS_DIR/${stamp}.jsonl"
    local stdout_dir="$RUNS_DIR/${stamp}-stdout"
    mkdir -p "$stdout_dir"

    info "executing pipeline; log -> $log_file"

    local i=0
    local overall_status=0
    local skip_optimize=0
    for node in "${NODE_ORDER[@]}"; do
        i=$((i + 1))
        local script="$NODES_DIR/${node}.sh"
        local started; started="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
        local status="ok"
        local rc=0
        local out_file="$stdout_dir/${node}.out"

        printf "${BOLD}[%d/5] %s${NC}\n" "$i" "$node" >&2

        # Skip optimize if a prior validate failed; per the DOT spec
        # validate red routes around optimize but still distills.
        if [ "$node" = "optimize" ] && [ "$skip_optimize" -eq 1 ]; then
            warn "validate failed earlier; skipping optimize per DOT spec"
            status="skipped"
            : > "$out_file"
        elif [ -x "$script" ]; then
            # Run the node. Capture *only stdout* (the JSON contract) to
            # $out_file via tee, while mirroring it to the operator's
            # stderr. The node's own stderr flows directly through to
            # the terminal and is NOT written to $out_file. Splitting
            # the streams (rather than `2>&1 | tee`) guarantees the LAST
            # non-empty line of $out_file is the node's JSON contract,
            # even if a future node emits late stderr diagnostics.
            #
            # Export ATTRACTOR_RUN_DIR so node scripts can drop sidecar
            # logs (e.g. validate.stderr) into the per-run stdout dir,
            # avoiding overwrites when two pipeline runs overlap.
            if ATTRACTOR_RUN_DIR="$stdout_dir" "$script" | tee "$out_file" >&2; then
                # `set -o pipefail` (set at the top of this file) means we only enter this
                # branch when every stage exits 0; PIPESTATUS[0] is
                # therefore always 0 here. The branch below handles
                # failures (and recovers PIPESTATUS to distinguish a
                # node failure from the rare case of tee failing).
                ok "$node passed"
            else
                # Two failure modes land here:
                #   * node script exited non-zero  -> PIPESTATUS[0] != 0
                #   * node passed but tee failed   -> PIPESTATUS[0] == 0,
                #     PIPESTATUS[1] != 0 (e.g. disk full)
                # The audit record below uses status="fail" in either
                # case, so the postmortem is honest even when rc==0.
                rc=${PIPESTATUS[0]:-1}
                if [ "$rc" -eq 0 ]; then
                    rc=${PIPESTATUS[1]:-1}
                    err "$node passed but tee failed (rc=$rc)"
                else
                    err "$node failed (rc=$rc)"
                fi
                status="fail"
            fi
        else
            warn "no node script at $script -- recording as 'stub'"
            status="stub"
            : > "$out_file"
        fi

        local finished; finished="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

        # Capture only the LAST non-empty line of stdout — node scripts
        # promise their final line is the JSON contract. The full stream
        # remains on disk in $stdout_dir for deeper postmortem.
        local last_line=""
        if [ -s "$out_file" ]; then
            last_line="$(awk 'NF{l=$0} END{print l}' "$out_file")"
        fi
        local output_json
        output_json="$(printf '%s' "$last_line" | json_escape)"
        if [ -z "$output_json" ]; then output_json='""'; fi

        printf '{"node":"%s","status":"%s","rc":%d,"started":"%s","finished":"%s","output_json":%s}\n' \
            "$node" "$status" "$rc" "$started" "$finished" "$output_json" >> "$log_file"

        # Validate is the contract: if it fails, route around optimize
        # but still distill the failed trajectory (per DOT spec).
        if [ "$node" = "validate" ] && [ "$status" = "fail" ]; then
            warn "validate failed; routing around optimize, still distilling for ReasoningBank"
            skip_optimize=1
            overall_status=2
            continue
        fi

        # identify or implement failing means the trajectory is broken
        # before it can be evaluated; fail-fast so we don't run validate
        # on garbage and corrupt the bank.
        if [ "$status" = "fail" ] && { [ "$node" = "identify" ] || [ "$node" = "implement" ]; }; then
            err "$node failed before validate; aborting trajectory"
            overall_status=3
            break
        fi
        if [ "$status" = "fail" ]; then
            overall_status=3
        fi
    done

    if [ "$overall_status" -eq 0 ]; then
        ok "pipeline run complete"
    fi
    exit "$overall_status"
}

# ---- Dispatch -------------------------------------------------------------

main() {
    local subcmd="${1:---help}"
    case "$subcmd" in
        validate)  shift; cmd_validate "$@" ;;
        dry-run)   shift; cmd_dry_run ;;
        run)       shift; cmd_run ;;
        node)      shift; cmd_node "$@" ;;
        -h|--help) usage ;;
        *)
            err "unknown subcommand: $subcmd"
            usage
            exit 1
            ;;
    esac
}

main "$@"
