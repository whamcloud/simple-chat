#!/usr/bin/env python3
from __future__ import annotations

"""
Smoke test design:
- Build server/client binaries exactly as CI does.
- Start one server and three interactive clients.
- Script client input over time (send/leave) to simulate real chat usage.
- Assert cross-client delivery by checking captured stdout logs.
- Always perform best-effort cleanup so failed runs do not leave processes behind.
"""

import os
import re
import shutil
import subprocess
import sys
import threading
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Optional


@dataclass
class Proc:
    """Tracks a spawned subprocess plus locations of captured logs."""

    name: str
    p: subprocess.Popen
    out_path: Path
    err_path: Path


def log(msg: str) -> None:
    """Emit a standardized smoke-test log line."""

    print(f"[smoke] {msg}", flush=True)


def run_checked(cmd: list[str], cwd: Path) -> None:
    """Run a command and fail immediately if it exits non-zero."""

    log(f"run: {' '.join(cmd)}")
    subprocess.run(cmd, cwd=str(cwd), check=True)


def start_process(
    name: str,
    cmd: list[str],
    cwd: Path,
    out_path: Path,
    err_path: Path,
    stdin_pipe: bool = False,
) -> Proc:
    """Start a subprocess with line-buffered stdout/stderr capture."""

    out_f = out_path.open("w", encoding="utf-8", buffering=1)
    err_f = err_path.open("w", encoding="utf-8", buffering=1)

    p = subprocess.Popen(
        cmd,
        cwd=str(cwd),
        stdin=subprocess.PIPE if stdin_pipe else None,
        stdout=out_f,
        stderr=err_f,
        text=True,
        bufsize=1,
    )
    return Proc(name=name, p=p, out_path=out_path, err_path=err_path)


def schedule_write(proc: Proc, delay_s: float, line: str) -> threading.Timer:
    """Schedule one stdin line write to a running process after a delay."""

    def _write() -> None:
        try:
            # Skip writes once process has exited or has no stdin pipe.
            if proc.p.poll() is not None:
                return
            if proc.p.stdin is None:
                return
            proc.p.stdin.write(line + "\n")
            proc.p.stdin.flush()
        except Exception:
            # Ignore write failures if process already exited
            pass

    t = threading.Timer(delay_s, _write)
    t.daemon = True
    t.start()
    return t


def terminate(proc: Proc, grace_s: float = 1.0) -> None:
    """Terminate a process gracefully, then force-kill if needed."""

    if proc.p.poll() is not None:
        return
    try:
        proc.p.terminate()
    except Exception:
        return
    try:
        proc.p.wait(timeout=grace_s)
    except subprocess.TimeoutExpired:
        try:
            proc.p.kill()
        except Exception:
            pass


def wait_proc(proc: Proc, timeout_s: float) -> int:
    """Wait for process exit and map timeout to GNU-timeout-like code 124."""

    try:
        return proc.p.wait(timeout=timeout_s)
    except subprocess.TimeoutExpired:
        return 124  # like GNU timeout


def expect_in_file(path: Path, pattern: str) -> None:
    """Assert regex presence in a file, printing helpful diagnostics on miss."""

    data = path.read_text(encoding="utf-8", errors="replace")
    if re.search(pattern, data) is None:
        print("ASSERTION FAILED: expected pattern not found", file=sys.stderr)
        print(f"  file: {path}", file=sys.stderr)
        print(f"  pattern: {pattern}", file=sys.stderr)
        print("---- file content (tail) ----", file=sys.stderr)
        tail = "\n".join(data.splitlines()[-200:])
        print(tail, file=sys.stderr)
        print("-----------------------------", file=sys.stderr)
        raise SystemExit(1)


