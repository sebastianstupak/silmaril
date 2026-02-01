#!/usr/bin/env python3
"""
Port Availability Checker

Checks if required ports are available before starting dev environment.
"""

import socket
import sys
from typing import List, Tuple

# Configure UTF-8 output for Windows
if sys.platform == "win32":
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8', errors='replace')



def check_port(host: str, port: int) -> Tuple[bool, str]:
    """
    Check if a port is available.

    Returns:
        (available, message)
    """
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(1)
        result = sock.connect_ex((host, port))
        sock.close()

        if result == 0:
            return False, f"Port {port} is already in use"
        else:
            return True, f"Port {port} is available"
    except Exception as e:
        return False, f"Error checking port {port}: {e}"


def check_ports(ports: List[Tuple[str, int, str]]) -> bool:
    """
    Check multiple ports.

    Args:
        ports: List of (host, port, description) tuples

    Returns:
        True if all ports are available
    """
    print("[CHECKING] Checking port availability...")
    print()

    all_available = True

    for host, port, description in ports:
        available, message = check_port(host, port)

        if available:
            print(f"[OK] {description} ({host}:{port}) - {message}")
        else:
            print(f"[ERROR] {description} ({host}:{port}) - {message}")
            all_available = False

    print()
    return all_available


def find_process_using_port(port: int):
    """Find which process is using a port (platform-specific)."""
    import subprocess
    import platform

    system = platform.system()

    try:
        if system == "Windows":
            result = subprocess.run(
                ["netstat", "-ano", "-p", "TCP"],
                capture_output=True,
                text=True,
            )
            for line in result.stdout.split('\n'):
                if f":{port} " in line:
                    parts = line.split()
                    if len(parts) >= 5:
                        pid = parts[-1]
                        print(f"  Process ID: {pid}")
                        # Try to get process name
                        try:
                            tasklist = subprocess.run(
                                ["tasklist", "/FI", f"PID eq {pid}", "/FO", "CSV", "/NH"],
                                capture_output=True,
                                text=True,
                            )
                            if tasklist.stdout:
                                name = tasklist.stdout.split(',')[0].strip('"')
                                print(f"  Process Name: {name}")
                        except:
                            pass
        elif system == "Linux":
            result = subprocess.run(
                ["lsof", "-i", f":{port}"],
                capture_output=True,
                text=True,
            )
            if result.stdout:
                print(f"  {result.stdout}")
        elif system == "Darwin":  # macOS
            result = subprocess.run(
                ["lsof", "-i", f":{port}"],
                capture_output=True,
                text=True,
            )
            if result.stdout:
                print(f"  {result.stdout}")
    except Exception as e:
        print(f"  Could not determine process: {e}")


def main():
    """Main entry point."""
    # Default ports for agent-game-engine
    default_ports = [
        ("127.0.0.1", 7777, "Server TCP"),
        ("127.0.0.1", 7778, "Server UDP"),
        ("127.0.0.1", 8080, "Metrics/Health"),
    ]

    # Parse command line for additional ports
    ports = default_ports

    if len(sys.argv) > 1:
        # Custom ports provided
        custom_ports = []
        for arg in sys.argv[1:]:
            try:
                port = int(arg)
                custom_ports.append(("127.0.0.1", port, f"Custom port {port}"))
            except ValueError:
                print(f"[WARNING]  Invalid port: {arg}")

        if custom_ports:
            ports = custom_ports

    # Check all ports
    all_available = check_ports(ports)

    if not all_available:
        print("[ERROR] Some ports are not available")
        print()
        print("To find which process is using a port:")
        for host, port, desc in ports:
            available, _ = check_port(host, port)
            if not available:
                print(f"\nPort {port} ({desc}):")
                find_process_using_port(port)
        print()
        print("Solutions:")
        print("  1. Stop the process using the port")
        print("  2. Configure the engine to use different ports")
        print("  3. Wait for the port to be released")
        sys.exit(1)
    else:
        print("[OK] All ports are available")
        sys.exit(0)


if __name__ == "__main__":
    main()
