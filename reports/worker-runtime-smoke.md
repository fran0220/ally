# Worker Runtime Smoke Report

- Generated at: 2026-03-04T08:26:54.395Z
- Total: 4
- Executed: 4
- Skipped: 0
- Pass: 0
- Fail: 4

| Case | Result | Submit Status | Final Status | Task ID | Elapsed(ms) | Polls | Error Code |
|---|---|---|---|---|---|---|---|
| image-asset-hub-generate | FAIL | 400 | unknown |  | 691 | 0 |  |
| text-asset-hub-ai-design-character | FAIL | 400 | unknown |  | 659 | 0 |  |
| voice-asset-hub-voice-design | FAIL | 400 | unknown |  | 255 | 0 |  |
| video-novel-generate-video | FAIL | 400 | unknown |  | 251 | 0 |  |

## Skipped Cases

- none

## Failures

- image-asset-hub-generate (POST /api/asset-hub/generate-image)
  - submit response missing taskId
  - submit status 400 != expected 200
- text-asset-hub-ai-design-character (POST /api/asset-hub/ai-design-character)
  - submit response missing taskId
  - submit status 400 != expected 200
- voice-asset-hub-voice-design (POST /api/asset-hub/voice-design)
  - submit response missing taskId
  - submit status 400 != expected 200
- video-novel-generate-video (POST /api/novel-promotion/38ee4854-f3e0-4599-9135-e9483a1620ec/generate-video)
  - submit response missing taskId
  - submit status 400 != expected 200

