use engine_ops::undo::*;
use serde_json::json;

#[test]
fn set_component_undo_restores_old_value() {
    let mut stack = UndoStack::new(100);
    let action = EditorAction::SetComponent {
        entity: 1,
        type_name: "Health".into(),
        old: json!(100),
        new: json!(50),
    };
    stack.push(action);

    let undone = stack.undo().unwrap();
    match undone {
        EditorAction::SetComponent { old, new, .. } => {
            assert_eq!(old, json!(100));
            assert_eq!(new, json!(50));
        }
        _ => panic!("Expected SetComponent"),
    }
}

#[test]
fn create_entity_undo_returns_action() {
    let mut stack = UndoStack::new(100);
    stack.push(EditorAction::CreateEntity { id: 42, name: None });

    let undone = stack.undo().unwrap();
    match undone {
        EditorAction::CreateEntity { id, .. } => assert_eq!(id, 42),
        _ => panic!("Expected CreateEntity"),
    }
    assert!(!stack.can_undo());
}

#[test]
fn delete_entity_undo_restores_snapshot() {
    let mut stack = UndoStack::new(100);
    let snapshot = EntitySnapshot {
        id: 7,
        name: None,
        components: vec![
            ("Health".into(), json!(100)),
            ("Position".into(), json!({"x": 1.0, "y": 2.0})),
        ],
    };
    stack.push(EditorAction::DeleteEntity {
        id: 7,
        snapshot: snapshot.clone(),
    });

    let undone = stack.undo().unwrap();
    match undone {
        EditorAction::DeleteEntity { id, snapshot } => {
            assert_eq!(id, 7);
            assert_eq!(snapshot.components.len(), 2);
            assert_eq!(snapshot.components[0].0, "Health");
            assert_eq!(snapshot.components[1].1, json!({"x": 1.0, "y": 2.0}));
        }
        _ => panic!("Expected DeleteEntity"),
    }
}

#[test]
fn redo_after_undo_reapplies_action() {
    let mut stack = UndoStack::new(100);
    stack.push(EditorAction::SetComponent {
        entity: 1,
        type_name: "Health".into(),
        old: json!(100),
        new: json!(50),
    });

    stack.undo().unwrap();
    assert!(stack.can_redo());

    let redone = stack.redo().unwrap();
    match redone {
        EditorAction::SetComponent { new, .. } => assert_eq!(new, json!(50)),
        _ => panic!("Expected SetComponent"),
    }
    assert!(stack.can_undo());
    assert!(!stack.can_redo());
}

#[test]
fn new_action_clears_redo_stack() {
    let mut stack = UndoStack::new(100);
    stack.push(EditorAction::CreateEntity { id: 1, name: None });
    stack.push(EditorAction::CreateEntity { id: 2, name: None });

    stack.undo().unwrap();
    assert!(stack.can_redo());

    // Pushing a new action should clear redo
    stack.push(EditorAction::CreateEntity { id: 3, name: None });
    assert!(!stack.can_redo());
}

#[test]
fn batch_undo_reverses_all_in_one_step() {
    let mut stack = UndoStack::new(100);
    let batch = EditorAction::Batch {
        label: "Move and resize".into(),
        actions: vec![
            EditorAction::SetComponent {
                entity: 1,
                type_name: "Position".into(),
                old: json!({"x": 0}),
                new: json!({"x": 10}),
            },
            EditorAction::SetComponent {
                entity: 1,
                type_name: "Scale".into(),
                old: json!(1.0),
                new: json!(2.0),
            },
        ],
    };
    stack.push(batch);

    let undone = stack.undo().unwrap();
    match undone {
        EditorAction::Batch { label, actions } => {
            assert_eq!(label, "Move and resize");
            assert_eq!(actions.len(), 2);
        }
        _ => panic!("Expected Batch"),
    }
}

#[test]
fn max_depth_evicts_oldest() {
    let mut stack = UndoStack::new(3);
    stack.push(EditorAction::CreateEntity { id: 1, name: None });
    stack.push(EditorAction::CreateEntity { id: 2, name: None });
    stack.push(EditorAction::CreateEntity { id: 3, name: None });
    stack.push(EditorAction::CreateEntity { id: 4, name: None });

    // Only 3 should remain; id=1 was evicted
    let a1 = stack.undo().unwrap();
    let a2 = stack.undo().unwrap();
    let a3 = stack.undo().unwrap();
    assert!(stack.undo().is_err());

    match a1 {
        EditorAction::CreateEntity { id, .. } => assert_eq!(id, 4),
        _ => panic!("Expected CreateEntity"),
    }
    match a2 {
        EditorAction::CreateEntity { id, .. } => assert_eq!(id, 3),
        _ => panic!("Expected CreateEntity"),
    }
    match a3 {
        EditorAction::CreateEntity { id, .. } => assert_eq!(id, 2),
        _ => panic!("Expected CreateEntity"),
    }
}

#[test]
fn empty_undo_redo_returns_error() {
    let mut stack = UndoStack::new(100);
    assert!(!stack.can_undo());
    assert!(!stack.can_redo());
    assert!(stack.undo().is_err());
    assert!(stack.redo().is_err());
}
