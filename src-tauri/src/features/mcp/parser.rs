const MCP_SPEC_VERSION_SUPPORTED: &str = "1.0";

fn mcp_definition_json_schema() -> Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "Easy Call AI MCP Definition",
        "type": "object",
        "required": ["version", "mcpServers"],
        "properties": {
            "version": { "type": "string", "const": MCP_SPEC_VERSION_SUPPORTED },
            "mcpServers": {
                "type": "object",
                "minProperties": 1,
                "additionalProperties": {
                    "type": "object",
                    "anyOf": [
                        { "required": ["command"] },
                        { "required": ["url"] }
                    ],
                    "properties": {
                        "transport": { "type": "string" },
                        "command": { "type": "string" },
                        "args": { "type": "array", "items": { "type": "string" } },
                        "env": { "type": "object", "additionalProperties": { "type": "string" } },
                        "cwd": { "type": "string" },
                        "url": { "type": "string" },
                        "bearerTokenEnvVar": { "type": "string" },
                        "httpHeaders": { "type": "object", "additionalProperties": { "type": "string" } },
                        "envHttpHeaders": { "type": "object", "additionalProperties": { "type": "string" } },
                        "enabledTools": { "type": "array", "items": { "type": "string" } },
                        "disabledTools": { "type": "array", "items": { "type": "string" } }
                    }
                }
            }
        }
    })
}

#[derive(Debug, Clone)]
struct McpDefinitionValidationError {
    code: String,
    message: String,
    details: Vec<String>,
}

