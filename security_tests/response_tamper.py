"""
⚠️ DEPRECATED - DO NOT USE ⚠️

This is a mitmproxy HTTP script that does NOT test HSIP.
HSIP is a UDP protocol. HTTP response tampering doesn't apply to UDP.

For proper HSIP AEAD tampering testing, use:
    PowerShell: .\hsip_response_tamper.ps1
    or manually: Capture UDP with Wireshark, modify ciphertext bytes,
                 replay packet → HSIP AEAD will reject it

This file is kept for historical reference only.
"""

# DEPRECATED CODE BELOW - DO NOT USE

from mitmproxy import http

def response(flow: http.HTTPFlow) -> None:
    """Tamper with response data to test AEAD authentication"""

    if not flow.response:
        return

    print(f"[ATTACK] Intercepted response from {flow.request.host}")
    print(f"[ATTACK] Original status: {flow.response.status_code}")
    print(f"[ATTACK] Original content length: {len(flow.response.content)}")

    # Attack 1: Replace entire response body
    original_content = flow.response.content
    flow.response.content = b"TAMPERED_DATA_BY_ATTACKER"

    # Attack 2: Modify response headers
    flow.response.headers["X-Tampered"] = "true"

    # Attack 3: Change status code
    original_status = flow.response.status_code
    flow.response.status_code = 200

    print(f"[ATTACK] Tampered response body")
    print(f"[ATTACK] Changed status: {original_status} -> {flow.response.status_code}")
    print(f"[ATTACK] Expected: HSIP AEAD should detect tampering and reject")

def request(flow: http.HTTPFlow) -> None:
    """Track request for correlation"""
    print(f"[INFO] Request: {flow.request.method} {flow.request.url}")
