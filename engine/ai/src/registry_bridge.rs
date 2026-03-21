//! Translates `McpCommand` entries to MCP tool descriptors for `tools/list`.

use crate::McpCommand;
use serde_json::{json, Value};

/// MCP tool descriptor (one entry in `tools/list` response).
#[derive(Debug, Clone, serde::Serialize)]
pub struct McpTool {
    /// Name of the tool (typically the command id).
    pub name: String,
    /// Human-readable description of what the tool does.
    pub description: String,
    /// JSON Schema describing the tool's input parameters.
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Derive the permission category from a command id's namespace.
///
/// | Namespace | Category |
/// |-----------|----------|
/// | `scene.*` | `scene` |
/// | `viewport.*` | `viewport` |
/// | `project.*` | `build` |
/// | `module.*` | `modules` |
/// | anything else | `read` |
///
/// # Example
///
/// ```
/// use engine_ai::registry_bridge::namespace_to_category;
///
/// assert_eq!(namespace_to_category("scene.create_entity"), "scene");
/// assert_eq!(namespace_to_category("project.build"), "build");
/// assert_eq!(namespace_to_category("editor.get_state"), "read");
/// ```
pub fn namespace_to_category(command_id: &str) -> &'static str {
    if command_id.starts_with("scene.") {
        return "scene";
    }
    if command_id.starts_with("viewport.") {
        return "viewport";
    }
    if command_id.starts_with("project.") {
        return "build";
    }
    if command_id.starts_with("module.") {
        return "modules";
    }
    "read"
}

/// Convert a single `McpCommand` to an MCP tool descriptor.
///
/// Uses the command's `args_schema` if provided; otherwise defaults to an empty
/// object schema `{ "type": "object", "properties": {} }`.
/// The description comes from the command's optional `description` field, falling back
/// to the `label` if not set.
///
/// # Example
///
/// ```
/// use engine_ai::{McpCommand, registry_bridge::command_to_mcp_tool};
/// use serde_json::json;
///
/// let cmd = McpCommand {
///     id: "scene.create_entity".into(),
///     label: "Create Entity".into(),
///     category: "scene".into(),
///     description: Some("Creates a new entity in the scene".into()),
///     args_schema: Some(json!({
///         "type": "object",
///         "properties": {
///             "name": { "type": "string" }
///         }
///     })),
///     returns_data: false,
/// };
///
/// let tool = command_to_mcp_tool(&cmd);
/// assert_eq!(tool.name, "scene.create_entity");
/// assert_eq!(tool.description, "Creates a new entity in the scene");
/// ```
pub fn command_to_mcp_tool(cmd: &McpCommand) -> McpTool {
    let input_schema = cmd.args_schema.clone().unwrap_or_else(|| {
        json!({ "type": "object", "properties": {} })
    });
    McpTool {
        name: cmd.id.clone(),
        description: cmd
            .description
            .clone()
            .unwrap_or_else(|| cmd.label.clone()),
        input_schema,
    }
}

/// Convert a full registry snapshot to the `tools/list` result array.
///
/// Each command in the slice becomes one entry in the resulting `Vec<McpTool>`.
///
/// # Example
///
/// ```
/// use engine_ai::{McpCommand, registry_bridge::commands_to_tools};
///
/// let cmds = vec![
///     McpCommand {
///         id: "scene.create_entity".into(),
///         label: "Create Entity".into(),
///         category: "scene".into(),
///         description: None,
///         args_schema: None,
///         returns_data: false,
///     },
/// ];
///
/// let tools = commands_to_tools(&cmds);
/// assert_eq!(tools.len(), 1);
/// assert_eq!(tools[0].name, "scene.create_entity");
/// ```
pub fn commands_to_tools(commands: &[McpCommand]) -> Vec<McpTool> {
    commands.iter().map(command_to_mcp_tool).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cmd(id: &str, schema: Option<Value>) -> McpCommand {
        McpCommand {
            id: id.into(),
            label: "Test".into(),
            category: "test".into(),
            description: Some("A test command".into()),
            args_schema: schema,
            returns_data: false,
        }
    }

    #[test]
    fn namespace_to_category_scene() {
        assert_eq!(namespace_to_category("scene.create_entity"), "scene");
    }

    #[test]
    fn namespace_to_category_viewport() {
        assert_eq!(namespace_to_category("viewport.screenshot"), "viewport");
    }

    #[test]
    fn namespace_to_category_project() {
        assert_eq!(namespace_to_category("project.build"), "build");
        assert_eq!(namespace_to_category("project.add_module"), "build");
    }

    #[test]
    fn namespace_to_category_module() {
        assert_eq!(
            namespace_to_category("module.physics.add_rigidbody"),
            "modules"
        );
    }

    #[test]
    fn namespace_to_category_editor_and_template_fall_back_to_read() {
        // editor.* and template.* are intentionally in the "read" category (fallback)
        assert_eq!(namespace_to_category("editor.get_scene_state"), "read");
        assert_eq!(namespace_to_category("template.open"), "read");
        assert_eq!(namespace_to_category("unknowncommand"), "read");
    }

    #[test]
    fn command_to_mcp_tool_passthrough_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": { "name": { "type": "string" } }
        });
        let cmd = make_cmd("scene.create_entity", Some(schema.clone()));
        let tool = command_to_mcp_tool(&cmd);
        assert_eq!(tool.name, "scene.create_entity");
        assert_eq!(tool.input_schema, schema);
    }

    #[test]
    fn command_to_mcp_tool_no_schema_gives_empty_object() {
        let cmd = make_cmd("viewport.screenshot", None);
        let tool = command_to_mcp_tool(&cmd);
        assert_eq!(tool.input_schema["type"], "object");
        assert!(tool.input_schema["properties"].is_object());
    }

    #[test]
    fn commands_to_tools_preserves_count() {
        let cmds = vec![
            make_cmd("scene.create_entity", None),
            make_cmd("viewport.screenshot", None),
        ];
        let tools = commands_to_tools(&cmds);
        assert_eq!(tools.len(), 2);
    }
}
