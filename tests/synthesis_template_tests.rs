use std::collections::BTreeMap;

use refsyn::synthesis_template::{
    ClassDefinitions, Declaration, DeclarationValue, Environment, HoleId, Object, ObjectId,
    Statement, Template, Value, ValueType, VertexId, Write,
};

fn number(value: i64) -> Value {
    Value::Number(value.into())
}

fn classes() -> ClassDefinitions {
    BTreeMap::from([(
        "Node".into(),
        BTreeMap::from([
            ("val".into(), Value::Undefined),
            ("next".into(), Value::Null),
        ]),
    )])
}

fn hole(vertex: usize, id: usize, value_type: ValueType) -> Declaration {
    Declaration {
        vertex: VertexId(vertex),
        value: DeclarationValue::Hole {
            id: HoleId(id),
            value_type,
        },
    }
}

fn create(vertex: usize) -> Declaration {
    Declaration {
        vertex: VertexId(vertex),
        value: DeclarationValue::Create("Node".into()),
    }
}

fn write(from: usize, field: &str, to: usize) -> Write {
    Write {
        from: VertexId(from),
        field: field.into(),
        to: VertexId(to),
    }
}

fn insert() -> Template {
    Template::new(
        vec![
            hole(0, 0, ValueType::Reference("Node".into())),
            hole(1, 1, ValueType::Reference("Node".into())),
            create(2),
            hole(3, 2, ValueType::Number),
        ],
        vec![write(2, "val", 3), write(0, "next", 2), write(2, "next", 1)],
        vec![HoleId(2), HoleId(0), HoleId(1)],
    )
    .unwrap()
}

fn input() -> Environment {
    Environment {
        objects: vec![
            Object {
                class: "Node".into(),
                fields: BTreeMap::from([
                    ("val".into(), number(39)),
                    ("next".into(), Value::Reference(ObjectId(1))),
                ]),
            },
            Object {
                class: "Node".into(),
                fields: BTreeMap::from([("val".into(), number(27)), ("next".into(), Value::Null)]),
            },
        ],
        bindings: BTreeMap::from([
            ("this".into(), Value::Reference(ObjectId(0))),
            ("arg".into(), number(89)),
            ("alias".into(), Value::Reference(ObjectId(1))),
        ]),
    }
}

#[test]
fn insert_lowers_to_seven_statements_in_the_paper_order() {
    assert_eq!(
        insert().statements(),
        &[
            Statement::Create {
                vertex: VertexId(2),
                class: "Node".into()
            },
            Statement::BindHole {
                vertex: VertexId(3),
                hole: HoleId(2),
                value_type: ValueType::Number
            },
            Statement::BindHole {
                vertex: VertexId(0),
                hole: HoleId(0),
                value_type: ValueType::Reference("Node".into())
            },
            Statement::BindHole {
                vertex: VertexId(1),
                hole: HoleId(1),
                value_type: ValueType::Reference("Node".into())
            },
            Statement::Write(write(2, "val", 3)),
            Statement::Write(write(0, "next", 2)),
            Statement::Write(write(2, "next", 1)),
        ]
    );
}

#[test]
fn insert_hole_inputs_include_allocation_and_only_prior_hole_results() {
    let before = input();
    let mut inputs = Vec::new();
    let bound = insert()
        .bind_holes(before.clone(), &classes(), |id, _, environment| {
            inputs.push((id, environment.clone()));
            Ok(match id {
                HoleId(2) => environment.call.bindings["arg"].clone(),
                HoleId(0) => environment.call.bindings["this"].clone(),
                HoleId(1) => {
                    let Value::Reference(prev) = environment.value(VertexId(0))? else {
                        panic!("expected prev reference")
                    };
                    environment.call.objects[prev.0].fields["next"].clone()
                }
                _ => panic!("unexpected hole"),
            })
        })
        .unwrap();
    assert_eq!(
        inputs.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        vec![HoleId(2), HoleId(0), HoleId(1)]
    );
    for (_, environment) in &inputs {
        assert_eq!(&environment.call.objects[..2], before.objects.as_slice());
        assert_eq!(environment.call.bindings, before.bindings);
        assert_eq!(
            environment.call.objects[2],
            Object {
                class: "Node".into(),
                fields: classes()["Node"].clone()
            }
        );
        assert_eq!(
            environment.vertices[&VertexId(2)],
            Value::Reference(ObjectId(2))
        );
    }
    assert_eq!(
        inputs[0].1.vertices,
        BTreeMap::from([(VertexId(2), Value::Reference(ObjectId(2)))])
    );
    assert_eq!(
        inputs[1].1.vertices,
        BTreeMap::from([
            (VertexId(2), Value::Reference(ObjectId(2))),
            (VertexId(3), number(89))
        ])
    );
    assert_eq!(
        inputs[2].1.vertices,
        BTreeMap::from([
            (VertexId(2), Value::Reference(ObjectId(2))),
            (VertexId(3), number(89)),
            (VertexId(0), Value::Reference(ObjectId(0)))
        ])
    );
    assert_eq!(
        bound.value(VertexId(1)).unwrap(),
        &Value::Reference(ObjectId(1))
    );
}

