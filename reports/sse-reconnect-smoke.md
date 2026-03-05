# SSE Reconnect Smoke Report

- Generated at: 2026-03-04T08:30:12.135Z
- Result: FAIL
- Project ID: global-asset-hub
- First Task ID: N/A
- Second Task ID: N/A
- Last Event ID Before Disconnect: N/A
- Reconnect Event ID: N/A

## Assertions

- first connection established via event heartbeat

## Error

- task submit failed status=500 body={"success":false,"requestId":"a54f85ae-661a-41db-9672-143d6f2d614a","error":{"code":"INTERNAL_ERROR","message":"database error: error returned from database: 1054 (42S22): Unknown column 'billingInfo' in 'field list'","retryable":false,"category":"SYSTEM","user_message_key":"errors.INTERNAL_ERROR"},"code":"INTERNAL_ERROR","message":"database error: error returned from database: 1054 (42S22): Unknown column 'billingInfo' in 'field list'"}

