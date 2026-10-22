#!/usr/bin/env python3
import sys
import json
import subprocess
import argparse
import os
import secrets
import shlex

def call_mcp(tool_name, arguments, extra_env=None):
    env = child_env()
    if extra_env:
        env.update(extra_env)
    proc = subprocess.Popen(
        ["waggledance", "mcp"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env
    )
    req = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": arguments
        }
    }
    stdout, stderr = proc.communicate(input=json.dumps(req) + "\n", timeout=120)
    for line in stdout.splitlines():
        try:
            res = json.loads(line)
            if "result" in res:
                return res["result"]
            elif "error" in res:
                return {"error": res["error"]}
        except Exception:
            continue
    return {"error": f"Failed to get valid json-rpc response. Stderr: {stderr}"}

def child_env():
    env = dict(os.environ)
    env.pop("PASEO_AGENT_ID", None)
    env.pop("PASEO_AGENT_CWD", None)
    return env

def fail(message):
    print(f"error: {message}", file=sys.stderr)
    sys.exit(1)

WD_ENV = {"WAGGLEDANCE_CALLER": "wd"}


def wd_call(tool, arguments, dry_run):
    if dry_run:
        print(shlex.join(["WAGGLEDANCE_CALLER=wd", "waggledance", "mcp"]))
        print(json.dumps({"tool": tool, "arguments": arguments}, ensure_ascii=False))
        return None
    res = call_mcp(tool, arguments, WD_ENV)
    if "error" in res:
        fail(f"{tool} failed: {json.dumps(res['error'], ensure_ascii=False)[:300]}")
    if res.get("isError"):
        fail(f"{tool} failed: {res_text(res)}")
    return res


def hand_off(project, task, corr, dry_run):
    res = wd_call(
        "waggledance_dispatch",
        {"project": project, "task": task, "leader": True, "request_id": corr},
        dry_run,
    )
    if res is None:
        return
    structured = res.get("structuredContent") or {}
    run_id = structured.get("run_id")
    if not run_id:
        fail(f"waggledance dispatch printed no run_id: {res_text(res)[:300]}")
    print(json.dumps({
        "corr": corr,
        "run_id": run_id,
        "project": project,
        "status": structured.get("status"),
        "duplicate": structured.get("duplicate"),
        "warnings": structured.get("warnings", []),
    }, ensure_ascii=False))


def res_text(res):
    return " ".join(
        part.get("text", "") for part in res.get("content", []) if isinstance(part, dict)
    )


def follow_up(corr, action, dry_run, text=None, req_id=None):
    arguments = {"corr": corr, "action": action}
    if text is not None:
        arguments["text"] = text
    if req_id:
        arguments["req_id"] = req_id
    res = wd_call("waggledance_task", arguments, dry_run)
    if res is None:
        return
    print(json.dumps({
        "corr": corr,
        "action": action,
        "text": res_text(res),
        **(res.get("structuredContent") or {}),
    }, ensure_ascii=False))

def task_text(args):
    if args.task_file:
        try:
            with open(args.task_file, encoding="utf-8") as f:
                return f.read().strip()
        except OSError as e:
            fail(f"cannot read --task-file {args.task_file}: {e}")
    return args.task

