"""Hook handler script generator."""

from __future__ import annotations

import textwrap


class HookBuilder:
    """Generate hook handler scripts and configs."""

    @staticmethod
    def command_handler(name: str, event: str) -> str:
        """Generate a Python command handler script template.

        The script reads JSON from stdin, performs a check, and prints
        a JSON result to stdout.
        """
        return textwrap.dedent(f"""\
            #!/usr/bin/env python3
            \"\"\"Hook handler: {name} ({event})\"\"\"

            import json
            import sys


            def main():
                payload = json.loads(sys.stdin.read())
                tool_name = payload.get("tool_name", "")
                tool_input = payload.get("tool_input", {{}})

                # TODO: implement your {event} check logic here
                # Example: inspect tool_name and tool_input, decide allow/block

                result = {{
                    "decision": "allow",  # or "block"
                    "reason": "{name} check passed",
                }}
                print(json.dumps(result))


            if __name__ == "__main__":
                main()
        """)

    @staticmethod
    def http_handler(name: str, event: str) -> str:
        """Generate a FastAPI HTTP handler endpoint template."""
        return textwrap.dedent(f"""\
            \"\"\"HTTP hook handler: {name} ({event})

            Run with: uvicorn {name}:app --port 8090
            \"\"\"

            from fastapi import FastAPI

            app = FastAPI()


            @app.post("/hook/{name}")
            async def handle(payload: dict) -> dict:
                tool_name = payload.get("tool_name", "")
                tool_input = payload.get("tool_input", {{}})

                # TODO: implement your {event} check logic here

                return {{
                    "decision": "allow",
                    "reason": "{name} check passed",
                }}
        """)

    @staticmethod
    def prompt_handler(prompt: str) -> dict:
        """Return a config dict for a prompt-type handler.

        Prompt handlers don't need script generation — they are inline
        configurations in the SKILL.md frontmatter.
        """
        return {
            "handler_type": "prompt",
            "config": {"prompt": prompt},
        }
