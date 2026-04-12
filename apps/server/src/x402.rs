use reqwest::Client;
use serde::Deserialize;
use std::sync::LazyLock;

static HTTP: LazyLock<Client> = LazyLock::new(Client::new);

pub struct ToolResult {
    pub body: String,
    /// USDC minor units (6 decimals).
    pub cost: u64,
}

pub async fn call_tool(tool_id: &str, prompt: &str) -> Result<ToolResult, String> {
    let result = match tool_id {
        t if t.starts_with("gemini/") => call_gemini(t, prompt).await?,
        t if t.starts_with("groq/") => call_groq(t, prompt).await?,
        t if t.starts_with("openai/") => call_openai(t, prompt).await?,
        t if t.starts_with("coingecko/") => simulate_tool(t, prompt)?,
        t if t.starts_with("pyth/") => simulate_tool(t, prompt)?,
        t if t.starts_with("helius/") => simulate_tool(t, prompt)?,
        _ => return Err(format!("unknown tool: {tool_id}")),
    };

    tracing::info!(tool = tool_id, cost = result.cost, "tool call completed");
    Ok(result)
}

async fn call_gemini(model_path: &str, prompt: &str) -> Result<ToolResult, String> {
    let api_key = std::env::var("GEMINI_API_KEY")
        .map_err(|_| "GEMINI_API_KEY not set".to_string())?;

    let model = model_path.strip_prefix("gemini/").unwrap_or("gemini-2.0-flash");

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"
    );

    let body = serde_json::json!({
        "contents": [{ "parts": [{ "text": prompt }] }]
    });

    let resp = HTTP
        .post(&url)
        .header("x-goog-api-key", &api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("gemini request: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("gemini {status}: {text}"));
    }

    let json: GeminiResponse = resp
        .json()
        .await
        .map_err(|e| format!("gemini parse: {e}"))?;

    let text = json
        .candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.content.parts.into_iter().next())
        .map(|p| p.text)
        .unwrap_or_default();

    let cost = estimate_cost_gemini(prompt.len(), text.len());

    Ok(ToolResult { body: text, cost })
}

async fn call_groq(model_path: &str, prompt: &str) -> Result<ToolResult, String> {
    let api_key = std::env::var("GROQ_API_KEY")
        .map_err(|_| "GROQ_API_KEY not set".to_string())?;

    let model = model_path.strip_prefix("groq/").unwrap_or("llama-3.3-70b-versatile");

    let body = serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": prompt }],
        "max_tokens": 1024,
    });

    let resp = HTTP
        .post("https://api.groq.com/openai/v1/chat/completions")
        .bearer_auth(&api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("groq request: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("groq {status}: {text}"));
    }

    let json: OpenAIChatResponse = resp
        .json()
        .await
        .map_err(|e| format!("groq parse: {e}"))?;

    let text = json
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .unwrap_or_default();

    let cost = estimate_cost_groq(prompt.len(), text.len());

    Ok(ToolResult { body: text, cost })
}

async fn call_openai(model_path: &str, prompt: &str) -> Result<ToolResult, String> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| "OPENAI_API_KEY not set".to_string())?;

    let model = model_path.strip_prefix("openai/").unwrap_or("gpt-4o-mini");

    let body = serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": prompt }],
        "max_tokens": 1024,
    });

    let resp = HTTP
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(&api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("openai request: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("openai {status}: {text}"));
    }

    let json: OpenAIChatResponse = resp
        .json()
        .await
        .map_err(|e| format!("openai parse: {e}"))?;

    let text = json
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .unwrap_or_default();

    let cost = estimate_cost_openai(model, prompt.len(), text.len());

    Ok(ToolResult { body: text, cost })
}

fn simulate_tool(tool_id: &str, prompt: &str) -> Result<ToolResult, String> {
    match tool_id {
        t if t.starts_with("coingecko/") => Ok(ToolResult {
            body: r#"{"sol_usd":148.25,"btc_usd":67420.50}"#.into(),
            cost: 500_000,
        }),
        t if t.starts_with("pyth/") => Ok(ToolResult {
            body: r#"{"price":148.12,"conf":0.08,"slot":312456789}"#.into(),
            cost: 750_000,
        }),
        t if t.starts_with("helius/") => Ok(ToolResult {
            body: r#"{"status":"ok","rpc":"enhanced","latency_ms":12}"#.into(),
            cost: 1_200_000,
        }),
        _ => Err(format!("unknown tool: {tool_id} (prompt: {prompt})")),
    }
}

/// Rough cost in USDC minor units. ~$0.10/1M input tokens, ~$0.40/1M output tokens for Flash.
fn estimate_cost_gemini(input_chars: usize, output_chars: usize) -> u64 {
    let input_tokens = input_chars / 4;
    let output_tokens = output_chars / 4;
    let microdollars = (input_tokens as u64 * 100 + output_tokens as u64 * 400) / 1_000_000;
    microdollars.max(50_000) // minimum 0.05 USDC
}

/// Groq is very cheap. ~$0.59/1M input, ~$0.79/1M output for llama-3.3-70b.
fn estimate_cost_groq(input_chars: usize, output_chars: usize) -> u64 {
    let input_tokens = input_chars / 4;
    let output_tokens = output_chars / 4;
    let microdollars = (input_tokens as u64 * 590 + output_tokens as u64 * 790) / 1_000_000;
    microdollars.max(50_000)
}

fn estimate_cost_openai(model: &str, input_chars: usize, output_chars: usize) -> u64 {
    let input_tokens = input_chars / 4;
    let output_tokens = output_chars / 4;

    // Rough per-million-token pricing
    let (input_rate, output_rate): (u64, u64) = if model.contains("gpt-4o-mini") {
        (150, 600)       // $0.15 / $0.60
    } else if model.contains("gpt-4o") {
        (2_500, 10_000)  // $2.50 / $10.00
    } else {
        (1_000, 3_000)   // default estimate
    };

    let microdollars =
        (input_tokens as u64 * input_rate + output_tokens as u64 * output_rate) / 1_000_000;
    microdollars.max(100_000) // minimum 0.10 USDC for OpenAI
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Deserialize)]
struct OpenAIChatResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Deserialize)]
struct OpenAIMessage {
    content: String,
}
