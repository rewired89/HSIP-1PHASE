# Capture boundary (one page, fill in before any pilot)

A proof shows that a recorded action is intact. It cannot show an action that was never recorded. This page says exactly what is and is not recorded for one workflow, so a reviewer knows what the evidence covers.

**Workflow:**  **Owner:**  **Date:**  **Version:**

## 1. Action paths
List every way an action in this workflow can happen.

| Path | Automatic or manual | Recorded by HSIP? | Where in the code or system |
|---|---|---|---|
| | | yes / no / partly | |

## 2. What is recorded for each action
Actor, authorizing person or policy, action name, subject, hash of the full record, and which of these happened: proposed, permitted, executed, outcome observed.

## 3. What is not recorded, and why
Administrator changes made directly in a database, manual overrides, actions during an outage, data outside this workflow.

## 4. Failure behavior
What happens if HSIP is unreachable: the action is blocked, queued, or goes ahead unrecorded. Queue size and how long it holds. How gaps are marked.

## 5. How completeness is checked
The source system of record, how often its action count is compared with the HSIP record count, who reviews differences, and the last result.

## 6. Where records live
Which key signs (platform or customer), where original records are kept, how long, who can delete them.

## 7. Questions the reviewer will ask
Write the three questions the outside reviewer wants answered, and which part of the evidence pack answers each.