def main():
    parser = argparse.ArgumentParser(description="Waggledance CLI bridge for OpenClaw")
    subparsers = parser.add_subparsers(dest="command")

    subparsers.add_parser("projects", help="List registered projects")

    state_parser = subparsers.add_parser("state", help="Ask state of projects")
    state_parser.add_argument("--project", "-p", help="Project ID")

    search_parser = subparsers.add_parser("search", help="Full-text search in projects")
    search_parser.add_argument("query", help="Search query")
    search_parser.add_argument("--project", "-p", help="Project ID")
    search_parser.add_argument("--limit", "-l", type=int, default=10, help="Max hits")

    dispatch_parser = subparsers.add_parser("dispatch", help="Dispatch task to project")
    dispatch_parser.add_argument("--project", "-p", required=True, help="Target project ID")
    dispatch_parser.add_argument("--task", "-t", required=True, help="Task description / spec")
    dispatch_parser.add_argument("--preset", help="Agent preset label")
    dispatch_parser.add_argument("--pane-id", help="Existing pane ID")

    runs_parser = subparsers.add_parser("runs", help="List dispatched runs")
    runs_parser.add_argument("--project", "-p", help="Filter by project ID")

    await_parser = subparsers.add_parser("await", help="Wait for a run")
    await_parser.add_argument("run_id", help="Run ID")
    await_parser.add_argument("--timeout", type=int, default=60, help="Timeout seconds (max 60)")

    open_parser = subparsers.add_parser("open", help="Get browser URL for a markdown file")
    open_parser.add_argument("path", help="Relative or absolute path to markdown file")
    open_parser.add_argument("--project-root", help="Project root path")

    handoff_parser = subparsers.add_parser("handoff", help="Hand a task to a project's leader")
    handoff_parser.add_argument("--project", "-p", required=True, help="Target project ID")
    source = handoff_parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--task", help="Task text")
    source.add_argument("--task-file", help="File holding the task text")
    handoff_parser.add_argument("--corr", help="Correlation id (minted as t-<6 hex> when absent)")
    handoff_parser.add_argument("--dry-run", action="store_true", help="Print the commands without running them")

    for name, help_text in (("reply", "Relay the user's answer"), ("redo", "Ask for the work again")):
        relay_parser = subparsers.add_parser(name, help=help_text)
        relay_parser.add_argument("--corr", required=True, help="Correlation id")
        relay_parser.add_argument("--text", required=True, help="The user's words")
        relay_parser.add_argument("--dry-run", action="store_true", help="Print the commands without running them")

    accept_parser = subparsers.add_parser("accept", help="Relay the user's acceptance")
    accept_parser.add_argument("--corr", required=True, help="Correlation id")
    accept_parser.add_argument("--dry-run", action="store_true", help="Print the commands without running them")

    permit_parser = subparsers.add_parser("permit", help="Relay the user's allow or deny of a permission request")
    permit_parser.add_argument("answer", choices=["allow", "deny"], help="The user's answer")
    permit_parser.add_argument("corr", help="Correlation id")
    permit_parser.add_argument("req_id", nargs="?", help="Request id (the newest unanswered one when absent)")
    permit_parser.add_argument("--dry-run", action="store_true", help="Print the commands without running them")

    args = parser.parse_args()

    if args.command == "projects":
        res = call_mcp("waggledance_projects", {})
        print(json.dumps(res, indent=2, ensure_ascii=False))
    elif args.command == "state":
        arguments = {}
        if args.project:
            arguments["project"] = args.project
        res = call_mcp("waggledance_ask_state", arguments)
        print(json.dumps(res, indent=2, ensure_ascii=False))
    elif args.command == "search":
        cmd = ["waggledance", "search", args.query]
        subprocess.run(cmd)
        return
    elif args.command == "dispatch":
        arguments = {"project": args.project, "task": args.task}
        if args.preset:
            arguments["preset"] = args.preset
        if args.pane_id:
            arguments["pane_id"] = args.pane_id
        res = call_mcp("waggledance_dispatch", arguments)
        print(json.dumps(res, indent=2, ensure_ascii=False))
    elif args.command == "runs":
        arguments = {}
        if args.project:
            arguments["project"] = args.project
        res = call_mcp("waggledance_runs", arguments)
        print(json.dumps(res, indent=2, ensure_ascii=False))
    elif args.command == "await":
        arguments = {"run_id": args.run_id, "timeout_seconds": args.timeout}
        res = call_mcp("waggledance_await", arguments)
        print(json.dumps(res, indent=2, ensure_ascii=False))
    elif args.command == "open":
        arguments = {"relative_path": args.path}
        if args.project_root:
            arguments["project_root"] = args.project_root
        else:
            subprocess.run(["waggledance", "open", args.path])
            return
        res = call_mcp("waggledance_view_file", arguments)
        print(json.dumps(res, indent=2, ensure_ascii=False))
    elif args.command == "handoff":
        corr = args.corr or f"t-{secrets.token_hex(3)}"
        hand_off(args.project, task_text(args), corr, args.dry_run)
    elif args.command in ("reply", "redo"):
        follow_up(args.corr, args.command, args.dry_run, text=args.text)
    elif args.command == "accept":
        follow_up(args.corr, "accept", args.dry_run)
    elif args.command == "permit":
        follow_up(args.corr, args.answer, args.dry_run, req_id=args.req_id)
    else:
        parser.print_help()

if __name__ == "__main__":
    main()
