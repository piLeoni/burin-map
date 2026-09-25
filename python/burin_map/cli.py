"""The `burin-map` command: the Rust CLI via the Python entry point."""

import sys

from . import _burin


def main() -> None:
    try:
        _burin.run_cli(["burin-map", *sys.argv[1:]])
    except (RuntimeError, ValueError, ConnectionError) as e:
        sys.exit(f"burin-map: {e}")


if __name__ == "__main__":
    main()
