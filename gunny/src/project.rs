use crate::{Collection, Map, Source, Templates, Transform, Value, View};

/// A project conceptually brings together all the elements necessary to process
/// and produce the desired static content, minus the database.
#[derive(Debug)]
pub struct Project<'reg> {
    // Named global variables.
    globals: Map<String, Value>,
    // Named sources of data.
    sources: Map<String, Source>,
    // Named data transforms, which can be used by collections and views.
    transforms: Map<String, Transform>,
    // Named collections of loaded, transformed data.
    collections: Map<String, Collection>,
    // Pre-loaded and parsed templates, ready to be used by views.
    templates: Templates<'reg>,
    // Named views of data collections that can be rendered through templates.
    views: Map<String, View>,
}
