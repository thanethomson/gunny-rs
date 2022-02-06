use std::path::Path;

use handlebars::Handlebars;

use crate::{Error, Value};

/// The template engine for a particular project.
#[derive(Debug)]
pub struct Templates<'reg> {
    hb: Handlebars<'reg>,
}

impl<'reg> From<Handlebars<'reg>> for Templates<'reg> {
    fn from(hb: Handlebars<'reg>) -> Self {
        Self { hb }
    }
}

impl<'reg> From<Templates<'reg>> for Handlebars<'reg> {
    fn from(t: Templates<'reg>) -> Self {
        t.hb
    }
}

impl<'reg> Default for Templates<'reg> {
    fn default() -> Self {
        Self {
            hb: Handlebars::new(),
        }
    }
}

impl<'reg> Templates<'reg> {
    /// Load a template from the given file. On success returns the template's
    /// name (the filename without extension).
    pub fn load<P: AsRef<Path>>(path: P) -> Result<String, Error> {
        todo!()
    }

    /// Load a partial template from the given file. On success returns the
    /// partial's name (the filename without extension).
    pub fn load_partial<P: AsRef<Path>>(path: P) -> Result<String, Error> {
        todo!()
    }

    /// Load zero or more templates that match the given glob-style pattern from
    /// the file system.
    pub fn load_all(pattern: &str) -> Result<Vec<String>, Error> {
        todo!()
    }

    /// Load zero or more partial templates that match the given glob-style
    /// pattern from the file system.
    pub fn load_all_partials(pattern: &str) -> Result<Vec<String>, Error> {
        todo!()
    }

    /// Render the given data through the template with the specified name.
    pub fn render(&self, name: &str, data: &Value) -> Result<String, Error> {
        self.hb
            .render(name, data)
            .map_err(|e| Error::TemplateRender(name.to_string(), e))
    }
}
