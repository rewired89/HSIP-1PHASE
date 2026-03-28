# Example: AI Agent Consent Gate

This example shows the core HSIP pattern for AI agents:

> **An AI agent must request consent before taking any action on your behalf. HSIP is the consent layer.**

## The Problem

Without a consent layer, an AI agent connected to your systems can:
- Send messages in your name without you knowing
- Book appointments you didn't approve
- Access data you didn't intend to share
- Take actions that are hard or impossible to reverse

## The HSIP Solution

Every agent action goes through three steps:
1. **Request** — agent requests consent for a specific action
2. **Grant** — you grant consent (time-bounded, scoped)
3. **Act** — agent acts, and the action is signed and logged

If consent is revoked at any point, the agent cannot proceed.

## Scenario

An AI email assistant wants to send a message on your behalf.

```
Agent: "I want to send this draft email to alice@example.com on your behalf.
        Please grant consent."

You: [Review the email] → Grant consent for 10 minutes

Agent: Sends email, signs the action with your key, logs it to HSIP audit trail

You: Later, export audit log to verify exactly what was sent and when
```

## Code
