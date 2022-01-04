use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use eyre::Result;
use handlebars::Handlebars;
use log::{debug, warn};

use crate::data::load_data;
use crate::hash::sha256;
use crate::{Error, PartialView, View};

/// Execution context for a Gunny rendering operation.
pub struct Context<'a> {
    hb: Handlebars<'a>,
    // Maps template content hashes -> names.
    template_hashes: HashMap<String, String>,
    views: HashMap<String, View>,
}

impl<'a> Default for Context<'a> {
    fn default() -> Self {
        Self {
            hb: Handlebars::new(),
            template_hashes: HashMap::new(),
            views: HashMap::new(),
        }
    }
}

impl<'a> Context<'a> {
    /// Compiles the given template and adds it to the context, returning an
    /// error if a template with the same name already exists or if there was a
    /// problem parsing the template.
    ///
    /// Assumes that the template is a string in
    /// [Handlebars](https://handlebarsjs.com/) format.
    pub fn register_template<N, T>(&mut self, name: N, template: T) -> Result<()>
    where
        N: AsRef<str>,
        T: AsRef<str>,
    {
        let name = name.as_ref();
        let template = template.as_ref();
        let template_hash = sha256(template);
        let template_name_for_hash = self
            .template_hashes
            .get(&template_hash)
            .cloned()
            .unwrap_or_else(|| "".to_string());
        if self.hb.has_template(name) {
            if template_name_for_hash == name {
                debug!(
                    "Already have template {} with hash {}, skipping",
                    name, template_hash
                );
                // We already know about this template.
                return Ok(());
            }
            // We've just gotten a template with the same name as an already
            // registered one, but its content is different.
            return Err(Error::TemplateAlreadyExists(name.to_string()).into());
        }
        debug!("Registering template {} with hash {}", name, template_hash);
        self.template_hashes.insert(template_hash, name.to_string());
        Ok(self.hb.register_template_string(name, template)?)
    }

    /// Adds the given view to the context, returning an error if a view with
    /// the same name already exists.
    pub fn register_view(&mut self, view: View) -> Result<()> {
        let name = view.name().to_string();
        if self.views.contains_key(&name) {
            return Err(Error::ViewAlreadyExists(name).into());
        }
        self.views.insert(name.clone(), view);
        debug!("Registered view {}", name);
        Ok(())
    }

    /// Load a view from the given file and register it in this context.
    ///
    /// Returns the name of the view on success.
    pub fn load_view_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Option<String>> {
        let path = path.as_ref().canonicalize()?;
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        if self.views.contains_key(&name) {
            warn!(
                "Skipping duplicate view with name \"{}\" found at {}",
                name,
                path.display()
            );
            return Ok(None);
        }
        debug!("Attempting to load view {} from: {}", name, path.display());
        let content = fs::read_to_string(path)?;
        let mut partial_view = PartialView::new(name.clone(), content)?;

        let select = partial_view.select()?;
        debug!("View {} select = {}", name, select);
        let template = partial_view.template()?;
        let template_path = PathBuf::from(&template).canonicalize()?;
        let template_id = template_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let template_content = fs::read_to_string(&template_path)?;
        self.register_template(&template_id, template_content)?;

        let output_pattern_id = format!("{}-output-pattern", name);
        let output_pattern = partial_view.output_pattern()?;
        debug!(
            "View {} output pattern {} = {}",
            name, output_pattern_id, output_pattern
        );
        self.register_template(&output_pattern_id, output_pattern)?;

        self.register_view(View::new(
            partial_view,
            select,
            template_id,
            output_pattern_id,
        ))?;

        Ok(Some(name))
    }

    /// Load views from the file system that match the given patterns.
    ///
    /// On success, returns the names of all of the views loaded.
    pub fn load_views(&mut self, patterns: &[&str]) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for pattern in patterns {
            for entry_result in glob::glob(pattern)? {
                let entry = entry_result?;
                if entry.is_file() {
                    if let Some(name) = self.load_view_from_file(&entry)? {
                        names.push(name);
                    }
                }
            }
        }
        if names.is_empty() {
            Err(Error::NoViewsFound.into())
        } else {
            Ok(names)
        }
    }

    /// Renders the view with the given name.
    pub fn render_view<N: AsRef<str>>(&mut self, name: N) -> Result<()> {
        let name = name.as_ref();
        let view = self
            .views
            .get_mut(name)
            .ok_or_else(|| Error::NoSuchView(name.to_string()))?;
        let select_glob = view.select_glob()?;
        for entry_result in select_glob {
            let entry = entry_result?;
            if entry.is_file() {
                let data = serde_json::Value::from(load_data(&entry)?);
                // Only render the data if we get data back from the processing
                // step in the script.
                if let Some(processed) = view.process(&data)? {
                    let output_path =
                        PathBuf::from(self.hb.render(view.output_pattern_id(), &processed)?)
                            .canonicalize()?;
                    let rendered = self.hb.render(view.template_id(), &processed)?;
                    fs::write(&output_path, &rendered)?;
                    debug!("View {} generated {}", name, output_path.display());
                } else {
                    debug!("{}.process() skipped entry {}", name, entry.display());
                }
            }
        }
        Ok(())
    }

    /// Render all views.
    pub fn render_all(&mut self) -> Result<()> {
        let view_names = self.views.keys().cloned().collect::<Vec<String>>();
        for view_name in view_names {
            self.render_view(view_name)?;
        }
        Ok(())
    }
}
