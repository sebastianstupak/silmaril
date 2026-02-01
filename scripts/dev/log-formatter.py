#!/usr/bin/env python3
"""
Log Formatter

Pretty-prints structured logs with color coding and filtering.
"""

import sys
import re
import json
from datetime import datetime
from typing import Optional

# Configure UTF-8 output for Windows
if sys.platform == "win32":
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8', errors='replace')



class Colors:
    """ANSI color codes."""
    RESET = "\033[0m"
    BOLD = "\033[1m"
    DIM = "\033[2m"

    # Levels
    ERROR = "\033[91m"    # Bright red
    WARN = "\033[93m"     # Bright yellow
    INFO = "\033[92m"     # Bright green
    DEBUG = "\033[94m"    # Bright blue
    TRACE = "\033[90m"    # Dark gray

    # Components
    TIMESTAMP = "\033[90m"  # Dark gray
    MODULE = "\033[96m"     # Cyan
    SPAN = "\033[95m"       # Magenta


class LogFormatter:
    """Formats logs with colors and filtering."""

    def __init__(
        self,
        min_level: str = "INFO",
        module_filter: Optional[str] = None,
        color: bool = True,
    ):
        self.min_level = min_level.upper()
        self.module_filter = module_filter
        self.color = color

        self.level_priority = {
            "TRACE": 0,
            "DEBUG": 1,
            "INFO": 2,
            "WARN": 3,
            "ERROR": 4,
        }

    def _should_display(self, level: str, module: str) -> bool:
        """Check if log should be displayed."""
        # Check level
        log_priority = self.level_priority.get(level.upper(), 2)
        min_priority = self.level_priority.get(self.min_level, 2)
        if log_priority < min_priority:
            return False

        # Check module filter
        if self.module_filter and self.module_filter not in module:
            return False

        return True

    def _colorize(self, text: str, color: str) -> str:
        """Apply color to text if enabled."""
        if not self.color:
            return text
        return f"{color}{text}{Colors.RESET}"

    def _format_timestamp(self, timestamp: str) -> str:
        """Format timestamp."""
        try:
            dt = datetime.fromisoformat(timestamp.replace('Z', '+00:00'))
            formatted = dt.strftime("%H:%M:%S.%f")[:-3]
            return self._colorize(formatted, Colors.TIMESTAMP)
        except:
            return self._colorize(timestamp, Colors.TIMESTAMP)

    def _format_level(self, level: str) -> str:
        """Format log level with color."""
        level = level.upper()
        color_map = {
            "ERROR": Colors.ERROR,
            "WARN": Colors.WARN,
            "INFO": Colors.INFO,
            "DEBUG": Colors.DEBUG,
            "TRACE": Colors.TRACE,
        }
        color = color_map.get(level, Colors.RESET)
        return self._colorize(f"{level:<5}", color)

    def _format_module(self, module: str) -> str:
        """Format module name."""
        # Shorten module path for readability
        parts = module.split("::")
        if len(parts) > 3:
            module = "::".join(parts[:2] + ["..."] + parts[-1:])
        return self._colorize(module, Colors.MODULE)

    def format_json(self, line: str) -> Optional[str]:
        """Format JSON log line."""
        try:
            log = json.loads(line)
            level = log.get("level", "INFO")
            timestamp = log.get("timestamp", "")
            target = log.get("target", "unknown")
            message = log.get("message", "")
            fields = log.get("fields", {})

            if not self._should_display(level, target):
                return None

            # Build formatted line
            parts = []

            # Timestamp
            if timestamp:
                parts.append(self._format_timestamp(timestamp))

            # Level
            parts.append(self._format_level(level))

            # Module
            parts.append(self._format_module(target))

            # Message
            parts.append(message)

            # Fields
            if fields:
                field_strs = [f"{k}={v}" for k, v in fields.items()]
                fields_str = " ".join(field_strs)
                parts.append(self._colorize(f"[{fields_str}]", Colors.DIM))

            return " ".join(parts)

        except json.JSONDecodeError:
            # Not JSON, try plain text
            return self.format_plain(line)

    def format_plain(self, line: str) -> Optional[str]:
        """Format plain text log line."""
        # Try to parse common log format: TIMESTAMP LEVEL MODULE - MESSAGE
        pattern = r"(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z?)\s+(\w+)\s+([^\s]+)\s+-\s+(.+)"
        match = re.match(pattern, line)

        if match:
            timestamp, level, module, message = match.groups()

            if not self._should_display(level, module):
                return None

            parts = [
                self._format_timestamp(timestamp),
                self._format_level(level),
                self._format_module(module),
                message,
            ]
            return " ".join(parts)

        # Fallback: just return the line
        return line

    def format(self, line: str) -> Optional[str]:
        """Format a log line (auto-detect format)."""
        line = line.rstrip()
        if not line:
            return None

        # Try JSON first
        if line.startswith("{"):
            return self.format_json(line)

        # Try plain text
        return self.format_plain(line)


def main():
    """Main entry point - read stdin and format logs."""
    import argparse

    parser = argparse.ArgumentParser(description="Pretty log formatter")
    parser.add_argument(
        "--level",
        default="INFO",
        choices=["TRACE", "DEBUG", "INFO", "WARN", "ERROR"],
        help="Minimum log level to display",
    )
    parser.add_argument(
        "--module",
        help="Filter logs by module (substring match)",
    )
    parser.add_argument(
        "--no-color",
        action="store_true",
        help="Disable color output",
    )
    parser.add_argument(
        "--format",
        choices=["auto", "json", "plain"],
        default="auto",
        help="Log format",
    )

    args = parser.parse_args()

    formatter = LogFormatter(
        min_level=args.level,
        module_filter=args.module,
        color=not args.no_color,
    )

    try:
        for line in sys.stdin:
            formatted = formatter.format(line)
            if formatted:
                print(formatted)
                sys.stdout.flush()
    except KeyboardInterrupt:
        pass
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
