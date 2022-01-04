//! Gunny view-related functionality.

use boa::JsValue;
use eyre::Result;
use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::{
    js::{execute_fn_with_var, register_json_var},
    Config, Error,
};

/// A partial view is one whose script has been loaded and evaluated, but its
/// other parameters (like select statement, template, output pattern) have not
/// yet been loaded.
#[derive(Debug)]
pub struct PartialView {
    name: String,
    script_ctx: boa::Context,
}

impl PartialView {
    /// Constructor.
    pub fn new<N, S>(name: N, script: S) -> Result<Self>
    where
        N: AsRef<str>,
        S: AsRef<[u8]>,
    {
        let name = name.as_ref().to_string();
        let mut script_ctx = boa::Context::new();
        let result = script_ctx
            .eval(script)
            .map_err(|e| Error::ViewLoad(name.clone(), format!("{:?}", e)))?;
        if result.is_null_or_undefined() {
            Ok(Self { name, script_ctx })
        } else {
            Err(Error::ViewLoad(
                name,
                format!("got unexpected result from view evaluation: {:?}", result),
            )
            .into())
        }
    }

    /// Sets the global `config` property in the view script context so that
    /// it's accessible from the script.
    pub fn configure(&mut self, config: &Config) -> Result<()> {
        register_json_var(&mut self.script_ctx, "config", config)
    }

    /// Execute the script's `select()` method to obtain the data selection
    /// glob.
    pub fn select(&mut self) -> Result<String> {
        let result = self
            .script_ctx
            .eval("select()")
            .map_err(|e| Error::ViewSelect(self.name.clone(), format!("{:?}", e)))?;
        match &result {
            JsValue::String(s) => Ok(s.to_string()),
            _ => Err(Error::ViewSelect(
                self.name.clone(),
                format!("expected a string result, but got {:?}", result),
            )
            .into()),
        }
    }

    /// Get the name of the template for this view.
    pub fn template(&mut self) -> Result<String> {
        let result = self
            .script_ctx
            .eval("template()")
            .map_err(|e| Error::ViewTemplateName(self.name.clone(), format!("{:?}", e)))?;
        match &result {
            JsValue::String(s) => Ok(s.to_string()),
            _ => Err(Error::ViewTemplateName(
                self.name.clone(),
                format!("expected a string result, but got {:?}", result),
            )
            .into()),
        }
    }

    /// Get the output pattern for this view.
    ///
    /// The string should be in [Handlebars](https://handlebarsjs.com/) format.
    pub fn output_pattern(&mut self) -> Result<String> {
        let result = self
            .script_ctx
            .eval("outputPattern()")
            .map_err(|e| Error::ViewOutputPattern(self.name.clone(), format!("{:?}", e)))?;
        match &result {
            JsValue::String(s) => Ok(s.to_string()),
            _ => Err(Error::ViewOutputPattern(
                self.name.clone(),
                format!("expected a string result, but got {:?}", result),
            )
            .into()),
        }
    }
}

/// A view is code that facilitates filtering and transformation of data prior
/// to it being rendered through a template.
pub struct View {
    name: String,
    // Each view has its own scripting context to minimize the potential for
    // cross-contamination across views. It remains to be seen whether this
    // introduces a substantial cost in terms of performance and/or resource
    // usage.
    script_ctx: boa::Context,
    // Cached select statement.
    select: String,
    // The ID of the template associated with this view in the Handlebars
    // registry.
    template_id: String,
    // The ID of the output pattern template in the Handlebars registry.
    output_pattern_id: String,
}

impl View {
    /// Constructor to build a view from a partially loaded view.
    ///
    /// Assumes that an external entity (in this case the [`crate::Context`])
    /// will be loading the remaining parameters.
    pub fn new<S1, S2, S3>(
        partial: PartialView,
        select: S1,
        template_id: S2,
        output_pattern_id: S3,
    ) -> Self
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
        S3: AsRef<str>,
    {
        Self {
            name: partial.name,
            script_ctx: partial.script_ctx,
            select: select.as_ref().to_string(),
            template_id: template_id.as_ref().to_string(),
            output_pattern_id: output_pattern_id.as_ref().to_string(),
        }
    }

    /// Obtain the name of this view (automatically extracted from the file
    /// name).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the select glob associated with this view.
    pub fn select(&self) -> &str {
        &self.select
    }

    /// Returns an iterator derived from the select string with which one can
    /// iterate through all matching file names.
    pub fn select_glob(&self) -> Result<glob::Paths> {
        Ok(glob::glob(&self.select)?)
    }

    /// Get the ID of the template associated with this view.
    pub fn template_id(&self) -> &str {
        &self.template_id
    }

    /// Get the template ID of the output pattern associated with this view.
    pub fn output_pattern_id(&self) -> &str {
        &self.output_pattern_id
    }

    /// Register a globally accessible property within the scripting context for
    /// this view under the given name.
    pub fn register_global_property<V: Serialize>(&mut self, name: &str, item: &V) -> Result<()> {
        register_json_var(&mut self.script_ctx, name, item)
    }

    /// Calls the `process` method on the given data item. If the method returns
    /// `null` or `undefined`, then this method returns `Ok(None)`.
    pub fn process<V: Serialize>(&mut self, item: &V) -> Result<Option<JsonValue>> {
        let result = execute_fn_with_var(&mut self.script_ctx, "item", item, "process")?;
        match result {
            JsonValue::Null => Ok(None),
            JsonValue::Object(_) => Ok(Some(result)),
            _ => Err(Error::UnexpectedJavaScriptReturnValue(
                "process".to_string(),
                format!("{:?}", result),
            )
            .into()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn partial_view_construction() {
        let script = r#"
            function select() {
                return "data/**/*.md";
            }

            function template() {
                return "templates/post.html";
            }
        "#;

        let mut partial_view = PartialView::new("test", script).unwrap();
        assert_eq!(partial_view.select().unwrap(), "data/**/*.md");
        assert_eq!(partial_view.template().unwrap(), "templates/post.html");
    }
}
