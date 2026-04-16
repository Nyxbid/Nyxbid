// MCP (Model Context Protocol) surface for AI agents.
// Exposes the intent lifecycle as MCP tools so any MCP-capable agent
// (Claude, GPT, Gemini via adapters) can trade natively.
//
// Planned tools:
//   - nyxbid.list_markets
//   - nyxbid.create_intent { symbol, side, size, limit }
//   - nyxbid.list_quotes { intent_id }
//   - nyxbid.get_receipt { intent_id }
//   - nyxbid.cancel_intent { intent_id }
//
// Transport: JSON-RPC over stdio for local agents,
// HTTP+SSE at /mcp for hosted agents.
