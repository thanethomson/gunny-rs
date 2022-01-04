//! JSON/JavaScript-related functionality.
// TODO: When https://github.com/boa-dev/boa/pull/1746 lands, refactor all of
// this code.

use boa::JsValue;
use eyre::Result;
use serde::Serialize;
use serde_json::Value;

use crate::Error;

/// Execute a JavaScript function with an arbitrary variable, automatically
/// parsing the return result back into JSON.
pub fn execute_fn_with_var<V: Serialize>(
    ctx: &mut boa::Context,
    name: &str,
    value: &V,
    fn_name: &str,
) -> Result<Value> {
    let json_str = serde_json::to_string(value)?.replace('\'', "\\'");
    let script = format!(
        r#"
        let {name} = JSON.parse('{json_str}');
        let result = {fn_name}({name});
        JSON.stringify(result)
    "#,
        name = name,
        json_str = json_str,
        fn_name = fn_name
    );
    let result = ctx
        .eval(script)
        .map_err(|e| Error::JavaScript(fn_name.to_string(), format!("{:?}", e)))?;
    Ok(match &result {
        JsValue::String(s) => serde_json::from_str(s)?,
        _ => {
            return Err(Error::UnexpectedJavaScriptReturnValue(
                fn_name.to_string(),
                format!("{:?}", result),
            )
            .into())
        }
    })
}

/// Register an object parsed from the given JSON under the specified name in
/// the given context.
pub fn register_json_var<V: Serialize>(
    ctx: &mut boa::Context,
    name: &str,
    value: &V,
) -> Result<()> {
    let json_str = serde_json::to_string(value)?.replace('\'', "\\'");
    ctx.eval(format!(
        r#"let {name} = JSON.parse('{json_str}');"#,
        name = name,
        json_str = json_str
    ))
    .map_err(|e| Error::JsonToJavaScript(format!("{:?}", e)))?;
    Ok(())
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
        ctx.eval("function passThrough(val) { return val; }")
            .unwrap();
        let result = execute_fn_with_var(&mut ctx, "testObj", &json_obj, "passThrough").unwrap();
        match result {
            Value::Object(obj) => {
                assert_eq!(obj.get("title").unwrap(), "Test");
                assert_eq!(obj.get("magicNumber").unwrap(), 42);
            }
            _ => panic!("unexpected return type from function: {:?}", result),
        }
    }
}
