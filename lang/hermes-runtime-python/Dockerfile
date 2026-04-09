FROM python:3.11-slim

# System deps for hermes-agent
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential nodejs npm ripgrep git && \
    rm -rf /var/lib/apt/lists/*

# Install hermes-agent from local source
COPY 3th-party/harnesses/hermes-agent /opt/hermes
WORKDIR /opt/hermes
RUN pip install --no-cache-dir -e ".[all]"

# Install hermes-runtime
COPY lang/hermes-runtime-python /opt/hermes-runtime
WORKDIR /opt/hermes-runtime
RUN pip install --no-cache-dir -e ".[dev]"

# Link governance plugin into hermes plugin directory
ENV HERMES_HOME=/opt/data
RUN mkdir -p /opt/data/plugins && \
    ln -s /opt/hermes-runtime/src/hermes_runtime/governance_plugin /opt/data/plugins/grid-governance

EXPOSE 50053

CMD ["python", "-m", "hermes_runtime"]
