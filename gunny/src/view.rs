use crate::Map;

/// Provides a view of the data in a project.
#[derive(Debug)]
pub struct View {
    // A map of queries to the names of variables into which the data loaded by
    // executing the queries must be loaded.
    data: Map<String, Query>,
    // Mappings of data transformations to execute on specific variables. Each
    // item must correspond to a variable defined in `data`.
    transforms: Map<String, Vec<String>>,
    // The ID of the template for the output pattern.
    output_pattern_id: String,
    // The ID of the template to use for rendering output content from this
    // view.
    template_id: String,
}
