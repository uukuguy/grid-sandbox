/**
 * gRPC server wiring for ccb-runtime using @grpc/grpc-js + proto-loader.
 *
 * Loads runtime.proto at startup (no code-gen needed) and registers all
 * 16 RuntimeService methods via CcbRuntimeService.
 */

import * as grpc from "@grpc/grpc-js";
import * as protoLoader from "@grpc/proto-loader";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";
import type { CcbRuntimeService } from "./service.js";

const __dirname = dirname(fileURLToPath(import.meta.url));

// Resolve proto root: lang/ccb-runtime-ts/src/ -> repo root -> proto/
const REPO_ROOT = resolve(__dirname, "..", "..", "..", "..");
const PROTO_PATH = resolve(REPO_ROOT, "proto", "eaasp", "runtime", "v2", "runtime.proto");

function loadProto() {
  const pkgDef = protoLoader.loadSync(PROTO_PATH, {
    keepCase: false,
    longs: String,
    enums: String,
    defaults: true,
    oneofs: true,
    includeDirs: [resolve(REPO_ROOT, "proto")],
  });
  return grpc.loadPackageDefinition(pkgDef) as any;
}

export async function startGrpcServer(
  svc: CcbRuntimeService,
  host: string,
  port: number,
): Promise<void> {
  const proto = loadProto();
  const RuntimeService = proto.eaasp.runtime.v2.RuntimeService;

  const server = new grpc.Server();

  server.addService(RuntimeService.service, {
    Initialize(call: grpc.ServerUnaryCall<any, any>, cb: grpc.sendUnaryData<any>) {
      cb(null, svc.initialize(call.request));
    },

    Send(call: grpc.ServerWritableStream<any, any>) {
      (async () => {
        try {
          for await (const chunk of svc.send(call.request)) {
            call.write(chunk);
          }
          call.end();
        } catch (err) {
          call.destroy(err as Error);
        }
      })();
    },

    LoadSkill(call: grpc.ServerUnaryCall<any, any>, cb: grpc.sendUnaryData<any>) {
      cb(null, svc.loadSkill(call.request));
    },

    OnToolCall(call: grpc.ServerUnaryCall<any, any>, cb: grpc.sendUnaryData<any>) {
      cb(null, svc.onToolCall(call.request));
    },

    OnToolResult(call: grpc.ServerUnaryCall<any, any>, cb: grpc.sendUnaryData<any>) {
      cb(null, svc.onToolResult(call.request));
    },

    OnStop(call: grpc.ServerUnaryCall<any, any>, cb: grpc.sendUnaryData<any>) {
      cb(null, svc.onStop(call.request));
    },

    GetState(call: grpc.ServerUnaryCall<any, any>, cb: grpc.sendUnaryData<any>) {
      cb(null, svc.getState());
    },

    ConnectMCP(call: grpc.ServerUnaryCall<any, any>, cb: grpc.sendUnaryData<any>) {
      cb(null, svc.connectMcp(call.request));
    },

    EmitTelemetry(call: grpc.ServerUnaryCall<any, any>, cb: grpc.sendUnaryData<any>) {
      cb(null, svc.emitTelemetry(call.request));
    },

    GetCapabilities(call: grpc.ServerUnaryCall<any, any>, cb: grpc.sendUnaryData<any>) {
      cb(null, svc.getCapabilities());
    },

    Terminate(call: grpc.ServerUnaryCall<any, any>, cb: grpc.sendUnaryData<any>) {
      cb(null, svc.terminate());
    },

    RestoreState(call: grpc.ServerUnaryCall<any, any>, cb: grpc.sendUnaryData<any>) {
      cb(null, svc.restoreState(call.request));
    },

    Health(call: grpc.ServerUnaryCall<any, any>, cb: grpc.sendUnaryData<any>) {
      cb(null, svc.health());
    },

    DisconnectMcp(call: grpc.ServerUnaryCall<any, any>, cb: grpc.sendUnaryData<any>) {
      cb(null, svc.disconnectMcp(call.request));
    },

    PauseSession(call: grpc.ServerUnaryCall<any, any>, cb: grpc.sendUnaryData<any>) {
      cb(null, svc.pauseSession());
    },

    ResumeSession(call: grpc.ServerUnaryCall<any, any>, cb: grpc.sendUnaryData<any>) {
      cb(null, svc.resumeSession(call.request));
    },

    EmitEvent(call: grpc.ServerUnaryCall<any, any>, cb: grpc.sendUnaryData<any>) {
      cb(null, svc.emitEvent(call.request));
    },
  });

  await new Promise<void>((resolve, reject) => {
    server.bindAsync(
      `${host}:${port}`,
      grpc.ServerCredentials.createInsecure(),
      (err, boundPort) => {
        if (err) { reject(err); return; }
        console.info(`[ccb-runtime] gRPC listening on port ${boundPort}`);
        resolve();
      },
    );
  });

  // Keep process alive until SIGTERM/SIGINT.
  await new Promise<void>((resolve) => {
    const shutdown = () => {
      server.tryShutdown(() => resolve());
    };
    process.on("SIGTERM", shutdown);
    process.on("SIGINT", shutdown);
  });
}
