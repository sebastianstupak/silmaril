use engine_ops::command::TemplateCommand;
use engine_ops::processor::CommandProcessor;
use serde_json::json;
use tempfile::tempdir;

fn make_processor() -> (CommandProcessor, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.yaml");
    // Create an empty template YAML on disk so load_yaml can read it
    std::fs::write(&path, "name: test\nentities: []\n").unwrap();
    let proc = CommandProcessor::load(path).unwrap();
    (proc, dir)
}

#[test]
fn create_entity_adds_to_state() {
    let (mut proc, _dir) = make_processor();
    let result = proc.execute(TemplateCommand::CreateEntity { name: Some("Player".into()) }).unwrap();
    assert_eq!(result.new_state.entities.len(), 1);
    assert_eq!(result.new_state.entities[0].name.as_deref(), Some("Player"));
}

#[test]
fn create_entity_writes_yaml() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.yaml");
    std::fs::write(&path, "name: test\nentities: []\n").unwrap();
    let mut proc = CommandProcessor::load(path.clone()).unwrap();
    proc.execute(TemplateCommand::CreateEntity { name: Some("Hero".into()) }).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("Hero"), "YAML should contain entity name");
}

#[test]
fn undo_create_entity_removes_it() {
    let (mut proc, _dir) = make_processor();
    proc.execute(TemplateCommand::CreateEntity { name: Some("Temp".into()) }).unwrap();
    assert_eq!(proc.state_ref().entities.len(), 1);
    proc.undo().unwrap();
    assert_eq!(proc.state_ref().entities.len(), 0);
}

#[test]
fn undo_returns_none_when_empty() {
    let (mut proc, _dir) = make_processor();
    let result = proc.undo().unwrap();
    assert!(result.is_none(), "undo on empty stack should return Ok(None)");
}

#[test]
fn redo_returns_none_when_empty() {
    let (mut proc, _dir) = make_processor();
    let result = proc.redo().unwrap();
    assert!(result.is_none());
}

#[test]
fn undo_then_redo_restores_state() {
    let (mut proc, _dir) = make_processor();
    proc.execute(TemplateCommand::CreateEntity { name: Some("A".into()) }).unwrap();
    proc.undo().unwrap();
    assert!(proc.state_ref().entities.is_empty());
    proc.redo().unwrap();
    assert_eq!(proc.state_ref().entities.len(), 1);
}

#[test]
fn delete_entity_not_found_returns_error() {
    let (mut proc, _dir) = make_processor();
    let result = proc.execute(TemplateCommand::DeleteEntity { id: 999 });
    assert!(result.is_err());
}

#[test]
fn delete_entity_undo_restores_name_and_components() {
    let (mut proc, _dir) = make_processor();
    let create_result = proc.execute(TemplateCommand::CreateEntity { name: Some("Boss".into()) }).unwrap();
    let entity_id = create_result.new_state.entities[0].id;
    proc.execute(TemplateCommand::AddComponent {
        id: entity_id,
        type_name: "Health".into(),
        data: json!({"current": 100}),
    }).unwrap();
    proc.execute(TemplateCommand::DeleteEntity { id: entity_id }).unwrap();
    assert!(proc.state_ref().entities.is_empty());
    proc.undo().unwrap();  // undo delete
    let entity = proc.state_ref().find_entity(entity_id).unwrap();
    assert_eq!(entity.name.as_deref(), Some("Boss"));
    assert_eq!(entity.components.len(), 1);
}

#[test]
fn duplicate_entity_creates_copy() {
    let (mut proc, _dir) = make_processor();
    let result = proc.execute(TemplateCommand::CreateEntity { name: Some("Orig".into()) }).unwrap();
    let orig_id = result.new_state.entities[0].id;
    proc.execute(TemplateCommand::AddComponent {
        id: orig_id,
        type_name: "Health".into(),
        data: json!({"current": 50}),
    }).unwrap();
    proc.execute(TemplateCommand::DuplicateEntity { id: orig_id }).unwrap();
    assert_eq!(proc.state_ref().entities.len(), 2);
    let copy = &proc.state_ref().entities[1];
    assert_eq!(copy.components.len(), 1);
    assert_eq!(copy.components[0].type_name, "Health");
}

#[test]
fn duplicate_entity_undo_removes_copy_in_one_step() {
    let (mut proc, _dir) = make_processor();
    let result = proc.execute(TemplateCommand::CreateEntity { name: None }).unwrap();
    let id = result.new_state.entities[0].id;
    proc.execute(TemplateCommand::DuplicateEntity { id }).unwrap();
    assert_eq!(proc.state_ref().entities.len(), 2);
    proc.undo().unwrap();  // single undo removes the whole duplicate
    assert_eq!(proc.state_ref().entities.len(), 1);
}

#[test]
fn undo_history_persisted_and_loaded() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("level.yaml");
    std::fs::write(&path, "name: level\nentities: []\n").unwrap();
    {
        let mut proc = CommandProcessor::load(path.clone()).unwrap();
        proc.execute(TemplateCommand::CreateEntity { name: Some("Saved".into()) }).unwrap();
        // execute() calls write_state() which writes .undo.json
    } // proc is dropped here (not needed for the write)
    let undo_path = dir.path().join("level.undo.json");
    assert!(undo_path.exists(), ".undo.json should be written");
    // Reload and undo should work
    let mut proc2 = CommandProcessor::load(path).unwrap();
    assert_eq!(proc2.state_ref().entities.len(), 1);
    proc2.undo().unwrap();
    assert_eq!(proc2.state_ref().entities.len(), 0);
}

#[test]
fn history_summaries_returns_descriptions() {
    let (mut proc, _dir) = make_processor();
    proc.execute(TemplateCommand::CreateEntity { name: Some("X".into()) }).unwrap();
    let summaries = proc.history_summaries();
    assert_eq!(summaries.len(), 1);
    assert!(summaries[0].description.contains("Create Entity"));
}
