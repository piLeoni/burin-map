"""The `burin` command: the Rust command line, unchanged. `burin --help`."""

import sys

from . import _burin


def main() -> None:
    try:
        _burin.run_cli(["burin", *sys.argv[1:]])
    except (RuntimeError, ValueError, ConnectionError) as e:
        sys.exit(f"burin: {e}")


if __name__ == "__main__":
    main()