#[test]
fn insert_preserves_successor_identity_and_external_aliases() {
    let before = input();
    let after = insert()
        .execute(before.clone(), &classes(), |id, _, environment| {
            Ok(match id {
                HoleId(2) => environment.call.bindings["arg"].clone(),
                HoleId(0) => environment.call.bindings["this"].clone(),
                HoleId(1) => environment.call.objects[0].fields["next"].clone(),
                _ => panic!("unexpected hole"),
            })
        })
        .unwrap();
    let mut expected = before;
    expected.objects[0]
        .fields
        .insert("next".into(), Value::Reference(ObjectId(2)));
    expected.objects.push(Object {
        class: "Node".into(),
        fields: BTreeMap::from([
            ("val".into(), number(89)),
            ("next".into(), Value::Reference(ObjectId(1))),
        ]),
    });
    assert_eq!(after.call, expected);
}

#[test]
fn every_allocation_precedes_the_first_hole_and_uses_declaration_order() {
    let template = Template::new(
        vec![
            hole(0, 0, ValueType::Reference("Node".into())),
            create(9),
            create(4),
        ],
        vec![write(9, "next", 4)],
        vec![HoleId(0)],
    )
    .unwrap();
    let after = template
        .execute(input(), &classes(), |_, _, environment| {
            assert_eq!(environment.call.objects.len(), 4);
            assert_eq!(
                environment.value(VertexId(9))?,
                &Value::Reference(ObjectId(2))
            );
            assert_eq!(
                environment.value(VertexId(4))?,
                &Value::Reference(ObjectId(3))
            );
            assert_eq!(environment.call.objects[2].fields["next"], Value::Null);
            Ok(environment.value(VertexId(4))?.clone())
        })
        .unwrap();
    assert_eq!(
        after.call.objects[2].fields["next"],
        Value::Reference(ObjectId(3))
    );
}

#[test]
fn shared_vertices_are_evaluated_once_and_repeated_writes_keep_their_order() {
    let template = Template::new(
        vec![
            hole(0, 0, ValueType::Reference("Node".into())),
            hole(1, 1, ValueType::Number),
            hole(2, 2, ValueType::Number),
        ],
        vec![write(0, "val", 1), write(0, "val", 2), write(0, "val", 1)],
        vec![HoleId(0), HoleId(1), HoleId(2)],
    )
    .unwrap();
    assert_eq!(
        &template.statements()[3..],
        &[
            Statement::Write(write(0, "val", 1)),
            Statement::Write(write(0, "val", 2)),
            Statement::Write(write(0, "val", 1))
        ]
    );
    let mut calls = Vec::new();
    let after = template
        .execute(input(), &classes(), |id, _, _| {
            calls.push(id);
            Ok(match id {
                HoleId(0) => Value::Reference(ObjectId(0)),
                HoleId(1) => number(101),
                HoleId(2) => number(202),
                _ => unreachable!(),
            })
        })
        .unwrap();
    assert_eq!(calls, vec![HoleId(0), HoleId(1), HoleId(2)]);
    assert_eq!(after.call.objects[0].fields["val"], number(101));
}

#[test]
fn null_vertices_write_null_without_allocating_or_evaluating_a_hole() {
    let template = Template::new(
        vec![
            create(0),
            Declaration {
                vertex: VertexId(1),
                value: DeclarationValue::Null,
            },
        ],
        vec![write(0, "next", 1)],
        vec![],
    )
    .unwrap();
    let after = template
        .execute(input(), &classes(), |_, _, _| panic!("null is not a hole"))
        .unwrap();
    assert_eq!(after.call.objects.len(), 3);
    assert_eq!(after.call.objects[2].fields["next"], Value::Null);
}

