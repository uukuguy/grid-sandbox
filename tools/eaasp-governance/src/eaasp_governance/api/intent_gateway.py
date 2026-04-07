"""契约 2: 意图网关 API (§8.2).

POST /v1/intents — resolve user text to skill_id
"""

from __future__ import annotations

import uuid

from fastapi import APIRouter, Request
from pydantic import BaseModel

router = APIRouter(prefix="/v1/intents", tags=["intents"])

# MVP: keyword-based intent mapping (future: NLU model)
_INTENT_MAP: dict[str, str] = {
    "入职": "hr-onboarding",
    "onboarding": "hr-onboarding",
    "新员工": "hr-onboarding",
    "hire": "hr-onboarding",
}


class IntentRequest(BaseModel):
    text: str
    user_id: str
    org_unit: str = ""


@router.post("")
async def resolve_intent(req: IntentRequest, request: Request):
    """Resolve user text to a skill_id via keyword matching."""
    text_lower = req.text.lower()
    matched_skill = None
    confidence = 0.0

    for keyword, skill_id in _INTENT_MAP.items():
        if keyword in text_lower:
            matched_skill = skill_id
            confidence = 0.9
            break

    if not matched_skill:
        return {
            "intent_id": f"int-{uuid.uuid4().hex[:8]}",
            "skill_id": None,
            "confidence": 0.0,
            "skill_name": None,
        }

    return {
        "intent_id": f"int-{uuid.uuid4().hex[:8]}",
        "skill_id": matched_skill,
        "confidence": confidence,
        "skill_name": matched_skill,
    }
