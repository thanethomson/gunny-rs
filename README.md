# Gunny

Gunny aims to provide a **framework** for building static content generators in
Rust.

## How it works

The following diagram gives a rough outline of how Gunny transforms raw data
into static content.

```
 +-------------+   +-------+   +------+   +----------------+
 | Data source |-->| Model |-->| View |-->| Static content |
 +-------------+   +-------+   +------+   +----------------+
                                  ^
                                  |
                              +----------+
                              | Template |
                              +----------+
```

In words:

- Views produce static content
- Each view uses a template to render structured data
- Structured data is loaded from raw data sources

## License

Copyright 2022 Thane Thomson

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

> <http://www.apache.org/licenses/LICENSE-2.0>

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

