use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use eyre::{Result, WrapErr};
use handlebars::Handlebars;
use log::{debug, warn};
use serde_json::Value as JsonValue;

use crate::fs::maybe_canonicalize;
use crate::hash::sha256;
use crate::js::markdown_to_html;
use crate::template::{format_date, format_date_time, pad};
use crate::{Error, PartialView, Value, View};

/// Execution context for a Gunny rendering operation.
pub struct Context<'a> {
    config: Value,
    output_base_path: PathBuf,
    hb: Handlebars<'a>,
    // Maps template content hashes -> names.
    template_hashes: HashMap<String, String>,
    views: HashMap<String, View>,
}

impl<'a> Context<'a> {
    /// Constructor.
    pub fn new<P1, P2>(maybe_config_file: P1, output_base_path: P2) -> Result<Self>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        debug!(
            "Attempting to load config file: {}",
            maybe_config_file.as_ref().display()
        );
        let maybe_config_file = maybe_config_file.as_ref();
        let output_base_path = output_base_path.as_ref();
        ensure_path_exists(output_base_path)?;

        let config = match maybe_canonicalize(&maybe_config_file)? {
            Some(config_path) => {
                let config_path = config_path.canonicalize()?;
                let v = Value::load_from_file(&config_path)
                    .wrap_err_with(|| Error::FailedToLoadConfig(config_path.clone()))?;
                debug!("Loaded configuration from {}", config_path.display());
                v
            }
            None => {
                debug!(
                    "No such configuration file, skipping configuration file loading: {}",
                    maybe_config_file.display()
                );
                Value::empty_object()
            }
        };

        let mut hb = Handlebars::new();
        hb.register_helper("format_date", Box::new(format_date));
        hb.register_helper("format_date_time", Box::new(format_date_time));
        hb.register_helper("pad", Box::new(pad));
        Ok(Self {
            config,
            output_base_path: output_base_path.to_path_buf(),
            hb: Handlebars::new(),
            template_hashes: HashMap::new(),
            views: HashMap::new(),
        })
    }

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
        let path = path.as_ref();
        let path = path
            .canonicalize()
            .wrap_err_with(|| Error::FailedToLoadView(path.to_path_buf()))?;
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
        let content =
            fs::read_to_string(&path).wrap_err_with(|| Error::FailedToLoadView(path.clone()))?;
        let mut partial_view = PartialView::new(name.clone(), content)?;

        let select = partial_view.select()?;
        debug!("View {} select = {}", name, select);
        let template = partial_view.template()?;
        let template_path = PathBuf::from(&template);
        let template_path = template_path
            .canonicalize()
            .wrap_err_with(|| Error::FailedToLoadTemplate(template_path.to_path_buf()))?;
        let template_id = template_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let template_content = fs::read_to_string(&template_path)
            .wrap_err_with(|| Error::FailedToLoadTemplate(template_path.clone()))?;
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
        // SAFETY: We just registered the view in the preceding line.
        let view = self.views.get_mut(&name).unwrap();
        view.register_global_property("config", &self.config)?;
        view.register_global_function("markdownToHtml", markdown_to_html)?;

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
    pub fn render_view<N: AsRef<str>>(&mut self, name: N) -> Result<u64> {
        let mut output_count = 0_u64;
        let name = name.as_ref();
        let view = self
            .views
            .get_mut(name)
            .ok_or_else(|| Error::NoSuchView(name.to_string()))?;
        let select_glob = view.select_glob()?;
        let mut all_data = Vec::new();
        for entry_result in select_glob {
            let entry = entry_result?;
            if entry.is_file() {
                let data = Value::load_from_file(&entry)?;
                all_data.push(data);
            }
        }
        // Only render the data if we get data back from the processing
        // step in the script.
        if let Some(all_processed) = view.process(&all_data[..])? {
            let all_processed = JsonValue::from(all_processed);
            let all_processed = match all_processed {
                JsonValue::Array(arr) => arr,
                JsonValue::Object(obj) => vec![JsonValue::Object(obj)],
                _ => {
                    return Err(Error::UnexpectedJavaScriptReturnValue(
                        "process".to_string(),
                        "expected either an object or an array".to_string(),
                    )
                    .into())
                }
            };
            for processed in all_processed {
                let output_path_rendered =
                    PathBuf::from(self.hb.render(view.output_pattern_id(), &processed)?);
                let output_path = if output_path_rendered.is_relative() {
                    self.output_base_path.join(output_path_rendered)
                } else {
                    output_path_rendered
                };
                ensure_parent_path_exists(&output_path)?;
                let rendered = self.hb.render(view.template_id(), &processed)?;
                fs::write(&output_path, &rendered)?;
                debug!("View {} generated {}", name, output_path.display());
                output_count += 1;
            }
        } else {
            debug!("{}.process() produced no output", name);
        }
        Ok(output_count)
    }

    /// Render all views.
    pub fn render_all(&mut self) -> Result<u64> {
        let mut output_count = 0_u64;
        let view_names = self.views.keys().cloned().collect::<Vec<String>>();
        for view_name in view_names {
            output_count += self.render_view(view_name)?;
        }
        Ok(output_count)
    }
}

fn ensure_parent_path_exists(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::PathMissingParent(path.to_path_buf()))?;
    ensure_path_exists(parent)
}

fn ensure_path_exists(path: &Path) -> Result<()> {
    if !path.is_dir() {
        fs::create_dir_all(path)?;
        debug!("Created path: {}", path.display());
    }
    Ok(())
}
