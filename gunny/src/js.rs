//! JSON/JavaScript-related functionality.
// TODO: When https://github.com/boa-dev/boa/pull/1746 lands, refactor all of this code.

use boa::{JsResult, JsString, JsValue};
use eyre::Result;
use log::trace;
use pulldown_cmark::{Options, Parser};
use serde_json::Value as JsonValue;

use crate::{Error, Value};

/// Execute a JavaScript function with an array of variables.
pub fn execute_fn_with_vars(
    ctx: &mut boa::Context,
    name: &str,
    values: &[Value],
    fn_name: &str,
) -> Result<Value> {
    let script = format!(
        r#"
            let {name} = [
                {vars}
            ];
            let result = {fn_name}({name});
            JSON.stringify(result)
        "#,
        name = name,
        vars = values
            .iter()
            .map(|value| {
                Ok(format!(
                    r#"JSON.parse('{}')"#,
                    format_json_str(&serde_json::to_string(&JsonValue::from(value.clone()))?)
                ))
            })
            .collect::<Result<Vec<String>>>()?
            .join(",\n"),
        fn_name = fn_name,
    );
    trace!("Attempting to execute script:\n{}", script);
    let result = ctx
        .eval(script)
        .map_err(|e| Error::JavaScript(fn_name.to_string(), format!("{:#?}", e)))?;
    Ok(match &result {
        JsValue::String(s) => Value::from(serde_json::from_str::<JsonValue>(s)?),
        _ => {
            return Err(Error::UnexpectedJavaScriptReturnValue(
                fn_name.to_string(),
                format!("{:?}", result),
            )
            .into());
        }
    })
}

/// Register an object parsed from the given JSON under the specified name in
/// the given context.
pub fn register_json_var(ctx: &mut boa::Context, name: &str, value: &Value) -> Result<()> {
    let json_str = format_json_str(&serde_json::to_string(&JsonValue::from(value.clone()))?);
    ctx.eval(format!(
        r#"let {name} = JSON.parse('{json_str}');"#,
        name = name,
        json_str = json_str
    ))
    .map_err(|e| Error::JsonToJavaScript(format!("{:?}", e)))?;
    Ok(())
}

fn format_json_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

pub fn markdown_to_html(
    _this: &JsValue,
    args: &[JsValue],
    _ctx: &mut boa::Context,
) -> JsResult<JsValue> {
    trace!("args = {:#?}", args);
    if args.len() != 1 {
        return Err(JsValue::String(JsString::new(
            "expecting a single argument for markdownToHtml",
        )));
    }
    let content = match &args[0] {
        JsValue::String(s) => s.to_string(),
        _ => return Err(JsValue::String(JsString::new("expected a string argument"))),
    };
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_HEADING_ATTRIBUTES;
    let parser = Parser::new_ext(&content, options);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    Ok(JsValue::String(JsString::from(html)))
}

#[cfg(test)]
mod test {
    use super::*;
    use boa::Context;
    use serde_json::json;

    #[test]
    fn json_to_js_and_back() {
        let mut ctx = Context::new();
        let json_obj = json!({
            "title": "Test",
            "magicNumber": 42,
        });
        ctx.eval("function passThrough(vals) { return vals; }")
            .unwrap();
        let result =
            execute_fn_with_vars(&mut ctx, "testObj", &[json_obj.into()], "passThrough").unwrap();
        match result {
            Value::Array(arr) => {
                assert_eq!(arr.len(), 1);
                match &arr[0] {
                    Value::Object(obj) => {
                        assert_eq!(obj.get("title").unwrap().as_str().unwrap(), "Test");
                        assert_eq!(obj.get("magicNumber").unwrap().as_u64().unwrap(), 42);
                    }
                    _ => panic!("unexpected return type from function: {:?}", arr[0]),
                }
            }
            _ => panic!("unexpected return type from function: {:?}", result),
        }
    }
}
