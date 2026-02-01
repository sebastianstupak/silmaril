#!/usr/bin/env python3
"""
Process Manager

Handles process lifecycle, PID files, and status tracking.
"""

import os
import sys
import signal
import json
import psutil
from pathlib import Path
from typing import Optional, Dict, List
from datetime import datetime

# Configure UTF-8 output for Windows
if sys.platform == "win32":
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8', errors='replace')


class ProcessInfo:
    """Information about a managed process."""

    def __init__(self, name: str, pid: int, command: str, started_at: str):
        self.name = name
        self.pid = pid
        self.command = command
        self.started_at = started_at

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "pid": self.pid,
            "command": self.command,
            "started_at": self.started_at,
        }

    @classmethod
    def from_dict(cls, data: dict) -> "ProcessInfo":
        return cls(
            name=data["name"],
            pid=data["pid"],
            command=data["command"],
            started_at=data["started_at"],
        )


class ProcessManager:
    """Manages process lifecycle with PID tracking."""

    def __init__(self, state_dir: Optional[Path] = None):
        if state_dir is None:
            # Use temp directory
            if sys.platform == "win32":
                state_dir = Path(os.getenv("TEMP", "C:\\temp")) / "agent-game-engine"
            else:
                state_dir = Path("/tmp/agent-game-engine")

        self.state_dir = Path(state_dir)
        self.state_dir.mkdir(parents=True, exist_ok=True)
        self.state_file = self.state_dir / "processes.json"

    def _load_state(self) -> Dict[str, ProcessInfo]:
        """Load process state from file."""
        if not self.state_file.exists():
            return {}

        try:
            with open(self.state_file, 'r') as f:
                data = json.load(f)
                return {
                    name: ProcessInfo.from_dict(info)
                    for name, info in data.items()
                }
        except Exception as e:
            print(f"WARNING: Error loading state: {e}")
            return {}

    def _save_state(self, processes: Dict[str, ProcessInfo]):
        """Save process state to file."""
        try:
            with open(self.state_file, 'w') as f:
                data = {
                    name: info.to_dict()
                    for name, info in processes.items()
                }
                json.dump(data, f, indent=2)
        except Exception as e:
            print(f"WARNING: Error saving state: {e}")

    def register(self, name: str, pid: int, command: str):
        """Register a new process."""
        processes = self._load_state()
        processes[name] = ProcessInfo(
            name=name,
            pid=pid,
            command=command,
            started_at=datetime.now().isoformat(),
        )
        self._save_state(processes)
        print(f"[OK] Registered {name} (PID: {pid})")

    def unregister(self, name: str):
        """Unregister a process."""
        processes = self._load_state()
        if name in processes:
            del processes[name]
            self._save_state(processes)
            print(f"[OK] Unregistered {name}")

    def get_status(self) -> List[Dict[str, any]]:
        """Get status of all registered processes."""
        processes = self._load_state()
        status = []

        for name, info in processes.items():
            is_running = self._is_running(info.pid)
            status.append({
                "name": name,
                "pid": info.pid,
                "command": info.command,
                "started_at": info.started_at,
                "running": is_running,
            })

        return status

    def _is_running(self, pid: int) -> bool:
        """Check if a process is running."""
        try:
            process = psutil.Process(pid)
            return process.is_running()
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            return False

    def stop_all(self):
        """Stop all registered processes."""
        processes = self._load_state()

        if not processes:
            print("INFO: No processes to stop")
            return

        print("Stopping all registered processes...")

        for name, info in processes.items():
            if self._is_running(info.pid):
                try:
                    print(f"  Stopping {name} (PID: {info.pid})...")
                    process = psutil.Process(info.pid)
                    process.terminate()

                    # Wait for graceful shutdown
                    try:
                        process.wait(timeout=5)
                        print(f"  [OK] {name} stopped gracefully")
                    except psutil.TimeoutExpired:
                        print(f"  WARNING: {name} didn't stop, killing...")
                        process.kill()
                        print(f"  [OK] {name} killed")

                except (psutil.NoSuchProcess, psutil.AccessDenied) as e:
                    print(f"  WARNING: Could not stop {name}: {e}")
            else:
                print(f"  INFO: {name} is not running")

        # Clear state
        self._save_state({})
        print("[OK] All processes stopped")

    def clean_stale(self):
        """Remove stale process entries."""
        processes = self._load_state()
        stale = []

        for name, info in processes.items():
            if not self._is_running(info.pid):
                stale.append(name)

        if stale:
            print(f"Cleaning {len(stale)} stale process entries...")
            for name in stale:
                del processes[name]
            self._save_state(processes)
            print("[OK] Stale entries cleaned")
        else:
            print("[OK] No stale entries found")


def main():
    """CLI entry point."""
    if len(sys.argv) < 2:
        print("Usage: process-manager.py <command> [args]")
        print("\nCommands:")
        print("  register <name> <pid> <command> - Register a process")
        print("  unregister <name>                - Unregister a process")
        print("  status                           - Show process status")
        print("  stop-all                         - Stop all processes")
        print("  clean                            - Clean stale entries")
        sys.exit(1)

    manager = ProcessManager()
    command = sys.argv[1]

    if command == "register":
        if len(sys.argv) < 5:
            print("Usage: process-manager.py register <name> <pid> <command>")
            sys.exit(1)
        name = sys.argv[2]
        pid = int(sys.argv[3])
        cmd = " ".join(sys.argv[4:])
        manager.register(name, pid, cmd)

    elif command == "unregister":
        if len(sys.argv) < 3:
            print("Usage: process-manager.py unregister <name>")
            sys.exit(1)
        name = sys.argv[2]
        manager.unregister(name)

    elif command == "status":
        status = manager.get_status()
        if not status:
            print("INFO: No registered processes")
        else:
            print("Process Status")
            print("=" * 80)
            for proc in status:
                running = "[RUNNING]" if proc["running"] else "[STOPPED]"
                print(f"{proc['name']:<20} PID: {proc['pid']:<8} {running}")
                print(f"  Command: {proc['command']}")
                print(f"  Started: {proc['started_at']}")
                print()

    elif command == "stop-all":
        manager.stop_all()

    elif command == "clean":
        manager.clean_stale()

    else:
        print(f"ERROR: Unknown command: {command}")
        sys.exit(1)


if __name__ == "__main__":
    main()
