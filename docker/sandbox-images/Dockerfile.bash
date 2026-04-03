FROM alpine:3.21
COPY install-base-alpine.sh /tmp/
RUN chmod +x /tmp/install-base-alpine.sh && /tmp/install-base-alpine.sh && rm /tmp/install-base-alpine.sh
RUN adduser -D -s /bin/bash sandbox
USER sandbox
WORKDIR /workspace
LABEL org.grid-sandbox.type="bash" org.grid-sandbox.version="1.0"