#[test]
fn invalid_vertex_references_and_hole_orders_are_rejected() {
    let declarations = vec![create(0), hole(1, 0, ValueType::Number)];
    assert!(Template::new(
        declarations.clone(),
        vec![write(0, "val", 9)],
        vec![HoleId(0)]
    )
    .is_err());
    assert!(Template::new(declarations.clone(), vec![], vec![]).is_err());
    assert!(Template::new(declarations.clone(), vec![], vec![HoleId(0), HoleId(0)]).is_err());
    assert!(Template::new(declarations.clone(), vec![], vec![HoleId(9)]).is_err());
    assert!(Template::new(vec![create(0), create(0)], vec![], vec![]).is_err());
    assert!(Template::new(
        vec![hole(0, 0, ValueType::Number), hole(1, 0, ValueType::Number)],
        vec![],
        vec![HoleId(0)]
    )
    .is_err());
    assert!(Template::new(declarations, vec![write(1, "val", 0)], vec![HoleId(0)]).is_err());
}

#[test]
fn invalid_heap_references_and_incompatible_hole_results_are_rejected() {
    for value in [Value::Reference(ObjectId(99)), number(1)] {
        let template = Template::new(
            vec![hole(0, 0, ValueType::Reference("Node".into()))],
            vec![],
            vec![HoleId(0)],
        )
        .unwrap();
        assert!(template
            .bind_holes(input(), &classes(), |_, _, _| Ok(value.clone()))
            .is_err());
    }
    let mut environment = input();
    environment
        .bindings
        .insert("bad".into(), Value::Reference(ObjectId(99)));
    assert!(insert()
        .bind_holes(environment, &classes(), |_, _, _| panic!(
            "invalid input must be rejected before evaluation"
        ))
        .is_err());
    assert!(insert()
        .bind_holes(input(), &BTreeMap::new(), |_, _, _| panic!(
            "missing class must be rejected before evaluation"
        ))
        .is_err());
}

#[test]
fn swapping_values_reads_both_original_values_before_either_write() {
    let template = Template::new(
        vec![
            hole(0, 0, ValueType::Reference("Node".into())),
            hole(1, 1, ValueType::Reference("Node".into())),
            hole(2, 2, ValueType::Number),
            hole(3, 3, ValueType::Number),
        ],
        vec![write(0, "val", 3), write(1, "val", 2)],
        vec![HoleId(0), HoleId(1), HoleId(2), HoleId(3)],
    )
    .unwrap();
    let before = input();
    let after = template
        .execute(before.clone(), &classes(), |id, _, environment| {
            Ok(match id {
                HoleId(0) => Value::Reference(ObjectId(0)),
                HoleId(1) => Value::Reference(ObjectId(1)),
                HoleId(2) | HoleId(3) => {
                    assert_eq!(environment.call, before);
                    let vertex = if id == HoleId(2) {
                        VertexId(0)
                    } else {
                        VertexId(1)
                    };
                    let Value::Reference(object) = environment.value(vertex)? else {
                        panic!("expected object")
                    };
                    environment.call.objects[object.0].fields["val"].clone()
                }
                _ => unreachable!(),
            })
        })
        .unwrap();
    let mut expected = before;
    expected.objects[0].fields.insert("val".into(), number(27));
    expected.objects[1].fields.insert("val".into(), number(39));
    assert_eq!(after.call, expected);
}

#[test]
fn class_and_field_names_come_from_the_template_and_class_definition() {
    let template = Template::new(
        vec![
            Declaration {
                vertex: VertexId(0),
                value: DeclarationValue::Create("Cell".into()),
            },
            hole(1, 0, ValueType::String),
        ],
        vec![write(0, "payload", 1)],
        vec![HoleId(0)],
    )
    .unwrap();
    let definitions = BTreeMap::from([(
        "Cell".into(),
        BTreeMap::from([
            ("payload".into(), Value::Undefined),
            ("link".into(), Value::Null),
        ]),
    )]);
    let after = template
        .execute(input(), &definitions, |_, _, environment| {
            assert_eq!(environment.call.objects[2].class, "Cell");
            assert_eq!(
                environment.call.objects[2].fields["payload"],
                Value::Undefined
            );
            assert_eq!(environment.call.objects[2].fields["link"], Value::Null);
            Ok(Value::String("text".into()))
        })
        .unwrap();
    assert_eq!(
        after.call.objects[2],
        Object {
            class: "Cell".into(),
            fields: BTreeMap::from([
                ("payload".into(), Value::String("text".into())),
                ("link".into(), Value::Null)
            ])
        }
    );
}