def main() -> int:
    """Run the end-to-end smoke flow and return a process-style exit code."""

    repo_root = Path(__file__).resolve().parent.parent
    out_dir = repo_root / "smoke_test_output"
    out_dir.mkdir(parents=True, exist_ok=True)

    # Clean output dir
    for child in out_dir.iterdir():
        if child.is_file():
            child.unlink()

    listen_addr = os.environ.get("CHAT_LISTEN_ADDR", "127.0.0.1:9786")
    server_host = os.environ.get("CHAT_SERVER_HOST", "127.0.0.1")
    server_port = os.environ.get("CHAT_SERVER_PORT", "9786")

    msg_b2a = "bob-to-alice"
    msg_a2b = "alice-to-bob"

    # 1) Build (keep it aligned with your repo workflow)
    run_checked(["make", "build"], cwd=repo_root)

    server: Optional[Proc] = None
    clients: list[Proc] = []
    timers: list[threading.Timer] = []

    try:
        # 2) Start server
        log(f"starting server on {listen_addr}")
        server = start_process(
            "server",
            ["./bin/chat-server", "--listen", listen_addr],
            cwd=repo_root,
            out_path=out_dir / "server.log",
            err_path=out_dir / "server.err",
            stdin_pipe=False,
        )

        # Give server time to bind
        time.sleep(0.4)

        # 3) Start clients (stdin_pipe=True so we can feed commands)
        def start_client(user: str) -> Proc:
            # Keep CLI args explicit so test behavior is independent of env defaults.
            return start_process(
                user,
                [
                    "./bin/chat-client",
                    "--user",
                    user,
                    "--server-host",
                    server_host,
                    "--server-port",
                    server_port,
                ],
                cwd=repo_root,
                out_path=out_dir / f"{user}.out",
                err_path=out_dir / f"{user}.err",
                stdin_pipe=True,
            )

        log("starting clients: alice, bob, charlie")
        alice = start_client("alice")
        bob = start_client("bob")
        charlie = start_client("charlie")
        clients.extend([alice, bob, charlie])

        # Let them join / receive Joined ack
        time.sleep(0.6)

        # 4) Schedule sends + leaves (same intent as bash plan)
        timers.append(schedule_write(bob, 0.2, f"send {msg_b2a}"))
        timers.append(schedule_write(alice, 0.3, f"send {msg_a2b}"))

        timers.append(schedule_write(alice, 1.2, "leave"))
        timers.append(schedule_write(bob, 1.3, "leave"))
        timers.append(schedule_write(charlie, 1.6, "leave"))

        # 5) Wait for clients to exit (timeout safety)
        log("waiting clients to finish")
        for c in clients:
            # Per-client timeout keeps hangs localized and diagnosable.
            rc = wait_proc(c, timeout_s=12.0)
            if rc == 124:
                raise RuntimeError(f"{c.name} timed out")
            # If client returns non-zero, treat as failure.
            # (If your client intentionally returns non-zero on some paths, we can adjust.)
            if rc != 0:
                raise RuntimeError(f"{c.name} exited with rc={rc} (see {c.err_path})")

        # 6) Verify outputs
        log("verifying outputs")
        # Your client prints messages as: "[from] text\n"
        expect_in_file(out_dir / "alice.out", re.escape(f"[bob] {msg_b2a}"))
        expect_in_file(out_dir / "bob.out", re.escape(f"[alice] {msg_a2b}"))
        expect_in_file(out_dir / "charlie.out", re.escape(f"[bob] {msg_b2a}"))
        expect_in_file(out_dir / "charlie.out", re.escape(f"[alice] {msg_a2b}"))

        log("OK (alice<->bob; charlie receives both)")
        return 0

    except Exception as e:
        print(f"[smoke] FAILED: {e}", file=sys.stderr)
        return 1

    finally:
        # Cancel timers (best-effort)
        for t in timers:
            try:
                t.cancel()
            except Exception:
                pass

        # Close client stdin to encourage exit (best-effort)
        for c in clients:
            try:
                # Signals EOF to scanner loops waiting for input.
                if c.p.stdin:
                    c.p.stdin.close()
            except Exception:
                pass

        # Terminate clients if still running
        for c in clients:
            terminate(c, grace_s=1.0)

        # Terminate server
        if server is not None:
            terminate(server, grace_s=1.0)


if __name__ == "__main__":
    raise SystemExit(main())
