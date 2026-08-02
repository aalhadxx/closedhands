"""HTTP MCP client for Mangools — used by SEO Expert agent."""
import json
import httpx

URL = "https://mcp.mangools.com/mcp"
HEADERS = {
    "x-access-token": "c4c9d93202b61a8758a9a64849d785a2a9caf58083cad2e2ba04f618ad681e74",
    "Content-Type": "application/json",
    "Accept": "application/json, text/event-stream",
}


class MangoolsMCP:
    def __init__(self):
        self._id = 0
        self._client = httpx.Client(headers=HEADERS, timeout=60)
        self._initialized = False
        self._init()

    def _init(self):
        payload = {
            "jsonrpc": "2.0",
            "id": self._next_id(),
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "closedhands", "version": "0.1.0"},
            },
        }
        resp = self._client.post(URL, json=payload)
        resp.raise_for_status()
        self._initialized = True

    def _next_id(self):
        self._id += 1
        return self._id

    def call(self, tool_name: str, arguments: dict):
        if not self._initialized:
            self._init()
        payload = {
            "jsonrpc": "2.0",
            "id": self._next_id(),
            "method": "tools/call",
            "params": {"name": tool_name, "arguments": arguments},
        }
        resp = self._client.post(URL, json=payload)
        resp.raise_for_status()
        data = resp.json()
        if "error" in data:
            raise RuntimeError(f"MCP error: {data['error']}")
        return data["result"]

    def search_related_keywords(self, kw: str, location_id: int = 0):
        return self.call("kwfinder_search_related_keywords", {"kw": kw, "location_id": location_id})

    def get_keyword_details(self, kw: str, location_id: int = 0):
        return self.call("kwfinder_get_keyword_details", {"kw": kw, "location_id": location_id})

    def get_serp(self, kw: str, location_id: int = 0):
        return self.call("serpchecker_get_serp", {"kw": kw, "location_id": location_id})


if __name__ == "__main__":
    mcp = MangoolsMCP()
    result = mcp.search_related_keywords("luxury system monitor")
    print(json.dumps(result, indent=2, default=str))
