# Example: LangChain Tool — HSIP Consent + Audit

Adds HSIP as a LangChain tool so any LangChain agent automatically:
- Signs messages before sending
- Requests consent before acting on your behalf
- Logs every action to the tamper-proof HSIP audit trail

## Install

```bash
pip install langchain langchain-openai hsip-sdk
```

## Usage

```python
from langchain_hsip import HSIPSignTool, HSIPConsentTool, HSIPAuditTool
from langchain.agents import AgentExecutor, create_tool_calling_agent
from langchain_openai import ChatOpenAI

tools = [HSIPSignTool(), HSIPConsentTool(), HSIPAuditTool()]
llm = ChatOpenAI(model="gpt-4o")
agent = create_tool_calling_agent(llm, tools, prompt)
executor = AgentExecutor(agent=agent, tools=tools)

result = executor.invoke({
    "input": "Sign a message confirming my agreement to the contract terms."
})
```

See `langchain_hsip_tools.py` for the full tool implementations.
