/**
 * ccb-runtime gRPC server entry point.
 *
 * Listens on CCB_RUNTIME_GRPC_ADDR (default 0.0.0.0:50057).
 * Uses @connectrpc/connect-node with the gRPC protocol.
 *
 * NOTE: This is a scaffold using a minimal hand-rolled gRPC server
 * (net.createServer + HTTP/2 framing via connect-node). For the
 * contract test suite the server only needs to respond to the 16
 * RuntimeService methods; full ccb subprocess wiring is deferred.
 */

import { CcbRuntimeService } from "./service.js";
import { startGrpcServer } from "./server.js";

const addr = process.env["CCB_RUNTIME_GRPC_ADDR"] ?? "0.0.0.0:50057";
const deploymentMode = process.env["EAASP_DEPLOYMENT_MODE"] ?? "shared";

const [host, portStr] = addr.split(":");
const port = parseInt(portStr ?? "50057", 10);

const service = new CcbRuntimeService(deploymentMode);

console.info(`[ccb-runtime] starting on ${addr} mode=${deploymentMode}`);
await startGrpcServer(service, host ?? "0.0.0.0", port);
