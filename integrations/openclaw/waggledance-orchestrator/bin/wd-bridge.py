#!/usr/bin/env python3
import sys
import json
import subprocess
import argparse

def call_mcp(tool_name, arguments):
    proc = subprocess.Popen(
        ["waggledance", "mcp"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
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

def main():
    parser = argparse.ArgumentParser(description="Waggledance CLI bridge for OpenClaw")
    subparsers = parser.add_subparsers(dest="command")

    # projects
    subparsers.add_parser("projects", help="List registered projects")

    # state
    state_parser = subparsers.add_parser("state", help="Ask state of projects")
    state_parser.add_argument("--project", "-p", help="Project ID")

    # search
    search_parser = subparsers.add_parser("search", help="Full-text search in projects")
    search_parser.add_argument("query", help="Search query")
    search_parser.add_argument("--project", "-p", help="Project ID")
    search_parser.add_argument("--limit", "-l", type=int, default=10, help="Max hits")

    # dispatch
    dispatch_parser = subparsers.add_parser("dispatch", help="Dispatch task to project")
    dispatch_parser.add_argument("--project", "-p", required=True, help="Target project ID")
    dispatch_parser.add_argument("--task", "-t", required=True, help="Task description / spec")
    dispatch_parser.add_argument("--preset", help="Agent preset label")
    dispatch_parser.add_argument("--pane-id", help="Existing pane ID")

    # runs
    runs_parser = subparsers.add_parser("runs", help="List dispatched runs")
    runs_parser.add_argument("--project", "-p", help="Filter by project ID")

    # await
    await_parser = subparsers.add_parser("await", help="Wait for a run")
    await_parser.add_argument("run_id", help="Run ID")
    await_parser.add_argument("--timeout", type=int, default=60, help="Timeout seconds (max 60)")

    # open
    open_parser = subparsers.add_parser("open", help="Get browser URL for a markdown file")
    open_parser.add_argument("path", help="Relative or absolute path to markdown file")
    open_parser.add_argument("--project-root", help="Project root path")

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
        # Direct CLI fallback for search to avoid daemon background re-indexing lockups over stdio
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
            # fallback to CLI
            subprocess.run(["waggledance", "open", args.path])
            return
        res = call_mcp("waggledance_view_file", arguments)
        print(json.dumps(res, indent=2, ensure_ascii=False))
    else:
        parser.print_help()

if __name__ == "__main__":
    main()
