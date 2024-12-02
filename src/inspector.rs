use crate::prelude::*;
use bevy_ecs::prelude::*;
use bevy_reflect::{NamedField, PartialReflect, ReflectFromPtr};
use bevy_text::prelude::*;
use bevy_ui::prelude::*;

#[derive(Data)]
struct FieldItem {
    field: NamedField,
    reflect: Box<dyn PartialReflect>,
}

#[derive(Data)]
struct Item {
    name: String,
    fields: Vec<FieldItem>,
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[derive(Data)]
/// ECS inspector UI composable.
pub struct Inspector {}

impl Compose for Inspector {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let resources = use_mut(&cx, Vec::<Item>::new);

        use_world(&cx, move |world: &World| {
            let mut new_resources = Vec::new();

            for (info, ptr) in world.iter_resources() {
                let type_registry = world.resource::<AppTypeRegistry>().read();

                let Some(type_id) = info.type_id() else {
                    continue;
                };

                let Some(registration) = type_registry.get(type_id) else {
                    continue;
                };

                let reflect_from_ptr = registration.data::<ReflectFromPtr>().unwrap();
                let reflect = unsafe { reflect_from_ptr.as_reflect(ptr) };

                let mut fields = Vec::new();

                match reflect.reflect_ref() {
                    bevy_reflect::ReflectRef::Struct(dyn_struct) => {
                        let info = dyn_struct.get_represented_struct_info().unwrap();
                        for (field_info, field) in info.iter().zip(dyn_struct.iter_fields()) {
                            fields.push(FieldItem {
                                field: field_info.clone(),
                                reflect: field.clone_value(),
                            });
                            field.clone_value();
                        }
                    }
                    _ => {}
                }

                new_resources.push(Item {
                    name: info.name().to_owned(),
                    fields,
                });
            }

            SignalMut::set_if_neq(resources, new_resources);
        });

        spawn(Node {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        })
        .content((
            spawn(Text::new("Resources")),
            compose::from_iter(resources, |item| {
                spawn(Node {
                    flex_direction: FlexDirection::Column,
                    margin: UiRect::left(Val::Px(10.)),
                    ..Default::default()
                })
                .content((
                    spawn((
                        Text::new(item.name.to_string()),
                        TextFont {
                            font_size: 12.,
                            ..Default::default()
                        },
                        Node {
                            margin: UiRect::left(Val::Px(10.)),
                            ..Default::default()
                        },
                    )),
                    compose::from_iter(Signal::map(item, |i| &i.fields), |item| {
                        spawn((
                            Text::new(format!("{}: {:?}", item.field.name(), item.reflect)),
                            TextFont {
                                font_size: 10.,
                                ..Default::default()
                            },
                            Node {
                                margin: UiRect::left(Val::Px(20.)),
                                ..Default::default()
                            },
                        ))
                    }),
                ))
            }),
        ))
    }
}
