use proc_macro::{self, TokenStream};
use proc_macro2::Ident;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(NodeEventEmitter)]
pub fn derive_node_event_emitter(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);
    let struct_name = &ident;
    let event_struct_name = Ident::new(&format!("{}Event", struct_name), struct_name.span());

    let output = match data {
        syn::Data::Struct(data_struct) => match data_struct.fields {
            syn::Fields::Unit => {
                quote! {
                    #[derive(Event, Reflect, Default, Clone)]
                    #[reflect(Event)]
                    pub struct #event_struct_name;

                    impl NodeEventEmitter for #struct_name {
                        fn make(&self, _actors: &[Actor]) -> Box<dyn Reflect> {
                            Box::from(#event_struct_name)
                        }
                    }
                }
            }
            syn::Fields::Named(fs) => {
                let field_names: Vec<Ident> =
                    fs.named.iter().map(|f| f.ident.clone().unwrap()).collect();
                let field_types: Vec<&syn::Type> = fs.named.iter().map(|f| &f.ty).collect();

                quote! {
                    #[derive(Event, Reflect, Default, Clone)]
                    #[reflect(Event)]
                    pub struct #event_struct_name {
                        actors: Vec<String>,
                        #( #field_names: #field_types, )*
                    }

                    impl NodeEventEmitter for #struct_name {
                        fn make(&self, actors: &[Actor]) -> Box<dyn Reflect> {
                            Box::from(#event_struct_name {
                                actors: actors.iter().map(|a| a.name.clone()).collect(),
                                #( #field_names: self.#field_names.clone(), )*
                            })
                        }
                    }
                }
            }
            syn::Fields::Unnamed(_) => {
                quote! {
                    compile_error!("NodeEventEmitter can only be derived for structs with named fields. Tuple structs are not supported yet.");
                }
            }
        },
        syn::Data::Enum(_) => quote! {
            compile_error!("NodeEventEmitter can only be derived for structs. Enums and unions are not supported yet.");
        },
        syn::Data::Union(_) => quote! {
            compile_error!("NodeEventEmitter can only be derived for structs. Enums and unions are not supported yet.");
        },
    };

    output.into()
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_talks::prelude::*;

    #[derive(NodeEventEmitter, Component)]
    struct TestEmpty;

    #[derive(NodeEventEmitter, Component)]
    struct TestNamed {
        field1: bool,
        field2: i32,
    }

    #[test]
    fn test_empty_struct() {
        let empty = TestEmpty;
        let boxed_event = empty.make(&[]);
        assert!(boxed_event.is::<TestEmptyEvent>());
    }

    #[test]
    fn test_named_struct() {
        let named = TestNamed {
            field1: true,
            field2: 42,
        };
        let boxed_event = named.make(&[]);
        assert!(boxed_event.is::<TestNamedEvent>());
        let event = boxed_event.downcast_ref::<TestNamedEvent>().unwrap();
        assert_eq!(event.field1, true);
        assert_eq!(event.field2, 42);
    }

    #[test]
    fn test_named_has_actors() {
        let named = TestNamed {
            field1: true,
            field2: 42,
        };

        let boxed_event = named.make(&[Actor::new("actor", "Actor")]);
        assert!(boxed_event.is::<TestNamedEvent>());
        let event = boxed_event.downcast_ref::<TestNamedEvent>().unwrap();
        assert_eq!(event.actors.len(), 1);
        assert_eq!(event.actors[0], "Actor");
    }
}
