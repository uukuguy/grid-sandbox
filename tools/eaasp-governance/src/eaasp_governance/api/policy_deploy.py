"""契约 1: 策略部署 API (§8.1).

PUT  /v1/policies/deploy  — compile & store policy
GET  /v1/policies          — list deployed policies
GET  /v1/policies/{id}     — get policy details
"""

from __future__ import annotations

from fastapi import APIRouter, HTTPException, Request

from eaasp_governance.compiler import CompileError, compile_yaml_to_hooks

router = APIRouter(prefix="/v1/policies", tags=["policies"])


@router.put("/deploy")
async def deploy_policy(request: Request):
    """Compile and deploy a policy YAML."""
    body = await request.body()
    yaml_content = body.decode("utf-8")

    try:
        hooks_json, digest = compile_yaml_to_hooks(yaml_content)
    except CompileError as e:
        raise HTTPException(status_code=400, detail=str(e))

    # Store in app state
    store = request.app.state.policy_store
    from eaasp_governance.compiler import compile_policy_yaml

    bundle = compile_policy_yaml(yaml_content)
    policy_id = bundle.metadata.name

    store[policy_id] = {
        "id": policy_id,
        "name": bundle.metadata.name,
        "scope": bundle.metadata.scope,
        "org_unit": bundle.metadata.org_unit,
        "version": bundle.metadata.version,
        "rules_count": len(bundle.rules),
        "compiled_hooks_json": hooks_json,
        "compiled_hooks_digest": digest,
    }

    return {
        "policy_id": policy_id,
        "rules_count": len(bundle.rules),
        "compiled_hooks_digest": digest,
    }


@router.get("")
async def list_policies(request: Request):
    """List all deployed policies."""
    store = request.app.state.policy_store
    return [
        {
            "id": p["id"],
            "name": p["name"],
            "scope": p["scope"],
            "org_unit": p["org_unit"],
            "version": p["version"],
            "rules_count": p["rules_count"],
        }
        for p in store.values()
    ]


@router.get("/{policy_id}")
async def get_policy(policy_id: str, request: Request):
    """Get policy details including compiled hooks."""
    store = request.app.state.policy_store
    policy = store.get(policy_id)
    if not policy:
        raise HTTPException(status_code=404, detail=f"Policy not found: {policy_id}")
    return policy