fn validate_mcp_servers_schema(value: &Value) -> Result<(), Vec<String>> {
    let mut errors = Vec::<String>::new();
    let Some(root) = value.as_object() else {
        return Err(vec!["root must be JSON object".to_string()]);
    };
    match root.get("version").and_then(Value::as_str) {
        Some(v) if v == MCP_SPEC_VERSION_SUPPORTED => {}
        Some(v) => errors.push(format!(
            "version must be '{}' (got '{}')",
            MCP_SPEC_VERSION_SUPPORTED, v
        )),
        None => errors.push("missing required field: version".to_string()),
    }
    let servers = match root.get("mcpServers").and_then(Value::as_object) {
        Some(v) if !v.is_empty() => v,
        Some(_) => {
            errors.push("mcpServers is empty".to_string());
            return Err(errors);
        }
        None => {
            errors.push("missing required field: mcpServers".to_string());
            return Err(errors);
        }
    };

    for (name, node) in servers {
        let Some(server_obj) = node.as_object() else {
            errors.push(format!("mcpServers.{name} must be object"));
            continue;
        };
        let has_command = server_obj
            .get("command")
            .and_then(Value::as_str)
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let has_url = server_obj
            .get("url")
            .and_then(Value::as_str)
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !has_command && !has_url {
            errors.push(format!(
                "mcpServers.{name} must include either non-empty command or url"
            ));
        }
        if let Some(args) = server_obj.get("args") {
            if !args.is_array() {
                errors.push(format!("mcpServers.{name}.args must be array"));
            } else if args
                .as_array()
                .map(|items| items.iter().any(|v| !v.is_string()))
                .unwrap_or(false)
            {
                errors.push(format!("mcpServers.{name}.args must be string array"));
            }
        }
        for map_key in ["env", "httpHeaders", "envHttpHeaders"] {
            if let Some(map_value) = server_obj.get(map_key) {
                let Some(map) = map_value.as_object() else {
                    errors.push(format!("mcpServers.{name}.{map_key} must be object"));
                    continue;
                };
                if map.values().any(|v| !v.is_string()) {
                    errors.push(format!(
                        "mcpServers.{name}.{map_key} values must be strings"
                    ));
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn normalize_mcp_definition_for_validation(
    definition_json: &str,
) -> Result<(Value, Option<String>), McpDefinitionValidationError> {
    let parsed: Value = serde_json::from_str(definition_json).map_err(|err| McpDefinitionValidationError {
        code: "invalid_json".to_string(),
        message: format!("MCP definition JSON parse failed: {err}"),
        details: vec!["input must be valid JSON object".to_string()],
    })?;
    let mut root = parsed.as_object().cloned().ok_or_else(|| McpDefinitionValidationError {
        code: "invalid_root".to_string(),
        message: "MCP definition must be a JSON object".to_string(),
        details: vec!["root JSON type must be object".to_string()],
    })?;
    let mut migrated: Option<String> = None;

    match root.get("version").and_then(Value::as_str) {
        Some(v) if v == MCP_SPEC_VERSION_SUPPORTED => {}
        Some(v) if v.starts_with("0.") => {
            root.insert(
                "version".to_string(),
                Value::String(MCP_SPEC_VERSION_SUPPORTED.to_string()),
            );
            let migrated_value = Value::Object(root.clone());
            migrated = Some(
                serde_json::to_string_pretty(&migrated_value).unwrap_or_else(|_| definition_json.to_string()),
            );
        }
        Some(v) => {
            return Err(McpDefinitionValidationError {
                code: "unsupported_version".to_string(),
                message: format!(
                    "Unsupported MCP definition version '{}', expected '{}'",
                    v, MCP_SPEC_VERSION_SUPPORTED
                ),
                details: vec![
                    "upgrade definition to the supported version".to_string(),
                    "or provide a v0.x format that can be auto-migrated".to_string(),
                ],
            });
        }
        None => {
            root.insert(
                "version".to_string(),
                Value::String(MCP_SPEC_VERSION_SUPPORTED.to_string()),
            );
            if !root.contains_key("mcpServers") {
                let server_name = value_get_string(&Value::Object(root.clone()), "name")
                    .unwrap_or_else(|| "mcp-server".to_string());
                let mut legacy_server = root.clone();
                legacy_server.remove("version");
                let mut servers_map = serde_json::Map::new();
                servers_map.insert(server_name, Value::Object(legacy_server));
                root.insert("mcpServers".to_string(), Value::Object(servers_map));
            }
            let migrated_value = Value::Object(root.clone());
            migrated = Some(
                serde_json::to_string_pretty(&migrated_value).unwrap_or_else(|_| definition_json.to_string()),
            );
        }
    }

    let normalized = Value::Object(root);
    if let Err(details) = validate_mcp_servers_schema(&normalized) {
        return Err(McpDefinitionValidationError {
            code: "schema_validation_failed".to_string(),
            message: "MCP definition does not match required schema".to_string(),
            details,
        });
    }

    Ok((normalized, migrated))
}

fn value_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value
        .get(key)
        .or_else(|| value.get(&key.to_ascii_lowercase()))
        .or_else(|| {
            let snake = key
                .chars()
                .enumerate()
                .flat_map(|(idx, ch)| {
                    if ch.is_ascii_uppercase() {
                        if idx == 0 {
                            vec![ch.to_ascii_lowercase()]
                        } else {
                            vec!['_', ch.to_ascii_lowercase()]
                        }
                    } else {
                        vec![ch]
                    }
                })
                .collect::<String>();
            value.get(&snake)
        })
}

fn value_get_string(value: &Value, key: &str) -> Option<String> {
    value_get(value, key)
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn value_get_map_string_string(
    value: &Value,
    key: &str,
) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::<String, String>::new();
    let Some(map) = value_get(value, key).and_then(Value::as_object) else {
        return out;
    };
    for (k, v) in map {
        if let Some(text) = v.as_str() {
            let name = k.trim();
            let value = text.trim();
            if !name.is_empty() && !value.is_empty() {
                out.insert(name.to_string(), value.to_string());
            }
        }
    }
    out
}

fn value_get_string_array(value: &Value, key: &str) -> Vec<String> {
    value_get(value, key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_mcp_root_object(definition_json: &str) -> Result<(String, Value), String> {
    let parsed: Value = serde_json::from_str(definition_json)
        .map_err(|err| format!("MCP definition JSON parse failed: {err}"))?;
    let object = parsed
        .as_object()
        .ok_or_else(|| "MCP definition must be a JSON object".to_string())?;

    if let Some(servers) = object.get("mcpServers").and_then(Value::as_object) {
        if servers.is_empty() {
            return Err("mcpServers is empty".to_string());
        }
        let (name, node) = servers
            .iter()
            .next()
            .ok_or_else(|| "mcpServers is empty".to_string())?;
        return Ok((name.clone(), node.clone()));
    }

    let name = value_get_string(&parsed, "name").unwrap_or_else(|| "mcp-server".to_string());
    Ok((name, parsed))
}

fn parse_mcp_server_definition(definition_json: &str) -> Result<(String, ParsedMcpServerDefinition), String> {
    let (server_name, root) = parse_mcp_root_object(definition_json)?;

    let transport_text = value_get_string(&root, "transport")
        .or_else(|| value_get_string(&root, "type"))
        .unwrap_or_default()
        .to_ascii_lowercase();

    let command = value_get_string(&root, "command");
    let url = value_get_string(&root, "url");

    let transport = if matches!(
        transport_text.as_str(),
        "streamable_http" | "streamable-http" | "http" | "https" | "remote"
    ) {
        McpTransportKind::StreamableHttp
    } else if transport_text == "stdio" || transport_text == "local" {
        McpTransportKind::Stdio
    } else if command.is_some() {
        McpTransportKind::Stdio
    } else if url.is_some() {
        McpTransportKind::StreamableHttp
    } else {
        return Err("MCP definition must include either command(stdio) or url(streamable HTTP)".to_string());
    };

    let args = value_get_string_array(&root, "args");
    let env = value_get_map_string_string(&root, "env");
    let cwd = value_get_string(&root, "cwd");
    let bearer_token_env_var = value_get_string(&root, "bearerTokenEnvVar")
        .or_else(|| value_get_string(&root, "bearer_token_env_var"));
    let http_headers = value_get_map_string_string(&root, "httpHeaders");
    let env_http_headers = value_get_map_string_string(&root, "envHttpHeaders");

    match transport {
        McpTransportKind::Stdio => {
            if command.as_deref().unwrap_or_default().trim().is_empty() {
                return Err("stdio MCP definition requires command".to_string());
            }
        }
        McpTransportKind::StreamableHttp => {
            if url.as_deref().unwrap_or_default().trim().is_empty() {
                return Err("streamable HTTP MCP definition requires url".to_string());
            }
        }
    }

    Ok((
        server_name,
        ParsedMcpServerDefinition {
            transport,
            command,
            args,
            env,
            cwd,
            url,
            bearer_token_env_var,
            http_headers,
            env_http_headers,
        },
    ))
}

fn parse_mcp_server_definition_from_config(server: &McpServerConfig) -> Result<ParsedMcpServerDefinition, String> {
    let (_, parsed) = parse_mcp_server_definition(&server.definition_json)?;
    Ok(parsed)
}
