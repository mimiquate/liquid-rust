use liquid_core::Expression;
use liquid_core::ObjectView;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};
use std::collections::HashMap;

const KNOCK_TRANSLATIONS_VAR_NAME: &str = "knock_translations_config";
const NAMESPACE_SEPARATOR: char = ':';
const KEY_SEPARATOR: char = '.';

#[derive(Debug, FilterParameters)]
struct TranslateArgs {
    #[parameter(
        description = "Variables to be used for revaluating liquid once translation is resolved.",
        mode = "keyword_list"
    )]
    variables: HashMap<String, Expression>,
}

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "translate"]
struct TranslateFilter {
    #[parameters]
    args: TranslateArgs,
}

#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "t",
    description = "Access the desired translation for the recipient local",
    parameters(TranslateArgs),
    parsed(TranslateFilter)
)]
pub struct Translate;

impl Filter for TranslateFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        println!("==================== HERE AT evalution");
        let path = input.to_kstr().to_string();
        // let args = self.args.evaluate(runtime)?;
        println!("{:?}", self.args);

        // We load translations from the runtime (considering them as global variables).
        let translations_config = runtime.get(&[KNOCK_TRANSLATIONS_VAR_NAME.into()]);

        let translation = match translations_config {
            Ok(parsed_translation_configs) => {

                // translation configs will be converted to liquid objects when sending them as
                // params through the Rust NIF bridge
                match parsed_translation_configs.as_object() {
                    Some(translations_config_object) => resolve_translation(path, translations_config_object),
                    _ => return Ok(Value::scalar(""))
                }
            },
            Err(_) => return Ok(Value::scalar(""))
        };

        match translation {
            Ok(result) => Ok(Value::scalar(result)),
            _ => Ok(Value::scalar(""))
        }
    }
}

fn resolve_translation(path: String, translations_config: &dyn ObjectView) -> Result<String> {
    let namespace_and_path = path.split(NAMESPACE_SEPARATOR).collect::<Vec<&str>>();

    match namespace_and_path[..] {
        [namespace, path_as_string] => {
             let path_as_vector = path_as_string.split(KEY_SEPARATOR);
             let namespace_key = format!("$_{}", namespace);
             let namespace_translations = translations_config.get(&namespace_key);
             let mut resolved_translation = match namespace_translations {
                 Some(resolved_namespace_translations) => resolved_namespace_translations,
                 _ => return Ok(String::from(""))
             };

             for key in path_as_vector {
                 resolved_translation = match resolved_translation.as_object().unwrap().get(key) {
                     Some(value) => value,
                     None => return Ok(String::from("")),
                 };
             }

             match resolved_translation.as_scalar() {
                 Some(result) => Ok(result.into_cow_str().to_string()),
                 _ => Ok(String::from(""))
             }
        },
        [path_as_string] => {
            let mut path_parts = path_as_string.split('.');
            let mut resolved_translation;

            if let Some(first_part) = path_parts.next() {
                resolved_translation = match translations_config.get(first_part) {
                    Some(value) => value,
                    None => return Ok(String::from("")),
                };

                for key in path_parts {
                    resolved_translation = match resolved_translation.as_object().unwrap().get(key) {
                        Some(value) => value,
                        None => return Ok(String::from("")),
                    };
                }
            } else {
                return Ok(String::from(""))
            }

            match resolved_translation.as_scalar() {
                Some(result) => Ok(result.into_cow_str().to_string()),
                _ => Ok(String::from(""))
            }
        },
        _ => Ok(String::from(""))
    }

}

fn liquid_with_translate() -> liquid::Parser {
    liquid::ParserBuilder::with_stdlib()
        .filter(Translate)
        .build()
        .unwrap()
}

#[test]
fn test_filter_translation() {
    let assigns = o!({"first_name": "John"});
    //  "HI {{user.first_name}}"

    assert_template_result!(
        "Hello John, literal!",
        "{{ 'layout.header.hello_user' | t: name: first_name, other: 'literal' }}",
        assigns,
        liquid_with_translate()
    );
}
