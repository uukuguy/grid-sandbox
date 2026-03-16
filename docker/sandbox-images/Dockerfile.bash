FROM alpine:3.21
COPY install-base-alpine.sh /tmp/
RUN chmod +x /tmp/install-base-alpine.sh && /tmp/install-base-alpine.sh && rm /tmp/install-base-alpine.sh
RUN adduser -D -s /bin/bash sandbox
USER sandbox
WORKDIR /workspace
LABEL org.octo-sandbox.type="bash" org.octo-sandbox.version="1.0"
