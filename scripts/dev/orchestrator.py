#!/usr/bin/env python3
"""
Development Environment Orchestrator

Manages client/server processes with proper lifecycle management,
graceful shutdown, and clean output formatting.
"""

import subprocess
import sys
import os
import signal
import time
import atexit
from pathlib import Path
from typing import List, Optional, Dict
import threading

# Configure UTF-8 output for Windows
if sys.platform == "win32":
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8', errors='replace')



class ProcessManager:
    """Manages multiple processes with graceful shutdown."""

    def __init__(self):
        self.processes: Dict[str, subprocess.Popen] = {}
        self.running = True
        self.lock = threading.Lock()

        # Register cleanup handlers
        atexit.register(self.cleanup)
        signal.signal(signal.SIGINT, self._signal_handler)
        signal.signal(signal.SIGTERM, self._signal_handler)

    def _signal_handler(self, sig, frame):
        """Handle shutdown signals gracefully."""
        print("\n\n[STOPPING] Shutdown signal received. Stopping all processes...")
        self.running = False
        self.cleanup()
        sys.exit(0)

    def start_process(
        self,
        name: str,
        cmd: List[str],
        env: Optional[Dict[str, str]] = None,
        cwd: Optional[str] = None,
    ):
        """Start a process and track it."""
        with self.lock:
            if name in self.processes:
                print(f"[WARNING]  Process '{name}' already running")
                return

            print(f"[STARTING] Starting {name}...")

            process_env = os.environ.copy()
            if env:
                process_env.update(env)

            try:
                proc = subprocess.Popen(
                    cmd,
                    env=process_env,
                    cwd=cwd,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    bufsize=1,
                    universal_newlines=True,
                )
                self.processes[name] = proc
                print(f"[OK] {name} started (PID: {proc.pid})")

                # Start output reader thread
                thread = threading.Thread(
                    target=self._read_output,
                    args=(name, proc),
                    daemon=True,
                )
                thread.start()

            except Exception as e:
                print(f"[ERROR] Failed to start {name}: {e}")
                sys.exit(1)

    def _read_output(self, name: str, proc: subprocess.Popen):
        """Read process output in a thread."""
        # Color codes
        colors = {
            "server": "\033[94m",  # Blue
            "client": "\033[92m",  # Green
            "profiler": "\033[95m",  # Magenta
            "metrics": "\033[96m",  # Cyan
        }
        reset = "\033[0m"
        color = colors.get(name, "")

        try:
            for line in proc.stdout:
                if not self.running:
                    break
                print(f"{color}[{name}]{reset} {line.rstrip()}")
        except Exception:
            pass

    def stop_process(self, name: str):
        """Stop a specific process gracefully."""
        with self.lock:
            if name not in self.processes:
                return

            proc = self.processes[name]
            print(f"[STOPPING] Stopping {name} (PID: {proc.pid})...")

            try:
                # Try graceful shutdown first
                proc.terminate()
                try:
                    proc.wait(timeout=5)
                    print(f"[OK] {name} stopped gracefully")
                except subprocess.TimeoutExpired:
                    print(f"[WARNING]  {name} didn't stop gracefully, killing...")
                    proc.kill()
                    proc.wait()
                    print(f"[OK] {name} killed")
            except Exception as e:
                print(f"[WARNING]  Error stopping {name}: {e}")

            del self.processes[name]

    def cleanup(self):
        """Stop all processes."""
        with self.lock:
            if not self.processes:
                return

            print("\n[CLEANING] Cleaning up processes...")
            for name in list(self.processes.keys()):
                self.stop_process(name)
            print("[OK] All processes stopped")

    def wait(self):
        """Wait for all processes to complete."""
        while self.running and self.processes:
            time.sleep(0.5)


def main():
    """Main orchestrator entry point."""
    if len(sys.argv) < 2:
        print("Usage: orchestrator.py <mode> [options]")
        print("\nModes:")
        print("  full       - Run both client and server")
        print("  client     - Run client only")
        print("  server     - Run server only")
        print("  multi N    - Run N clients and 1 server")
        sys.exit(1)

    mode = sys.argv[1]
    manager = ProcessManager()

    # Find cargo binary
    cargo = "cargo"

    # Get project root
    project_root = Path(__file__).parent.parent.parent

    print("=" * 60)
    print("[GAME] Agent Game Engine - Development Environment")
    print("=" * 60)
    print(f"Mode: {mode}")
    print(f"Root: {project_root}")
    print("=" * 60)
    print()

    try:
        if mode == "full":
            # Start server first
            manager.start_process(
                "server",
                [cargo, "run", "--bin", "server"],
                env={"RUST_LOG": os.getenv("RUST_LOG", "info")},
                cwd=str(project_root),
            )
            time.sleep(2)  # Give server time to start

            # Start client
            manager.start_process(
                "client",
                [cargo, "run", "--bin", "client"],
                env={"RUST_LOG": os.getenv("RUST_LOG", "info")},
                cwd=str(project_root),
            )

        elif mode == "client":
            manager.start_process(
                "client",
                [cargo, "run", "--bin", "client"],
                env={"RUST_LOG": os.getenv("RUST_LOG", "info")},
                cwd=str(project_root),
            )

        elif mode == "server":
            manager.start_process(
                "server",
                [cargo, "run", "--bin", "server"],
                env={"RUST_LOG": os.getenv("RUST_LOG", "info")},
                cwd=str(project_root),
            )

        elif mode == "multi":
            if len(sys.argv) < 3:
                print("Error: multi mode requires client count")
                print("Usage: orchestrator.py multi <count>")
                sys.exit(1)

            count = int(sys.argv[2])

            # Start server
            manager.start_process(
                "server",
                [cargo, "run", "--bin", "server"],
                env={"RUST_LOG": os.getenv("RUST_LOG", "info")},
                cwd=str(project_root),
            )
            time.sleep(2)

            # Start multiple clients
            for i in range(count):
                manager.start_process(
                    f"client-{i+1}",
                    [cargo, "run", "--bin", "client"],
                    env={
                        "RUST_LOG": os.getenv("RUST_LOG", "info"),
                        "CLIENT_ID": str(i + 1),
                    },
                    cwd=str(project_root),
                )
                time.sleep(0.5)

        else:
            print(f"[ERROR] Unknown mode: {mode}")
            sys.exit(1)

        print()
        print("[OK] All processes started")
        print("[LOG] Press Ctrl+C to stop all processes")
        print()

        # Wait for processes
        manager.wait()

    except KeyboardInterrupt:
        print("\n\n[STOPPING] Keyboard interrupt received")
    except Exception as e:
        print(f"\n[ERROR] Error: {e}")
        sys.exit(1)
    finally:
        manager.cleanup()


if __name__ == "__main__":
    main()
