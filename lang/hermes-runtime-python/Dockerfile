# hermes-runtime — EAASP L1 Tier 2 Aligned adapter container image.
#
# Build from PROJECT ROOT:
#   docker build -f lang/hermes-runtime-python/Dockerfile -t hermes-runtime .
#
# Rebuilds on top of existing hermes-runtime:latest (which has hermes-agent).
# Updates: runtime adapter code + v2 proto stubs.

ARG HERMES_BASE=hermes-runtime:latest
FROM ${HERMES_BASE}

USER root

# Remove old hermes-runtime code
RUN rm -rf /opt/hermes-runtime

# Copy updated hermes-runtime adapter
COPY lang/hermes-runtime-python/ /opt/hermes-runtime/

# Copy claude-code-runtime proto stubs (shared via _fix_proto_imports.py).
# _fix_proto_imports resolves: <here>.parent^4 / "claude-code-runtime-python" / "src"
# <here> = /opt/hermes-runtime/src/hermes_runtime/_fix_proto_imports.py
# parent^4 = /opt → looks for /opt/claude-code-runtime-python/src/
COPY lang/claude-code-runtime-python/src/claude_code_runtime/ \
     /opt/claude-code-runtime-python/src/claude_code_runtime/

WORKDIR /opt/hermes-runtime
RUN uv pip install --no-cache-dir -e ".[dev]" 2>/dev/null || pip install --no-cache-dir -e ".[dev]"

# Link governance plugin
ENV HERMES_HOME=/opt/data
RUN mkdir -p /opt/data/plugins && \
    ln -sf /opt/hermes-runtime/src/hermes_runtime/governance_plugin /opt/data/plugins/grid-governance

# Use hermes user if it exists (hermes-agent base), otherwise root.
RUN id hermes >/dev/null 2>&1 && echo "hermes user exists" || echo "no hermes user, running as root"
USER root
EXPOSE 50053

CMD ["python", "-m", "hermes_runtime"]
