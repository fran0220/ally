# SSE Reconnect Smoke Report

- Generated at: 2026-03-01T13:41:58.243Z
- Result: PASS
- Project ID: global-asset-hub
- First Task ID: 24e7ed72-a519-4b41-ae4a-a2756d6d494a
- Second Task ID: 0b1dbac8-941c-44b4-81c1-f4cb7231240f
- Last Event ID Before Disconnect: 87
- Reconnect Event ID: 89

## Assertions

- first connection established via event task.lifecycle
- first lifecycle event received for task 24e7ed72-a519-4b41-ae4a-a2756d6d494a with id=87
- first connection closed intentionally
- reconnect received task lifecycle event for 0b1dbac8-941c-44b4-81c1-f4cb7231240f
- replayed event id 89 > last-event-id 87

